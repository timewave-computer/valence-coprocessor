use std::{net::TcpStream, sync::Arc, thread, time::Duration};

use flume::{Receiver, Sender};
use msgpacker::{Packable as _, Unpackable as _};
use sp1_sdk::{
    CpuProver, CudaProver, Prover as _, ProverClient, SP1Proof, SP1ProvingKey, SP1Stdin,
};
use tokio::sync::Mutex;
use tungstenite::WebSocket;
use valence_coprocessor::{Base64, Proof};

use crate::{
    cache::KeysCache,
    types::{Circuit, ProofType, RecursiveProof, Request, Response, Task},
};

/// A worker instance.
pub struct Worker {
    cache: KeysCache,
    sp1mock: CpuProver,
    sp1cpu: CpuProver,
    sp1gpu: Option<Arc<Mutex<CudaProver>>>,
    rx: Receiver<Task>,
    tx: Sender<()>,
}

impl Worker {
    fn read(s: &mut WebSocket<TcpStream>) -> Option<Vec<u8>> {
        match s.read() {
            Ok(m) => Some(m.into_data().to_vec()),
            Err(e) => {
                Self::send_err(s, format!("failed to accept socket: {e}"));
                None
            }
        }
    }

    fn send(s: &mut WebSocket<TcpStream>, r: Response) {
        let r = r.pack_to_vec();

        if let Err(e) = s.send(r.into()) {
            tracing::warn!("failed to write to socket: {e}");
        }
    }

    fn send_err<M: AsRef<str>>(s: &mut WebSocket<TcpStream>, m: M) {
        let e = Response::Err(m.as_ref().into());
        let e = e.pack_to_vec();

        if let Err(e) = s.send(e.into()) {
            tracing::warn!("failed to write to socket: {e}");
        }
    }

    async fn sp1pk(&self, circuit: Circuit) -> Option<SP1ProvingKey> {
        match circuit {
            Circuit::Identifier(c) => {
                let pk = self.cache.get(&c).await?;

                bincode::deserialize(&pk).ok()
            }

            Circuit::Elf { identifier, bytes } => {
                let elf = Base64::decode(bytes).ok()?;
                let (pk, _vk) = self.sp1cpu.setup(&elf);

                if let Ok(b) = bincode::serialize(&pk) {
                    self.cache.set(identifier, b).await;
                }

                Some(pk)
            }
        }
    }

    async fn execute(&self, req: Request) -> Response {
        tracing::debug!("worker received {req:?}");

        match req {
            Request::Sp1Proof {
                circuit,
                witnesses,
                t,
                recursive,
            } => {
                let pk = match self.sp1pk(circuit).await {
                    Some(pk) => pk,
                    None => return Response::ProvingKeyNotCached,
                };

                let witnesses = match Base64::decode(&witnesses) {
                    Ok(w) => w,
                    Err(e) => return Response::Err(format!("error decoding the witnesses: {e}")),
                };

                let recursive = match RecursiveProof::decode(recursive) {
                    Ok(r) => r,
                    Err(e) => {
                        return Response::Err(format!("error decoding the recursive proofs: {e}"))
                    }
                };

                let mut stdin = SP1Stdin::new();

                let is_recursive = !recursive.is_empty();

                for r in recursive {
                    stdin.write_proof(r.proof, r.vk);
                }

                stdin.write_slice(&witnesses);

                tracing::debug!("environment prepared...");

                let proof = match &self.sp1gpu {
                    Some(c) => {
                        // the SP1 prover crashes in case of invalid witnesses. to avoid that, we
                        // do a dry-run

                        if !is_recursive {
                            if let Err(e) = self.sp1mock.prove(&pk, &stdin).run() {
                                return Response::Err(format!("proof dry-run failed: {e}"));
                            }
                        }

                        let p = c.lock().await;
                        let proof = p.prove(&pk, &stdin).compressed();

                        match t {
                            ProofType::Compressed => proof.run(),
                            ProofType::Groth16 => proof.groth16().run(),
                        }
                    }
                    None => match t {
                        ProofType::Compressed => self.sp1cpu.prove(&pk, &stdin).compressed().run(),
                        ProofType::Groth16 => {
                            self.sp1cpu.prove(&pk, &stdin).compressed().groth16().run()
                        }
                    },
                };

                let proof = match proof {
                    Ok(p) => p,
                    Err(e) => return Response::Err(format!("failed computing gpu proof: {e}")),
                };

                tracing::debug!("proof computed");

                if let Err(e) = self.sp1cpu.verify(&proof, &pk.vk) {
                    return Response::Err(format!("Proof sanity check failed: {e}"));
                }

                tracing::debug!("proof verified");

                let public = proof.public_values.to_vec();
                let proof = match proof.proof {
                    SP1Proof::Plonk(_) | SP1Proof::Groth16(_) => proof.bytes(),
                    _ => serde_cbor::to_vec(&proof).unwrap(),
                };
                let proof = Proof::new(proof, public).to_base64();

                tracing::debug!("proof serialized.");

                Response::Proof(proof)
            }

            Request::Sp1GetVerifyingKey { circuit } => match self.sp1pk(circuit).await {
                Some(pk) => {
                    let vk = bincode::serialize(&pk.vk).unwrap_or_default();
                    let vk = Base64::encode(vk);

                    Response::VerifyingKey(vk)
                }
                None => Response::ProvingKeyNotCached,
            },

            Request::Close => Response::Ack,
        }
    }

    pub fn spawn(
        cache: KeysCache,
        rx: Receiver<Task>,
        tx: Sender<()>,
        sp1gpu: Option<Arc<Mutex<CudaProver>>>,
    ) {
        tracing::debug!("spawning a new worker thread...");

        let sp1mock = ProverClient::builder().mock().build();
        let sp1cpu = ProverClient::builder().cpu().build();

        let worker = Self {
            cache,
            sp1mock,
            sp1cpu,
            sp1gpu,
            rx,
            tx,
        };

        tokio::spawn(async move {
            while let Ok(t) = worker.rx.recv() {
                let s = &mut match t {
                    Task::Conn(s) => s,
                    Task::Quit => break,
                };

                while let Some(b) = Self::read(s) {
                    let req = match Request::unpack(&b) {
                        Ok((_, r)) => r,

                        Err(e) => {
                            Self::send_err(s, format!("invalid message: {e}"));
                            continue;
                        }
                    };

                    if matches!(req, Request::Close) {
                        let r = Response::Ack.pack_to_vec();

                        s.send(r.into()).ok();

                        break;
                    }

                    let res = worker.execute(req).await;

                    tracing::debug!("worker computed {res:?}");

                    Self::send(s, res);
                }
            }

            tracing::debug!("worker thread halted.");

            while let Err(e) = worker.tx.send(()) {
                tracing::error!("failed to coordinate worker shutdown: {e}");

                thread::sleep(Duration::from_millis(2000));
            }
        });
    }
}

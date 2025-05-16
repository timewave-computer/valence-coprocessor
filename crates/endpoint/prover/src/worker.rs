use std::{net::TcpStream, thread, time::Duration};

use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
use flume::{Receiver, Sender};
use msgpacker::{Packable as _, Unpackable as _};
use sp1_sdk::{CpuProver, CudaProver, Prover as _, ProverClient, SP1ProvingKey, SP1Stdin};
use tungstenite::WebSocket;

use crate::{
    cache::KeysCache,
    types::{Circuit, Request, Response, Task},
};

/// A worker instance.
pub struct Worker {
    cache: KeysCache,
    sp1mock: CpuProver,
    sp1gpu: Option<CudaProver>,
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
                let elf = Base64.decode(bytes).ok()?;
                let (pk, _vk) = self.sp1mock.setup(&elf);

                if let Ok(b) = bincode::serialize(&pk) {
                    self.cache.set(identifier, b).await;
                }

                Some(pk)
            }
        }
    }

    async fn execute(&self, req: Request) -> Response {
        tracing::debug!("worker received {req:?}");

        let sp1mock = matches!(req, Request::Sp1MockProof { .. });

        match req {
            Request::Sp1MockProof { circuit, witnesses }
            | Request::Sp1GpuProof { circuit, witnesses } => {
                let pk = match self.sp1pk(circuit).await {
                    Some(pk) => pk,
                    None => return Response::ProvingKeyNotCached,
                };

                let witnesses = match Base64.decode(&witnesses) {
                    Ok(w) => w,
                    Err(e) => return Response::Err(format!("error decoding the witnesses: {e}")),
                };

                let mut stdin = SP1Stdin::new();

                stdin.write_slice(&witnesses);

                tracing::debug!("environment prepared...");

                let proof = if sp1mock {
                    let proof = match self.sp1mock.prove(&pk, &stdin).run() {
                        Ok(p) => p,
                        Err(e) => {
                            return Response::Err(format!("failed computing mock proof: {e}"))
                        }
                    };

                    match bincode::serialize(&proof) {
                        Ok(p) => p,
                        Err(e) => {
                            return Response::Err(format!("failed serializing mock proof: {e}"))
                        }
                    }
                } else if let Some(c) = &self.sp1gpu {
                    match c.prove(&pk, &stdin).groth16().run() {
                        Ok(p) => p.bytes(),
                        Err(e) => return Response::Err(format!("failed computing gpu proof: {e}")),
                    }
                } else {
                    return Response::Err("GPU support not activated.".into());
                };

                tracing::debug!("proof computed.");

                Response::Proof(Base64.encode(proof))
            }

            Request::Sp1GetVerifyingKey { circuit } => match self.sp1pk(circuit).await {
                Some(pk) => {
                    let vk = bincode::serialize(&pk.vk).unwrap_or_default();
                    let vk = Base64.encode(vk);

                    Response::VerifyingKey(vk)
                }
                None => Response::ProvingKeyNotCached,
            },

            Request::Close => Response::Ack,
        }
    }

    pub fn spawn(cache: KeysCache, rx: Receiver<Task>, tx: Sender<()>, gpu: bool) {
        tracing::debug!("spawning a new worker thread...");

        let sp1mock = ProverClient::builder().mock().build();
        let sp1gpu = gpu.then(|| {
            tracing::info!("initializing GPU support...");

            ProverClient::builder().cuda().build()
        });

        let worker = Self {
            cache,
            sp1mock,
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

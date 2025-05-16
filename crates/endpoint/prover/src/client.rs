use std::net::{TcpStream, ToSocketAddrs};

use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
use msgpacker::{Packable as _, Unpackable as _};
use serde::{Deserialize, Serialize};
use sp1_sdk::{SP1ProofWithPublicValues, SP1VerifyingKey};
use tungstenite::{stream::MaybeTlsStream, WebSocket};
use valence_coprocessor::Hash;

use crate::types::{Circuit, Request, Response};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Client {
    pub addr: String,
}

impl Client {
    pub fn new<A>(addr: A) -> anyhow::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let addr = addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow::anyhow!("failed to define socket address"))?;

        let addr = format!("ws://{addr}/");

        Ok(Self { addr })
    }

    fn connect(&self) -> anyhow::Result<WebSocket<MaybeTlsStream<TcpStream>>> {
        Ok(tungstenite::connect(&self.addr)?.0)
    }

    /// Get a SP1 mock proof.
    ///
    /// The `circuit` argument will be used to index the proving key. If the proving key cannot be
    /// found, `elf` will be evaluated to return the elf binary so the key can be computed and stored.
    pub fn get_sp1_mock_proof<F, W>(
        &self,
        circuit: Hash,
        witnesses: W,
        elf: F,
    ) -> anyhow::Result<SP1ProofWithPublicValues>
    where
        F: FnOnce(&Hash) -> anyhow::Result<Vec<u8>>,
        W: AsRef<[u8]>,
    {
        let mut socket = self.connect()?;

        socket.send(
            Request::Sp1MockProof {
                circuit: circuit.into(),
                witnesses: Base64.encode(witnesses.as_ref()),
            }
            .pack_to_vec()
            .into(),
        )?;

        let res = socket.read()?.into_data().to_vec();
        let res = Response::unpack(&res)?.1;

        match res {
            Response::Proof(p) => {
                let proof = Base64.decode(p)?;
                let proof = bincode::deserialize(&proof)?;

                return Ok(proof);
            }

            Response::ProvingKeyNotCached => (),

            Response::Err(e) => anyhow::bail!("error processing request: {e}"),

            _ => anyhow::bail!("unexpected response {res:?}"),
        }

        let elf = elf(&circuit)?;
        let elf = Base64.encode(elf);

        socket.send(
            Request::Sp1MockProof {
                circuit: Circuit::Elf {
                    identifier: circuit,
                    bytes: elf,
                },
                witnesses: Base64.encode(witnesses.as_ref()),
            }
            .pack_to_vec()
            .into(),
        )?;

        let res = socket.read()?.into_data().to_vec();
        let res = Response::unpack(&res)?.1;

        let proof = match res {
            Response::Proof(p) => {
                let proof = Base64.decode(p)?;

                bincode::deserialize(&proof)?
            }

            Response::Err(e) => anyhow::bail!("error processing request: {e}"),

            _ => anyhow::bail!("unexpected response {res:?}"),
        };

        socket.send(Request::Close.pack_to_vec().into()).ok();

        Ok(proof)
    }

    /// Get a SP1 GPU proof.
    ///
    /// The `circuit` argument will be used to index the proving key. If the proving key cannot be
    /// found, `elf` will be evaluated to return the elf binary so the key can be computed and stored.
    pub fn get_sp1_gpu_proof<F, W>(
        &self,
        circuit: Hash,
        witnesses: &W,
        elf: F,
    ) -> anyhow::Result<SP1ProofWithPublicValues>
    where
        F: FnOnce(&Hash) -> anyhow::Result<Vec<u8>>,
        W: AsRef<[u8]>,
    {
        let mut socket = self.connect()?;

        socket.send(
            Request::Sp1GpuProof {
                circuit: circuit.into(),
                witnesses: Base64.encode(witnesses.as_ref()),
            }
            .pack_to_vec()
            .into(),
        )?;

        let res = socket.read()?.into_data().to_vec();
        let res = Response::unpack(&res)?.1;

        match res {
            Response::Proof(p) => {
                let proof = Base64.decode(p)?;
                let proof = bincode::deserialize(&proof)?;

                return Ok(proof);
            }

            Response::ProvingKeyNotCached => (),

            Response::Err(e) => anyhow::bail!("error processing request: {e}"),

            _ => anyhow::bail!("unexpected response {res:?}"),
        }

        let elf = elf(&circuit)?;
        let elf = Base64.encode(elf);

        socket.send(
            Request::Sp1GpuProof {
                circuit: Circuit::Elf {
                    identifier: circuit,
                    bytes: elf,
                },
                witnesses: Base64.encode(witnesses.as_ref()),
            }
            .pack_to_vec()
            .into(),
        )?;

        let res = socket.read()?.into_data().to_vec();
        let res = Response::unpack(&res)?.1;

        let proof = match res {
            Response::Proof(p) => {
                let proof = Base64.decode(p)?;

                bincode::deserialize(&proof)?
            }

            Response::Err(e) => anyhow::bail!("error processing request: {e}"),

            _ => anyhow::bail!("unexpected response {res:?}"),
        };

        socket.send(Request::Close.pack_to_vec().into()).ok();

        Ok(proof)
    }

    /// Get the verifying key for the given circuit.
    ///
    /// The `circuit` argument will be used to index the proving key. If the proving key cannot be
    /// found, `elf` will be evaluated to return the elf binary so the key can be computed and stored.
    pub fn get_sp1_verifying_key<F>(&self, circuit: Hash, elf: F) -> anyhow::Result<SP1VerifyingKey>
    where
        F: FnOnce(&Hash) -> anyhow::Result<Vec<u8>>,
    {
        let mut socket = self.connect()?;

        socket.send(
            Request::Sp1GetVerifyingKey {
                circuit: circuit.into(),
            }
            .pack_to_vec()
            .into(),
        )?;

        let res = socket.read()?.into_data().to_vec();
        let res = Response::unpack(&res)?.1;

        match res {
            Response::VerifyingKey(vk) => {
                let vk = Base64.decode(vk)?;

                return Ok(bincode::deserialize(&vk)?);
            }

            Response::ProvingKeyNotCached => (),

            Response::Err(e) => anyhow::bail!("error processing request: {e}"),
            _ => anyhow::bail!("unexpected response {res:?}"),
        }

        let elf = elf(&circuit)?;
        let elf = Base64.encode(elf);

        socket.send(
            Request::Sp1GetVerifyingKey {
                circuit: Circuit::Elf {
                    identifier: circuit,
                    bytes: elf,
                },
            }
            .pack_to_vec()
            .into(),
        )?;

        let res = socket.read()?.into_data().to_vec();
        let res = Response::unpack(&res)?.1;

        let vk = match res {
            Response::VerifyingKey(vk) => {
                let vk = Base64.decode(vk)?;

                bincode::deserialize(&vk)?
            }

            Response::Err(e) => anyhow::bail!("error processing request: {e}"),

            _ => anyhow::bail!("unexpected response {res:?}"),
        };

        socket.send(Request::Close.pack_to_vec().into()).ok();

        Ok(vk)
    }
}

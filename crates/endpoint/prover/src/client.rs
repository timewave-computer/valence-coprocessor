use std::{env, net::TcpStream};

use msgpacker::{Packable as _, Unpackable as _};
use serde::{Deserialize, Serialize};
use sp1_sdk::SP1VerifyingKey;
use tungstenite::{stream::MaybeTlsStream, WebSocket};
use valence_coprocessor::{Base64, Blake3Hasher, Hash, Hasher as _, Proof};

use crate::types::{Circuit, Request, Response};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Client {
    pub addr: String,
}

impl Client {
    pub fn new<A>(addr: A) -> Self
    where
        A: ToString,
    {
        Self {
            addr: addr.to_string(),
        }
    }

    fn connect(&self) -> anyhow::Result<WebSocket<MaybeTlsStream<TcpStream>>> {
        let mut stream = tungstenite::connect(&self.addr)?.0;

        let secret = env::var("VALENCE_PROVER_SECRET").unwrap_or_default();
        let challenge = stream.read()?.into_data().to_vec();
        let challenge = Blake3Hasher::hash(&[secret.as_bytes(), challenge.as_slice()].concat());

        stream.send(challenge.to_vec().into())?;

        Ok(stream)
    }

    /// Get a base64 encoded SP1 bytes proof.
    ///
    /// The `circuit` argument will be used to index the proving key. If the proving key cannot be
    /// found, `elf` will be evaluated to return the elf binary so the key can be computed and stored.
    pub fn get_sp1_proof<F, W>(&self, circuit: Hash, witnesses: &W, elf: F) -> anyhow::Result<Proof>
    where
        F: FnOnce(&Hash) -> anyhow::Result<Vec<u8>>,
        W: AsRef<[u8]>,
    {
        tracing::debug!(
            "sending SP1 prove with GPU request to remove prover on {}...",
            &self.addr
        );

        let mut socket = self.connect()?;

        tracing::debug!("prover socket connected on {}...", &self.addr);

        socket.send(
            Request::Sp1Proof {
                circuit: circuit.into(),
                witnesses: Base64::encode(witnesses.as_ref()),
            }
            .pack_to_vec()
            .into(),
        )?;

        let res = socket.read()?.into_data().to_vec();

        tracing::debug!("response received...");

        let res = Response::unpack(&res)?.1;

        match res {
            Response::Proof(p) => {
                tracing::debug!("proving key cached; returning proof");

                let proof = Base64::decode(p)?;
                let proof = Proof::unpack(&proof)?.1;

                return Ok(proof);
            }

            Response::ProvingKeyNotCached => (),

            Response::Err(e) => anyhow::bail!("error processing request: {e}"),

            _ => anyhow::bail!("unexpected response {res:?}"),
        }

        tracing::debug!("proving key cache miss...");

        let elf = elf(&circuit)?;
        let elf = Base64::encode(elf);

        socket.send(
            Request::Sp1Proof {
                circuit: Circuit::Elf {
                    identifier: circuit,
                    bytes: elf,
                },
                witnesses: Base64::encode(witnesses.as_ref()),
            }
            .pack_to_vec()
            .into(),
        )?;

        let res = socket.read()?.into_data().to_vec();

        tracing::debug!("response received...");

        let res = Response::unpack(&res)?.1;

        let proof = match res {
            Response::Proof(p) => p,
            Response::Err(e) => anyhow::bail!("error processing request: {e}"),
            _ => anyhow::bail!("unexpected response {res:?}"),
        };
        let proof = Base64::decode(proof)?;
        let proof = Proof::unpack(&proof)?.1;

        socket.send(Request::Close.pack_to_vec().into()).ok();

        tracing::debug!("proof returned...");

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
                let vk = Base64::decode(vk)?;

                return Ok(bincode::deserialize(&vk)?);
            }

            Response::ProvingKeyNotCached => (),

            Response::Err(e) => anyhow::bail!("error processing request: {e}"),
            _ => anyhow::bail!("unexpected response {res:?}"),
        }

        let elf = elf(&circuit)?;
        let elf = Base64::encode(elf);

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
                let vk = Base64::decode(vk)?;

                bincode::deserialize(&vk)?
            }

            Response::Err(e) => anyhow::bail!("error processing request: {e}"),

            _ => anyhow::bail!("unexpected response {res:?}"),
        };

        socket.send(Request::Close.pack_to_vec().into()).ok();

        Ok(vk)
    }
}

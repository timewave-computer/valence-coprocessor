use std::net::ToSocketAddrs;

use valence_coprocessor::{DataBackend, ExecutionContext, Hash, Proof, WitnessCoprocessor, ZkVm};
use valence_coprocessor_sp1::Sp1Hasher;

use crate::client::Client;

#[derive(Clone)]
pub struct ProverService {
    client: Client,
}

impl ProverService {
    pub fn new<A>(addr: A) -> anyhow::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let client = Client::new(addr)?;

        Ok(Self { client })
    }
}

impl ZkVm for ProverService {
    type Hasher = Sp1Hasher;

    fn prove<D>(
        &self,
        ctx: &ExecutionContext<Self::Hasher, D>,
        w: WitnessCoprocessor,
    ) -> anyhow::Result<Proof>
    where
        D: DataBackend,
    {
        tracing::debug!("initiating prove request...");

        let circuit = *ctx.library();
        let w = bincode::serialize(&w)?;

        tracing::debug!(
            "witnesses serialized for circuit {}...",
            hex::encode(circuit)
        );

        let proof = self.client.get_sp1_proof(circuit, &w, |_| {
            ctx.get_zkvm()
                .transpose()
                .ok_or_else(|| anyhow::anyhow!("failed to fetch ELF contents from context"))?
        })?;

        tracing::debug!("proof fetched from service...");

        Ok(proof)
    }

    fn verifying_key<D>(&self, ctx: &ExecutionContext<Self::Hasher, D>) -> anyhow::Result<Vec<u8>>
    where
        D: DataBackend,
    {
        let circuit = *ctx.library();
        let vk = self.client.get_sp1_verifying_key(circuit, |_| {
            ctx.get_zkvm()
                .transpose()
                .ok_or_else(|| anyhow::anyhow!("failed to fetch ELF contents from context"))?
        })?;

        Ok(bincode::serialize(&vk)?)
    }

    fn updated(&self, _program: &Hash) {}
}

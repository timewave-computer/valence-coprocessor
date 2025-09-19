use std::sync::Arc;

use hashbrown::HashMap;
use msgpacker::MsgPacker;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use valence_coprocessor::{
    DataBackend, ExecutionContext, Hash, Hasher, Proof, WitnessCoprocessor, ZkVm,
};
use valence_coprocessor_sp1::Sp1Hasher;

use crate::{client::Client, types::ProofType};

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, MsgPacker)]
pub struct Cluster {
    clients: Vec<Client>,
    next: usize,
}

impl Cluster {
    pub fn push<C>(&mut self, client: C)
    where
        C: ToString,
    {
        let client = Client::new(client);

        self.clients.push(client);
    }

    pub fn pop(&mut self) -> Option<Client> {
        if !self.clients.is_empty() {
            Some(self.clients.remove(0))
        } else {
            None
        }
    }

    pub fn remove(&mut self, address: &str) -> Option<Client> {
        let idx = self
            .clients
            .iter()
            .enumerate()
            .find_map(|(i, c)| (c.address() == address).then_some(i))?;

        Some(self.clients.remove(idx))
    }

    pub fn rotate(&mut self) -> Option<Client> {
        if self.next >= self.clients.len() {
            self.next = 0;
        }

        let client = self.clients.get(self.next).cloned();

        self.next += 1;

        match &client {
            Some(c) => tracing::debug!("rotating prover to `{}`...", c.address()),
            None => tracing::debug!("no prover available"),
        }

        client
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, MsgPacker)]
pub struct AllocatedProvers {
    pub public: Vec<String>,
    pub owned: Vec<String>,
}

#[derive(Default, Debug, Clone)]
pub struct ProverScheduler {
    public: Arc<Mutex<Cluster>>,
    owned: Arc<Mutex<HashMap<Vec<u8>, Cluster>>>,
}

impl ProverScheduler {
    fn get_client<H, D>(&self, ctx: &ExecutionContext<H, D>) -> anyhow::Result<Client>
    where
        H: Hasher,
        D: DataBackend,
    {
        if let Some(owner) = ctx.owner() {
            if let Some(c) = self.owned.lock().get_mut(owner).and_then(Cluster::rotate) {
                tracing::debug!("returning owned prover...");

                return Ok(c);
            }
        }

        tracing::debug!("no owned prover available; falling back to public cluster...");

        self.public
            .lock()
            .rotate()
            .ok_or_else(|| anyhow::anyhow!("no available public client"))
    }

    pub fn allocated(&self, owner: Option<&[u8]>) -> AllocatedProvers {
        let public = self
            .public
            .lock()
            .clients
            .iter()
            .map(|c| c.address().into())
            .collect();

        let owned = owner
            .and_then(|o| {
                self.owned
                    .lock()
                    .get(o)
                    .map(|c| c.clients.iter().map(|c| c.address().into()).collect())
            })
            .unwrap_or_default();

        AllocatedProvers { public, owned }
    }

    pub fn push(&self, owner: Option<&[u8]>, addr: &str) {
        match owner {
            Some(o) => self.owned.lock().entry(o.to_vec()).or_default().push(addr),

            None => self.public.lock().push(addr),
        }
    }

    pub fn remove(&self, owner: Option<&[u8]>, addr: &str) -> Option<String> {
        match owner {
            Some(o) => self
                .owned
                .lock()
                .get_mut(o)
                .and_then(|c| c.remove(addr))
                .map(|c| c.address().into()),

            None => self.public.lock().remove(addr).map(|c| c.address().into()),
        }
    }
}

impl ZkVm for ProverScheduler {
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

        let circuit = *ctx.controller();
        let w = bincode::serialize(&w)?;

        tracing::debug!(
            "witnesses serialized for circuit {}...",
            hex::encode(circuit)
        );

        let t = ProofType::Groth16;
        let client = self.get_client(ctx)?;
        let recursive = Vec::new();

        let proof = client.get_sp1_proof(circuit, t, &w, &recursive, |_| {
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
        let circuit = *ctx.controller();
        let client = self.get_client(ctx)?;
        let vk = client.get_sp1_verifying_key(circuit, |_| {
            ctx.get_zkvm()
                .transpose()
                .ok_or_else(|| anyhow::anyhow!("failed to fetch ELF contents from context"))?
        })?;

        Ok(bincode::serialize(&vk)?)
    }

    fn updated(&self, _circuit: &Hash) {}
}

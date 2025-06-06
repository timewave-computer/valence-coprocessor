use core::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use lru::LruCache;
use serde::{de::DeserializeOwned, Serialize};
use sp1_sdk::{
    CpuProver, CudaProver, NetworkProver, Prover as _, ProverClient, SP1Proof,
    SP1ProofWithPublicValues, SP1ProvingKey, SP1PublicValues, SP1Stdin, SP1VerifyingKey,
};
use valence_coprocessor::{
    Base64, DataBackend, ExecutionContext, Hash, Proof, WitnessCoprocessor, ZkVm,
};

use crate::Sp1Hasher;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
    Mock,
    Cpu,
    Gpu,
    Network,
}

enum WrappedClient {
    Mock(CpuProver),
    Cpu(CpuProver),
    Gpu(CudaProver),
    Network(NetworkProver),
}

impl From<&WrappedClient> for Mode {
    fn from(client: &WrappedClient) -> Self {
        match client {
            WrappedClient::Mock(_) => Mode::Mock,
            WrappedClient::Cpu(_) => Mode::Cpu,
            WrappedClient::Gpu(_) => Mode::Gpu,
            WrappedClient::Network(_) => Mode::Network,
        }
    }
}

impl TryFrom<&str> for Mode {
    type Error = anyhow::Error;

    fn try_from(mode: &str) -> anyhow::Result<Mode> {
        match mode {
            "mock" => Ok(Self::Mock),
            "cpu" => Ok(Self::Mock),
            "gpu" => Ok(Self::Mock),
            "network" => Ok(Self::Mock),
            _ => anyhow::bail!("invalid SP1 zkVM mode: `{mode}`"),
        }
    }
}

impl From<Mode> for WrappedClient {
    fn from(mode: Mode) -> Self {
        let client = ProverClient::builder();

        match mode {
            Mode::Mock => Self::Mock(client.mock().build()),
            Mode::Cpu => Self::Cpu(client.cpu().build()),
            Mode::Gpu => Self::Gpu(client.cuda().build()),
            Mode::Network => Self::Network(client.network().build()),
        }
    }
}

impl WrappedClient {
    fn prove(&self, pk: &SP1ProvingKey, w: WitnessCoprocessor) -> anyhow::Result<Proof> {
        tracing::debug!("prove routine initiated...");

        let mut stdin = SP1Stdin::new();

        stdin.write(&w);

        tracing::debug!("witnesses written to SP1 environment...");

        let proof = match self {
            WrappedClient::Mock(p) => p.prove(pk, &stdin).run()?,
            WrappedClient::Cpu(p) => p.prove(pk, &stdin).groth16().run()?,
            WrappedClient::Gpu(p) => p.prove(pk, &stdin).groth16().run()?,
            WrappedClient::Network(p) => p.prove(pk, &stdin).groth16().run()?,
        };

        tracing::debug!("proof executed...");

        let bytes = match &proof.proof {
            SP1Proof::Core(_) | SP1Proof::Compressed(_) => bincode::serialize(&proof)?,
            SP1Proof::Plonk(_) | SP1Proof::Groth16(_) => proof.bytes(),
        };

        tracing::debug!("proof generated!");

        Ok(Proof {
            proof: Base64::encode(bytes),
            inputs: Base64::encode(proof.public_values.to_vec()),
        })
    }

    fn setup(&self, elf: &[u8]) -> (SP1ProvingKey, SP1VerifyingKey) {
        match self {
            WrappedClient::Mock(p) => p.setup(elf),
            WrappedClient::Cpu(p) => p.setup(elf),
            WrappedClient::Gpu(p) => p.setup(elf),
            WrappedClient::Network(p) => p.setup(elf),
        }
    }

    fn verify(&self, vk: &SP1VerifyingKey, proof: &SP1ProofWithPublicValues) -> bool {
        match self {
            WrappedClient::Mock(p) => p.verify(proof, vk).is_ok(),
            WrappedClient::Cpu(p) => p.verify(proof, vk).is_ok(),
            WrappedClient::Gpu(p) => p.verify(proof, vk).is_ok(),
            WrappedClient::Network(p) => p.verify(proof, vk).is_ok(),
        }
    }
}

#[derive(Clone)]
pub struct Sp1ZkVm {
    client: Arc<WrappedClient>,
    keys: Arc<Mutex<LruCache<Hash, (SP1ProvingKey, SP1VerifyingKey)>>>,
}

impl Sp1ZkVm {
    pub fn new(mode: Mode, capacity: usize) -> anyhow::Result<Self> {
        let client = WrappedClient::from(mode);
        let client = Arc::new(client);

        let capacity =
            NonZeroUsize::new(capacity).ok_or_else(|| anyhow::anyhow!("invalid capacity"))?;
        let keys = LruCache::new(capacity);
        let keys = Arc::new(Mutex::new(keys));

        Ok(Self { client, keys })
    }

    pub fn mock() -> Self {
        let client = WrappedClient::from(Mode::Mock);
        let client = Arc::new(client);

        let capacity = NonZeroUsize::new(10).unwrap();
        let keys = LruCache::new(capacity);
        let keys = Arc::new(Mutex::new(keys));

        Self { client, keys }
    }

    pub fn verify(&self, vk: &SP1VerifyingKey, proof: &SP1ProofWithPublicValues) -> bool {
        self.client.verify(vk, proof)
    }

    pub fn outputs<T>(&self, proof: &Proof) -> anyhow::Result<T>
    where
        T: Serialize + DeserializeOwned,
    {
        let mut inputs = SP1PublicValues::new();

        let values = Base64::decode(&proof.inputs)?;

        inputs.write_slice(&values);

        Ok(inputs.read())
    }
}

impl ZkVm for Sp1ZkVm {
    type Hasher = Sp1Hasher;

    fn prove<D>(
        &self,
        ctx: &ExecutionContext<Sp1Hasher, D>,
        w: WitnessCoprocessor,
    ) -> anyhow::Result<Proof>
    where
        D: DataBackend,
    {
        let controller = ctx.controller();

        tracing::debug!("SP1 proving initialized...");

        self.keys
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking keys: {e}"))?
            .try_get_or_insert(*controller, || {
                tracing::debug!("fetching keys from context...");

                let elf = ctx
                    .get_zkvm()?
                    .ok_or_else(|| anyhow::anyhow!("failed to fetch zkvm from registry"))?;

                Ok(self.client.setup(&elf))
            })
            .and_then(|(pk, _vk)| {
                tracing::debug!("proving circuit...");

                self.client.prove(pk, w)
            })
    }

    fn verifying_key<D>(&self, ctx: &ExecutionContext<Sp1Hasher, D>) -> anyhow::Result<Vec<u8>>
    where
        D: DataBackend,
    {
        let controller = ctx.controller();

        self.keys
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking keys: {e}"))?
            .try_get_or_insert::<_, anyhow::Error>(*controller, || {
                tracing::debug!("fetching keys from context...");

                let elf = ctx
                    .get_zkvm()?
                    .ok_or_else(|| anyhow::anyhow!("failed to fetch zkvm from registry"))?;

                Ok(self.client.setup(&elf))
            })
            .and_then(|(_pk, vk)| Ok(bincode::serialize(&vk)?))
    }

    fn updated(&self, controller: &Hash) {
        match self.keys.lock() {
            Ok(mut k) => {
                k.pop(controller);
            }
            Err(e) => tracing::error!("error locking keys: {e}"),
        }
    }
}

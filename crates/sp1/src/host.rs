use std::sync::{Arc, Mutex};

use lru::LruCache;
use sp1_sdk::{
    CpuProver, CudaProver, NetworkProver, Prover as _, ProverClient, SP1Proof, SP1ProvingKey,
    SP1Stdin, SP1VerifyingKey,
};
use valence_coprocessor::{
    DataBackend, ExecutionContext, Hash, Hasher, ModuleVM, ProvenProgram, Witness, ZkVM,
};

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
    fn prove(&self, pk: &SP1ProvingKey, witnesses: Vec<Witness>) -> anyhow::Result<ProvenProgram> {
        tracing::debug!("prove routine initiated...");

        let mut stdin = SP1Stdin::new();

        stdin.write(&witnesses);

        tracing::debug!("witnesses written to SP1 environment...");

        // TODO evaluate if should output groth16 at this point

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

        Ok(ProvenProgram {
            proof: bytes,
            outputs: proof.public_values.to_vec(),
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
}

#[derive(Clone)]
pub struct Sp1ZkVM {
    client: Arc<WrappedClient>,
    keys: Arc<Mutex<LruCache<Hash, (SP1ProvingKey, SP1VerifyingKey)>>>,
}

impl Sp1ZkVM {
    pub fn new(mode: Mode, capacity: usize) -> anyhow::Result<Self> {
        let client = WrappedClient::from(mode);
        let client = Arc::new(client);

        let capacity = std::num::NonZeroUsize::new(capacity)
            .ok_or_else(|| anyhow::anyhow!("invalid capacity"))?;
        let keys = LruCache::new(capacity);
        let keys = Arc::new(Mutex::new(keys));

        Ok(Self { client, keys })
    }
}

impl ZkVM for Sp1ZkVM {
    fn prove<H, D, M>(
        &self,
        ctx: &ExecutionContext<H, D, M, Self>,
        witnesses: Vec<Witness>,
    ) -> anyhow::Result<ProvenProgram>
    where
        H: Hasher,
        D: DataBackend,
        M: ModuleVM<H, D, Sp1ZkVM>,
    {
        let program = ctx.program();

        let mut stdin = SP1Stdin::new();

        tracing::debug!("SP1 environment initialized...");

        stdin.write(&witnesses);

        tracing::debug!("witnesses written to environment...");

        self.keys
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking keys: {e}"))?
            .try_get_or_insert(*program, || {
                tracing::debug!("fetching keys from context...");

                let elf = ctx
                    .get_zkvm()?
                    .ok_or_else(|| anyhow::anyhow!("failed to fetch zkvm from registry"))?;

                Ok(self.client.setup(&elf))
            })
            .and_then(|(pk, _vk)| {
                tracing::debug!("proving program...");

                self.client.prove(pk, witnesses)
            })
    }

    fn verifying_key<H, D, M>(
        &self,
        ctx: &ExecutionContext<H, D, M, Self>,
    ) -> anyhow::Result<Vec<u8>>
    where
        H: Hasher,
        D: DataBackend,
        M: ModuleVM<H, D, Self>,
    {
        let program = ctx.program();

        self.keys
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking keys: {e}"))?
            .try_get_or_insert::<_, anyhow::Error>(*program, || {
                tracing::debug!("fetching keys from context...");

                let elf = ctx
                    .get_zkvm()?
                    .ok_or_else(|| anyhow::anyhow!("failed to fetch zkvm from registry"))?;

                Ok(self.client.setup(&elf))
            })
            .and_then(|(_pk, vk)| Ok(bincode::serialize(&vk)?))
    }

    fn updated(&self, program: &Hash) {
        match self.keys.lock() {
            Ok(mut k) => {
                k.pop(program);
            }
            Err(e) => tracing::error!("error locking keys: {e}"),
        }
    }
}

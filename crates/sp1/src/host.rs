use std::sync::{Arc, Mutex};

use lru::LruCache;
use sp1_sdk::{
    CpuProver, CudaProver, NetworkProver, Prover as _, ProverClient, SP1Proof, SP1ProvingKey,
    SP1Stdin,
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

impl Clone for WrappedClient {
    fn clone(&self) -> Self {
        let mode = Mode::from(self);

        Self::from(mode)
    }
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
        let mut stdin = SP1Stdin::new();

        stdin.write(&witnesses);

        // TODO evaluate if should output groth16 at this point

        let proof = match self {
            WrappedClient::Mock(p) => p.prove(pk, &stdin).run()?,
            WrappedClient::Cpu(p) => p.prove(pk, &stdin).groth16().run()?,
            WrappedClient::Gpu(p) => p.prove(pk, &stdin).groth16().run()?,
            WrappedClient::Network(p) => p.prove(pk, &stdin).groth16().run()?,
        };

        let bytes = match &proof.proof {
            SP1Proof::Core(_) | SP1Proof::Compressed(_) => bincode::serialize(&proof)?,
            SP1Proof::Plonk(_) | SP1Proof::Groth16(_) => proof.bytes(),
        };

        Ok(ProvenProgram {
            proof: bytes,
            outputs: proof.public_values.to_vec(),
        })
    }

    fn setup(&self, elf: &[u8]) -> Vec<u8> {
        let (pk, _vk) = match self {
            WrappedClient::Mock(p) => p.setup(elf),
            WrappedClient::Cpu(p) => p.setup(elf),
            WrappedClient::Gpu(p) => p.setup(elf),
            WrappedClient::Network(p) => p.setup(elf),
        };

        bincode::serialize(&pk).unwrap()
    }
}

#[derive(Clone)]
pub struct Sp1ZkVM {
    client: WrappedClient,
    keys: Arc<Mutex<LruCache<Hash, SP1ProvingKey>>>,
}

impl Sp1ZkVM {
    pub fn new(mode: Mode, capacity: usize) -> anyhow::Result<Self> {
        let client = WrappedClient::from(mode);

        let capacity = std::num::NonZeroUsize::new(capacity)
            .ok_or_else(|| anyhow::anyhow!("invalid capacity"))?;
        let keys = LruCache::new(capacity);
        let keys = Arc::new(Mutex::new(keys));

        Ok(Self { client, keys })
    }

    pub fn to_zkvm(&self, elf: &[u8]) -> Vec<u8> {
        self.client.setup(elf)
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

        stdin.write(&witnesses);

        self.keys
            .lock()
            .map_err(|e| anyhow::anyhow!("error locking keys: {e}"))?
            .try_get_or_insert(*program, || {
                let zkvm = ctx
                    .get_zkvm()?
                    .ok_or_else(|| anyhow::anyhow!("failed to fetch zkvm from registry"))?;

                Ok(bincode::deserialize(&zkvm)?)
            })
            .and_then(|pk| self.client.prove(pk, witnesses))
    }
}

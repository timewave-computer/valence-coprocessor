use valence_coprocessor::{DataBackend, ExecutionContext, Hash, Proof, WitnessCoprocessor, ZkVm};
use valence_coprocessor_prover::prover::ProverService;
use valence_coprocessor_sp1::{Mode, Sp1Hasher, Sp1ZkVm};

#[derive(Clone)]
pub enum ServiceZkVm {
    Mock(Sp1ZkVm),
    Service(ProverService),
}

impl ServiceZkVm {
    pub fn mock(capacity: usize) -> anyhow::Result<Self> {
        let zkvm = Sp1ZkVm::new(Mode::Mock, capacity)?;

        Ok(Self::Mock(zkvm))
    }

    pub fn service<A>(addr: A) -> Self
    where
        A: ToString,
    {
        let zkvm = ProverService::new(addr);

        Self::Service(zkvm)
    }
}

impl ZkVm for ServiceZkVm {
    type Hasher = Sp1Hasher;

    fn prove<D>(
        &self,
        ctx: &ExecutionContext<Self::Hasher, D>,
        w: WitnessCoprocessor,
    ) -> anyhow::Result<Proof>
    where
        D: DataBackend,
    {
        match self {
            ServiceZkVm::Mock(z) => z.prove(ctx, w),
            ServiceZkVm::Service(z) => z.prove(ctx, w),
        }
    }

    fn verifying_key<D>(&self, ctx: &ExecutionContext<Self::Hasher, D>) -> anyhow::Result<Vec<u8>>
    where
        D: DataBackend,
    {
        match self {
            ServiceZkVm::Mock(z) => z.verifying_key(ctx),
            ServiceZkVm::Service(z) => z.verifying_key(ctx),
        }
    }

    fn updated(&self, controller: &Hash) {
        match self {
            ServiceZkVm::Mock(z) => z.updated(controller),
            ServiceZkVm::Service(z) => z.updated(controller),
        }
    }
}

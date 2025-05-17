//! Mocks for the interfaces of the system.

use core::marker::PhantomData;

use alloc::vec::Vec;
use msgpacker::Packable as _;
use serde_json::Value;

use crate::{
    Blake3Hasher, DataBackend, ExecutionContext, Hash, Hasher, ProvenProgram, Vm, Witness,
    WitnessCoprocessor, ZkVm,
};

/// A mock implementation of a zkVM
#[derive(Debug, Default, Clone, Copy)]
pub struct MockZkVm<H: Hasher = Blake3Hasher> {
    h: PhantomData<H>,
}

impl<H: Hasher> MockZkVm<H> {
    /// Verify a proof.
    pub fn verify<D>(
        _ctx: &ExecutionContext<H, D>,
        library: &Hash,
        mut witnesses: Vec<Witness>,
        proven: ProvenProgram,
    ) -> bool
    where
        H: Hasher,
        D: DataBackend,
    {
        witnesses.sort();

        let mut bytes = library.to_vec();

        for w in witnesses {
            match w {
                Witness::StateProof(p) => bytes.extend(p.pack_to_vec()),
                Witness::Data(d) => bytes.extend(d),
            }
        }

        let proof = H::hash(&bytes).to_vec();

        proven.proof == proof
    }
}

impl<H: Hasher> ZkVm for MockZkVm<H> {
    type Hasher = H;

    fn prove<D>(
        &self,
        ctx: &ExecutionContext<Self::Hasher, D>,
        w: WitnessCoprocessor,
    ) -> anyhow::Result<ProvenProgram>
    where
        D: DataBackend,
    {
        let mut witnesses = w.validate::<H>()?.witnesses;

        witnesses.sort();

        let mut bytes = ctx.library().to_vec();

        for w in witnesses {
            match w {
                Witness::StateProof(p) => bytes.extend(p.pack_to_vec()),
                Witness::Data(d) => bytes.extend(d),
            }
        }

        let proof = H::hash(&bytes).to_vec();

        Ok(ProvenProgram { proof })
    }

    fn verifying_key<D>(&self, _ctx: &ExecutionContext<Self::Hasher, D>) -> anyhow::Result<Vec<u8>>
    where
        D: DataBackend,
    {
        Ok(vec![])
    }

    fn updated(&self, _program: &Hash) {}
}

/// A mock implementation for a VM.
#[derive(Debug, Clone, Copy)]
pub struct MockVm;

impl<H, D> Vm<H, D> for MockVm
where
    H: Hasher,
    D: DataBackend,
{
    fn execute(
        &self,
        _ctx: &ExecutionContext<H, D>,
        _lib: &Hash,
        _f: &str,
        args: Value,
    ) -> anyhow::Result<Value> {
        Ok(args)
    }

    fn updated(&self, _lib: &Hash) {}
}

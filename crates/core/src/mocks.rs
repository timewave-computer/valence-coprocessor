//! Mocks for the interfaces of the system.

use alloc::vec::Vec;
use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
use serde_json::{json, Value};

use crate::{
    DataBackend, ExecutionContext, Hash, Hasher, ProvenProgram, SmtOpening, Vm, Witness, ZkVm,
};

/// A mock implementation of a zkVM
#[derive(Debug, Default)]
pub struct MockZkVm;

impl MockZkVm {
    /// Verify a proof.
    pub fn verify<H, D, M>(
        _ctx: &ExecutionContext<H, D, M, Self>,
        library: &Hash,
        mut witnesses: Vec<Witness>,
        proven: ProvenProgram,
    ) -> bool
    where
        H: Hasher,
        D: DataBackend,
        M: Vm<H, D, Self>,
    {
        witnesses.sort();

        let mut bytes = library.to_vec();

        for w in witnesses {
            match w {
                Witness::DomainProof(SmtOpening {
                    key,
                    data,
                    root,
                    opening,
                }) => {
                    bytes.extend(key);
                    bytes.extend(data);
                    bytes.extend(root);

                    for p in opening.path {
                        bytes.extend(p);
                    }
                }

                Witness::StateProof(items) | Witness::Data(items) => bytes.extend(items),
            }
        }

        let proof = H::hash(&bytes).to_vec();
        let outputs = bytes;

        proven.proof == proof && proven.outputs == outputs
    }
}

impl ZkVm for MockZkVm {
    fn prove<H, D, M>(
        &self,
        ctx: &ExecutionContext<H, D, M, Self>,
        mut witnesses: Vec<Witness>,
    ) -> anyhow::Result<ProvenProgram>
    where
        H: Hasher,
        D: DataBackend,
        M: Vm<H, D, MockZkVm>,
    {
        witnesses.sort();

        let mut bytes = ctx.library().to_vec();

        for w in witnesses {
            match w {
                Witness::DomainProof(SmtOpening {
                    key,
                    data,
                    root,
                    opening,
                }) => {
                    bytes.extend(key);
                    bytes.extend(data);
                    bytes.extend(root);

                    for p in opening.path {
                        bytes.extend(p);
                    }
                }

                Witness::StateProof(items) | Witness::Data(items) => bytes.extend(items),
            }
        }

        let proof = H::hash(&bytes).to_vec();
        let outputs = bytes;

        Ok(ProvenProgram { proof, outputs })
    }

    fn verifying_key<H, D, M>(
        &self,
        ctx: &ExecutionContext<H, D, M, Self>,
    ) -> anyhow::Result<Vec<u8>>
    where
        H: Hasher,
        D: DataBackend,
        M: Vm<H, D, Self>,
    {
        Ok(ctx.library().to_vec())
    }

    fn updated(&self, _library: &Hash) {}
}

/// A mock implementation for a VM.
pub struct MockVm;

impl MockVm {
    /// Validates the execution of a library
    pub fn validate<H, D, Z>(
        _ctx: &ExecutionContext<H, D, Self, Z>,
        lib: &Hash,
        f: &str,
        args: Value,
        execution: Value,
    ) -> bool
    where
        H: Hasher,
        D: DataBackend,
        Z: ZkVm,
    {
        json!({
            "lib": Base64.encode(lib),
            "f": f,
            "args": args
        }) == execution
    }
}

impl<H, D, Z> Vm<H, D, Z> for MockVm
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVm,
{
    fn execute(
        &self,
        _ctx: &ExecutionContext<H, D, Self, Z>,
        lib: &Hash,
        f: &str,
        args: Value,
    ) -> anyhow::Result<Value> {
        Ok(json!({
            "lib": Base64.encode(lib),
            "f": f,
            "args": args
        }))
    }

    fn updated(&self, _lib: &Hash) {}
}

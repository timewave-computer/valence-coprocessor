//! Mocks for the interfaces of the system.

use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
use serde_json::{json, Value};

use crate::{
    DataBackend, ExecutionContext, Hash, Hasher, ModuleVM, ProvenProgram, SmtOpening, Witness, ZkVM,
};

/// A mock implementation of a zkVM
#[derive(Debug, Default)]
pub struct MockZkVM;

impl MockZkVM {
    /// Verify a proof.
    pub fn verify<H, D, M>(
        _ctx: &ExecutionContext<H, D, M, Self>,
        program: &Hash,
        mut witnesses: Vec<Witness>,
        proven: ProvenProgram,
    ) -> bool
    where
        H: Hasher,
        D: DataBackend,
        M: ModuleVM<H, D, Self>,
    {
        witnesses.sort();

        let mut bytes = program.to_vec();

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

impl ZkVM for MockZkVM {
    fn prove<H, D, M>(
        &self,
        ctx: &ExecutionContext<H, D, M, Self>,
        mut witnesses: Vec<Witness>,
    ) -> anyhow::Result<ProvenProgram>
    where
        H: Hasher,
        D: DataBackend,
        M: ModuleVM<H, D, MockZkVM>,
    {
        witnesses.sort();

        let mut bytes = ctx.program().to_vec();

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
        M: ModuleVM<H, D, Self>,
    {
        Ok(ctx.program().to_vec())
    }
}

/// A mock implementation for a module VM.
pub struct MockModuleVM;

impl MockModuleVM {
    /// Validates the execution of a module
    pub fn validate<H, D, Z>(
        _ctx: &ExecutionContext<H, D, Self, Z>,
        module: &Hash,
        f: &str,
        args: Value,
        execution: Value,
    ) -> bool
    where
        H: Hasher,
        D: DataBackend,
        Z: ZkVM,
    {
        json!({
            "module": Base64.encode(module),
            "f": f,
            "args": args
        }) == execution
    }
}

impl<H, D, Z> ModuleVM<H, D, Z> for MockModuleVM
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    fn execute(
        &self,
        _ctx: &ExecutionContext<H, D, Self, Z>,
        module: &Hash,
        f: &str,
        args: Value,
    ) -> anyhow::Result<Value> {
        Ok(json!({
            "module": Base64.encode(module),
            "f": f,
            "args": args
        }))
    }
}

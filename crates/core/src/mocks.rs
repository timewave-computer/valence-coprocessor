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
        M: ModuleVM,
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
        M: ModuleVM,
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

impl ModuleVM for MockModuleVM {
    fn execute<H, D, Z>(
        &self,
        _ctx: &ExecutionContext<H, D, Self, Z>,
        module: &Hash,
        f: &str,
        args: Value,
    ) -> anyhow::Result<Value>
    where
        H: Hasher,
        D: DataBackend,
        Z: ZkVM,
    {
        Ok(json!({
            "module": Base64.encode(module),
            "f": f,
            "args": args
        }))
    }
}

impl<H, D, M, Z> ExecutionContext<H, D, M, Z>
where
    H: Hasher,
    D: DataBackend,
    M: ModuleVM,
    Z: ZkVM,
{
    /// Executes an arbitrary program function.
    pub fn execute_module(&self, program: &Hash, f: &str, args: Value) -> anyhow::Result<Value> {
        self.module.execute(self, program, f, args)
    }

    /// Computes an arbitrary program proof.
    pub fn execute_proof(&self, witnesses: Vec<Witness>) -> anyhow::Result<ProvenProgram> {
        self.zkvm.prove(self, witnesses)
    }
}

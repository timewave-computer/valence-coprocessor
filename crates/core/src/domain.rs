use alloc::{string::String, vec::Vec};
use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
use serde_json::Value;

use crate::{DataBackend, DomainData, Hash, Hasher, Registry, Vm, Witness, ZkVm};

/// A domain definition for circuit verification.
pub trait DomainCircuit {
    /// The output of the verified circuit proof.
    type Output;

    /// Verifies a state proof.
    fn verify(proof: &Witness) -> Option<Self::Output>;
}

/// A domain definition.
pub trait DomainLibrary {
    /// A constant identifier.
    const ID: &str;

    /// Computes the serialized state proof from the provided arguments.
    fn state_proof_bytes(&self, args: Value) -> anyhow::Result<Vec<u8>>;

    /// Computes the base64 serialized state proof.
    fn state_proof(&self, args: Value) -> anyhow::Result<Value> {
        let bytes = self.state_proof_bytes(args)?;
        let proof = Base64.encode(bytes);

        Ok(Value::String(proof))
    }

    /// Deploy a compiled library to the registry.
    fn deploy<H, D, M, Z>(registry: &Registry<D>, vm: &M, lib: Vec<u8>) -> anyhow::Result<Hash>
    where
        H: Hasher,
        D: DataBackend,
        M: Vm<H, D, Z>,
        Z: ZkVm,
    {
        let domain = DomainData {
            name: String::from(Self::ID),
            lib,
        };

        registry.register_domain::<M, H, Z>(vm, domain)
    }
}

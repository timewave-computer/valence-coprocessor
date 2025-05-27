use serde_json::Value;

use crate::StateProof;

/// A domain definition for circuit verification.
pub trait DomainCircuit {
    /// The output of a state proof.
    type Output;

    /// Verifies a domain-specific state proof.
    fn verify(proof: &StateProof) -> anyhow::Result<Self::Output>;
}

/// A domain definition.
pub trait DomainController {
    /// A constant identifier.
    const ID: &str;

    /// Computes a state proof from the given arguments.
    fn state_proof(&self, args: Value) -> anyhow::Result<StateProof>;

    /// Computes a state proof from the given arguments.
    fn state_proof_value(&self, args: Value) -> anyhow::Result<Value> {
        let proof = self.state_proof(args)?;

        Ok(serde_json::to_value(proof)?)
    }
}

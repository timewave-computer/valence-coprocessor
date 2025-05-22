#![no_main]
sp1_zkvm::entrypoint!(main);

use valence_coprocessor::{DomainCircuit as _, WitnessCoprocessor};
use valence_coprocessor_integrated_tests_domain::Domain;
use valence_coprocessor_sp1::Sp1Hasher;

pub fn main() {
    let w = sp1_zkvm::io::read::<WitnessCoprocessor>();
    let w = w.validate::<Sp1Hasher>().unwrap();

    let state = Domain::verify(w.witnesses[0].as_state_proof().unwrap()).unwrap();
    let value = TryFrom::try_from(w.witnesses[1].as_data().unwrap()).unwrap();
    let value = u64::from_le_bytes(value);

    let out = state.checked_add(value).unwrap();

    sp1_zkvm::io::commit(&out);
}

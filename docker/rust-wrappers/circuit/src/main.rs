#![no_main]
sp1_zkvm::entrypoint!(main);

use valence_coprocessor::WitnessCoprocessor;
use valence_coprocessor_sp1::Sp1Hasher;

pub fn main() {
    let w = sp1_zkvm::io::read::<WitnessCoprocessor>();
    let w = w.validate::<Sp1Hasher>().unwrap();

    let r = w.root;
    let b = !LIB!::circuit(w.witnesses);
    let b = [&r[..], b.as_slice()].concat();

    sp1_zkvm::io::commit_slice(&b);
}

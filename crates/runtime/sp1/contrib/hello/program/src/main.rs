#![no_main]
sp1_zkvm::entrypoint!(main);

use valence_coprocessor::{Witness, WitnessCoprocessor};
use valence_coprocessor_sp1::Sp1Hasher;

pub fn main() {
    let w = sp1_zkvm::io::read::<WitnessCoprocessor>();
    let w = w.validate::<Sp1Hasher>().unwrap();

    let w = match &w.witnesses[0] {
        Witness::Data(d) => String::from_utf8(d.to_vec()).unwrap(),
        _ => panic!("unexpected data"),
    };
    let w = format!("Hello, {w}!");

    sp1_zkvm::io::commit(&w);
}

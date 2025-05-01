#![no_main]
sp1_zkvm::entrypoint!(main);

use valence_coprocessor::{DomainCircuit, Witness};
use valence_coprocessor_ethereum::Ethereum;

pub fn main() {
    let w = sp1_zkvm::io::read::<Vec<Witness>>();

    let ret = Ethereum::verify(&w[0]).unwrap();

    sp1_zkvm::io::commit(&ret);
}

#![no_main]
sp1_zkvm::entrypoint!(main);

use valence_coprocessor::Witness;
use valence_coprocessor_domain::{ethereum::Ethereum, DomainCircuit};

pub fn main() {
    let w = sp1_zkvm::io::read::<Vec<Witness>>();

    let ret = Ethereum::verify(&w[0]).unwrap();

    sp1_zkvm::io::commit(&ret);
}

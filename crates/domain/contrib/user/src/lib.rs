#![no_std]

use alloc::vec;
use valence_coprocessor::Witness;
use valence_coprocessor_domain::{ethereum::Ethereum, DomainLibrary as _};
use valence_coprocessor_wasm::abi;

extern crate alloc;

#[no_mangle]
pub extern "C" fn get_witnesses() {
    let args = abi::args().unwrap();

    let domain = Ethereum::ID;
    let proof = abi::get_state_proof(domain, &args).unwrap();
    let ret = vec![Witness::StateProof(proof.to_vec())];

    abi::ret_witnesses(ret).unwrap();
}

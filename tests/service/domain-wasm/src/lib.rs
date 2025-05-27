#![no_std]

extern crate alloc;

use valence_coprocessor::DomainController as _;
use valence_coprocessor_integrated_tests_domain::Domain;
use valence_coprocessor_wasm::abi;

pub const ID: &str = "domain";

#[no_mangle]
pub extern "C" fn validate_block() {
    let args = abi::args().unwrap();

    abi::ret(&args).unwrap();
}

#[no_mangle]
pub extern "C" fn get_state_proof() {
    let args = abi::args().unwrap();
    let proof = Domain.state_proof_value(args).unwrap();

    abi::ret(&proof).unwrap();
}

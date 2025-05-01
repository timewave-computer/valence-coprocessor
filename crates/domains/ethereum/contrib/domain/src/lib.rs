#![no_std]

use valence_coprocessor::DomainLibrary as _;
use valence_coprocessor_ethereum::Ethereum;
use valence_coprocessor_wasm::abi;

#[no_mangle]
pub extern "C" fn get_state_proof() {
    let args = abi::args().unwrap();

    let proof = Ethereum::default().state_proof(args).unwrap();

    abi::ret(&proof).unwrap();
}

#[no_mangle]
pub extern "C" fn validate_block() {
    let args = abi::args().unwrap();

    abi::ret(&args).unwrap();
}

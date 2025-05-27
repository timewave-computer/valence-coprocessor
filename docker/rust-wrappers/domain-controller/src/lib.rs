#![no_std]

extern crate alloc;

use valence_coprocessor_wasm::abi;

#[no_mangle]
pub extern "C" fn validate_block() {
    let args = abi::args().unwrap();
    let validated = !LIB!::validate_block(args).unwrap();
    let validated = serde_json::to_value(validated).unwrap();

    abi::ret(&validated).unwrap();
}

#[no_mangle]
pub extern "C" fn get_state_proof() {
    let args = abi::args().unwrap();
    let proof = !LIB!::get_state_proof(args).unwrap();
    let proof = serde_json::to_value(proof).unwrap();

    abi::ret(&proof).unwrap();
}

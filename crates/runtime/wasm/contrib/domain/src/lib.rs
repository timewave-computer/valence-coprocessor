#![no_std]

use valence_coprocessor_wasm::abi;

extern crate alloc;

#[no_mangle]
pub extern "C" fn validate_block() {
    let args = abi::args().unwrap();

    abi::ret(&args).unwrap();
}

#[no_mangle]
pub extern "C" fn get_block() {
    let args = abi::args().unwrap();
    let domain = args.as_str().unwrap();

    let block = abi::get_latest_block(domain).unwrap().unwrap();
    let ret = serde_json::to_value(block).unwrap();

    abi::ret(&ret).unwrap();
}

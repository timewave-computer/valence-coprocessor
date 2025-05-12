#![no_std]

use valence_coprocessor_wasm::abi;

extern crate alloc;

#[no_mangle]
pub extern "C" fn entrypoint() {
    let args = abi::args().unwrap();

    let domain = args["domain"].as_str().unwrap();
    let block = abi::get_latest_block(domain).unwrap();
    let block = serde_json::to_value(&block).unwrap();

    abi::ret(&block).unwrap();
}

#[no_mangle]
pub extern "C" fn validate_block() {
    let args = abi::args().unwrap();

    abi::ret(&args).unwrap();
}

#![no_std]

use valence_coprocessor_wasm::{abi, log};

extern crate alloc;

#[no_mangle]
pub extern "C" fn log() {
    let args = abi::args().unwrap();

    let name = args["name"].as_str().unwrap();

    log!("Hello, {name}!").unwrap();
    log!("Multiple entries").unwrap();
}

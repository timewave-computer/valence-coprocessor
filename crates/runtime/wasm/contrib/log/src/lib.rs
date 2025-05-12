#![no_std]

use valence_coprocessor_wasm::abi;

extern crate alloc;

#[no_mangle]
pub extern "C" fn entrypoint() {
    let args = abi::args().unwrap();

    let name = args["name"].as_str().unwrap();

    abi::log!("Hello, {name}!").unwrap();
    abi::log!("Multiple entries").unwrap();
}

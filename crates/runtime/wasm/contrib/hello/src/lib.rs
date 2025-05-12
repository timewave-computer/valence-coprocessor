#![no_std]

use valence_coprocessor_wasm::abi;

extern crate alloc;

#[no_mangle]
pub extern "C" fn entrypoint() {
    let args = abi::args().unwrap();

    let name = args["name"].as_str().unwrap();
    let message = alloc::format!("Hello, {name}!");
    let ret = serde_json::json!({"message": message});

    abi::ret(&ret).unwrap();
}

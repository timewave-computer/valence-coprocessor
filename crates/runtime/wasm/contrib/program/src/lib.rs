#![no_std]

use valence_coprocessor_wasm::abi;

extern crate alloc;

#[no_mangle]
pub extern "C" fn entrypoint() {
    let library = abi::get_library().unwrap();

    let ret = serde_json::json!(library);

    abi::ret(&ret).unwrap();
}

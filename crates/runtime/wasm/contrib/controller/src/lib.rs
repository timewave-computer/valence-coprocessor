#![no_std]

use valence_coprocessor_wasm::abi;

extern crate alloc;

#[no_mangle]
pub extern "C" fn entrypoint() {
    let controller = abi::get_controller().unwrap();

    let ret = serde_json::json!(controller);

    abi::ret(&ret).unwrap();
}

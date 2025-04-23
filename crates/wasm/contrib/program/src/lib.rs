#![no_std]

use valence_coprocessor_wasm::abi;

extern crate alloc;

#[no_mangle]
pub extern "C" fn program() {
    let program = abi::get_program().unwrap();

    let ret = serde_json::json!(program);

    abi::ret(&ret).unwrap();
}

#![no_std]

use valence_coprocessor_wasm::abi;

extern crate alloc;

#[no_mangle]
pub extern "C" fn http() {
    let args = abi::args().unwrap();

    let ret = abi::http(&args).unwrap();

    abi::ret(&ret).unwrap();
}

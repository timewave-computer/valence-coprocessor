#![no_std]

use valence_coprocessor_wasm::abi;

extern crate alloc;

#[no_mangle]
pub extern "C" fn storage() {
    let args = abi::args().unwrap();

    let mut storage = abi::get_program_storage().unwrap();

    storage.extend(args["name"].as_str().unwrap().as_bytes());

    abi::set_program_storage(&storage).unwrap();
}

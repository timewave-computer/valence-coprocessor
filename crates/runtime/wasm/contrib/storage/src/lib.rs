#![no_std]

use alloc::vec;
use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
use valence_coprocessor_wasm::abi;

extern crate alloc;

#[no_mangle]
pub extern "C" fn set() {
    let args = abi::args().unwrap();

    let path = args["path"].as_str().unwrap();
    let contents = args["contents"].as_str().unwrap().as_bytes();

    abi::set_storage_file(path, contents).unwrap();
}

#[no_mangle]
pub extern "C" fn get() {
    let args = abi::args().unwrap();

    let path = args["path"].as_str().unwrap();
    let bytes = abi::get_storage_file(path).unwrap();
    let b64 = Base64.encode(bytes);
    let ret = serde_json::json!({"b64": b64});

    abi::ret(&ret).unwrap();
}

#[no_mangle]
pub extern "C" fn set_large() {
    let args = abi::args().unwrap();

    let path = args["path"].as_str().unwrap();
    let byte = args["byte"].as_u64().unwrap() as u8;
    let count = args["count"].as_u64().unwrap() as usize;

    let contents = vec![byte; count];

    abi::set_storage_file(path, &contents).unwrap();
}

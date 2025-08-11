#![no_std]

use alloc::vec;
use valence_coprocessor::Base64;
use valence_coprocessor_wasm::abi;

extern crate alloc;

#[no_mangle]
pub extern "C" fn entrypoint() {
    let args = abi::args().unwrap();
    let cmd = args["cmd"].as_str().unwrap();

    match cmd {
        "set" => {
            let path = args["path"].as_str().unwrap();
            let contents = args["contents"].as_str().unwrap().as_bytes();

            abi::set_storage_file(path, contents).unwrap();
        }
        "get" => {
            let path = args["path"].as_str().unwrap();
            let bytes = abi::get_storage_file(path).unwrap().unwrap();
            let b64 = Base64::encode(bytes);
            let ret = serde_json::json!({"b64": b64});

            abi::ret(&ret).unwrap();
        }
        "exists" => {
            let path = args["path"].as_str().unwrap();
            let exists = abi::get_storage_file(path).unwrap().is_some();
            let ret = serde_json::json!({"exists": exists});

            abi::ret(&ret).unwrap();
        }
        "set_large" => {
            let path = args["path"].as_str().unwrap();
            let byte = args["byte"].as_u64().unwrap() as u8;
            let count = args["count"].as_u64().unwrap() as usize;

            let contents = vec![byte; count];

            abi::set_storage_file(path, &contents).unwrap();
        }
        _ => panic!("unknown command"),
    }
}

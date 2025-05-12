#![no_std]

use alloc::{format, string::String};
use serde_json::json;
use valence_coprocessor_wasm::abi;

extern crate alloc;

#[no_mangle]
pub extern "C" fn entrypoint() {
    let args = abi::args().unwrap();

    let name = args["name"].as_str().unwrap();
    let url = args["url"].as_str().unwrap();

    let ret = abi::http(&json!({
        "url": url,
        "method": "get",
        "headers": {
            "Accept": "text/plain"
        },
        "query": {
            "name": name
        }
    }))
    .unwrap();

    let body = serde_json::from_value(ret["body"].clone()).unwrap();
    let body = String::from_utf8(body).unwrap();

    assert_eq!(format!("Hello, {name}!"), body);

    abi::ret(&ret).unwrap();
}

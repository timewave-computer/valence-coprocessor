#![no_std]

use valence_coprocessor_wasm::{abi, core::ValidatedDomainBlock};

extern crate alloc;

#[no_mangle]
pub extern "C" fn entrypoint() {
    let args = abi::args().unwrap();
    let cmd = args["cmd"].as_str().unwrap();
    let domain = args["domain"].as_str().unwrap();

    let ret = match cmd {
        "latest" => {
            let block = abi::get_latest_block(domain).unwrap();
            let block = serde_json::to_value(&block).unwrap();

            block
        }

        "opening" => {
            let historical = abi::get_historical().unwrap();
            let ValidatedDomainBlock { root, key, .. } =
                abi::get_latest_block(domain).unwrap().unwrap();
            let payload = abi::get_historical_payload(domain, &root).unwrap().unwrap();
            let opening = abi::get_historical_opening(&historical, domain, &root)
                .unwrap()
                .unwrap();

            serde_json::json!({
                "historical": historical,
                "root": root,
                "key": key,
                "payload": payload,
                "opening": opening,
            })
        }

        _ => panic!("unknown command"),
    };

    abi::ret(&ret).unwrap();
}

#[no_mangle]
pub extern "C" fn validate_block() {
    let args = abi::args().unwrap();

    abi::ret(&args).unwrap();
}

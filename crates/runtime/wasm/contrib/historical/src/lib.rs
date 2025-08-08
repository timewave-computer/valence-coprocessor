#![no_std]

use valence_coprocessor_wasm::abi;

extern crate alloc;

#[no_mangle]
pub extern "C" fn entrypoint() {
    let args = abi::args().unwrap();

    let domain = args["domain"].as_str().unwrap();
    let number = args["number"].as_u64().unwrap();

    let root = abi::get_historical().unwrap();
    let proof = abi::get_block_proof(domain, number).unwrap();
    let update = abi::get_historical_update(&root).unwrap().unwrap();

    abi::ret(&serde_json::json!({
        "root": root,
        "proof": proof,
        "update": update,
    }))
    .unwrap();
}

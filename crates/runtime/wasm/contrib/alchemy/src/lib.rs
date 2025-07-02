#![no_std]

use valence_coprocessor_wasm::abi;

extern crate alloc;

#[no_mangle]
pub extern "C" fn entrypoint() {
    let args = abi::args().unwrap();

    let chain = args["chain"].as_str().unwrap();
    let method = args["method"].as_str().unwrap();
    let params = &args["params"];

    let ret = abi::alchemy(chain, method, params).unwrap();

    abi::ret(&ret).unwrap();
}

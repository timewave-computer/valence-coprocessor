#![no_std]

extern crate alloc;

use alloc::{string::ToString, vec};
use valence_coprocessor::Witness;
use valence_coprocessor_integrated_tests_domain::ID;
use valence_coprocessor_wasm::abi;

#[no_mangle]
pub extern "C" fn get_witnesses() {
    let args = abi::args().unwrap();

    let value = args["value"].as_u64().unwrap();
    let value = value.to_le_bytes().to_vec();

    let state = args["state"].clone();
    let proof = abi::get_state_proof(ID, &state).unwrap();

    let witnesses = vec![Witness::StateProof(proof), Witness::Data(value)];

    abi::ret_witnesses(witnesses).unwrap();
}

#[no_mangle]
pub extern "C" fn entrypoint() {
    let args = abi::args().unwrap();
    let cmd = args["payload"]["cmd"].as_str().unwrap();

    match cmd {
        "store" => {
            let path = args["payload"]["path"].as_str().unwrap().to_string();
            let bytes = serde_json::to_vec(&args).unwrap();

            abi::set_storage_file(&path, &bytes).unwrap();
        }

        _ => panic!("unknown entrypoint command"),
    }

    abi::ret(&args).unwrap();
}

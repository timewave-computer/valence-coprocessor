use std::{env, fs, path::PathBuf, process::Command};

use serde_json::json;
use valence_coprocessor::{mocks::MockZkVM, Blake3Context, MemoryBackend, ProgramData, Registry};
use valence_coprocessor_wasm::host::ValenceWasm;

fn get_hello_bytes() -> Vec<u8> {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = PathBuf::from(dir).join("contrib").join("hello");

    Command::new("cargo")
        .current_dir(&dir)
        .args(["build", "--target", "wasm32-unknown-unknown", "--release"])
        .output()
        .unwrap();

    let dir = dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join("valence_coprocessor_wasm_hello.wasm");

    fs::read(dir).unwrap()
}

#[test]
fn deploy_hello() {
    let hello = get_hello_bytes();
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let program = ProgramData::default().with_module(hello);
    let program = registry.register_program(program).unwrap();

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let ctx = Blake3Context::init(program, data, vm, MockZkVM);

    let ret = ctx
        .execute_module(&program, "hello", json!({"name": "Valence"}))
        .unwrap()["message"]
        .as_str()
        .unwrap()
        .to_string();

    assert_eq!(ret, "Hello, Valence!");
}

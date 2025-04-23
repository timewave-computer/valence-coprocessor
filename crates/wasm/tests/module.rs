use std::{env, fs, path::PathBuf, process::Command};

use serde_json::json;
use valence_coprocessor::{mocks::MockZkVM, Blake3Context, MemoryBackend, ProgramData, Registry};
use valence_coprocessor_wasm::host::ValenceWasm;

fn get_program_bytes(name: &str) -> Vec<u8> {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = PathBuf::from(dir).join("contrib").join(name);

    Command::new("cargo")
        .current_dir(&dir)
        .args(["build", "--target", "wasm32-unknown-unknown", "--release"])
        .output()
        .unwrap();

    let dir = dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("valence_coprocessor_wasm_{name}.wasm"));

    fs::read(dir).unwrap()
}

#[test]
fn deploy_hello() {
    let hello = get_program_bytes("hello");
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

#[test]
fn deploy_storage() {
    let storage = get_program_bytes("storage");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let program = ProgramData::default().with_module(storage);
    let program = registry.register_program(program).unwrap();

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let ctx = Blake3Context::init(program, data, vm, MockZkVM);

    assert!(ctx.get_program_storage().unwrap().is_none());

    ctx.execute_module(&program, "storage", json!({"name": "Valence"}))
        .unwrap();

    let storage = ctx.get_program_storage().unwrap().unwrap();

    assert_eq!(storage, b"Valence");
}

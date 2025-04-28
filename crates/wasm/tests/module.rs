use std::{env, fs, path::PathBuf, process::Command, thread};

use serde_json::json;
use valence_coprocessor::{mocks::MockZkVM, Blake3Context, MemoryBackend, ProgramData, Registry};
use valence_coprocessor_wasm::host::ValenceWasm;

fn get_program_bytes(name: &str) -> Vec<u8> {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = PathBuf::from(dir).join("contrib").join(name);

    assert!(Command::new("cargo")
        .current_dir(&dir)
        .args(["build", "--target", "wasm32-unknown-unknown", "--release"])
        .status()
        .unwrap()
        .success());

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

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVM;

    let program = ProgramData::default().with_lib(hello);
    let program = registry.register_program(&vm, &zkvm, program).unwrap();

    let ctx = Blake3Context::init(program, data, vm, MockZkVM);

    let ret = ctx
        .execute_lib(&program, "hello", json!({"name": "Valence"}))
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

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVM;

    let program = ProgramData::default().with_lib(storage);
    let program = registry.register_program(&vm, &zkvm, program).unwrap();

    let ctx = Blake3Context::init(program, data, vm, MockZkVM);

    assert!(ctx.get_program_storage().unwrap().is_none());

    ctx.execute_lib(&program, "storage", json!({"name": "Valence"}))
        .unwrap();

    let storage = ctx.get_program_storage().unwrap().unwrap();

    assert_eq!(storage, b"Valence");
}

#[test]
fn deploy_program() {
    let program = get_program_bytes("program");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVM;

    let program = ProgramData::default().with_lib(program);
    let program = registry.register_program(&vm, &zkvm, program).unwrap();

    let ctx = Blake3Context::init(program, data, vm, MockZkVM);

    let ret: Vec<_> = ctx
        .execute_lib(&program, "program", json!({}))
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect();

    assert_eq!(&program, ret.as_slice());
}

#[test]
fn deploy_http() {
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port();

    thread::spawn(move || {
        for r in server.incoming_requests() {
            let name = r.url().split_once('=').unwrap().1;

            let res = format!("Hello, {name}!");
            let res = tiny_http::Response::from_string(res);

            r.respond(res).unwrap();
        }
    });

    let program = get_program_bytes("http");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVM;

    let program = ProgramData::default().with_lib(program);
    let program = registry.register_program(&vm, &zkvm, program).unwrap();

    let ctx = Blake3Context::init(program, data, vm, MockZkVM);

    let ret = ctx
        .execute_lib(
            &program,
            "http",
            json!({
                "url": format!("http://127.0.0.1:{port}"),
                "name": "Valence"
            }),
        )
        .unwrap();

    let body = serde_json::from_value(ret["body"].clone()).unwrap();
    let body = String::from_utf8(body).unwrap();

    assert_eq!("Hello, Valence!", body.as_str());
}

#[test]
fn deploy_log() {
    let hello = get_program_bytes("log");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVM;

    let program = ProgramData::default().with_lib(hello);
    let program = registry.register_program(&vm, &zkvm, program).unwrap();

    let ctx = Blake3Context::init(program, data, vm, MockZkVM);

    ctx.execute_lib(&program, "log", json!({"name": "Valence"}))
        .unwrap();

    let mut log = ctx.get_log().unwrap();

    assert_eq!("Hello, Valence!", log.remove(0));
    assert_eq!("Multiple entries", log.remove(0));
}

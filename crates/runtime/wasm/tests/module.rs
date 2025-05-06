use std::{array, env, fs, path::PathBuf, process::Command, thread};

use serde_json::{json, Value};
use valence_coprocessor::{
    mocks::MockZkVm, Blake3Context, DomainData, MemoryBackend, ProgramData, Registry,
    ValidatedBlock,
};
use valence_coprocessor_wasm::host::ValenceWasm;

fn get_library_bytes(name: &str) -> Vec<u8> {
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
    let hello = get_library_bytes("hello");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVm;

    let library = ProgramData::default().with_lib(hello);
    let library = registry.register_program(&vm, &zkvm, library).unwrap();

    let ctx = Blake3Context::init(library, data, vm, MockZkVm);

    let ret = ctx
        .execute_lib(&library, "hello", json!({"name": "Valence"}))
        .unwrap()["message"]
        .as_str()
        .unwrap()
        .to_string();

    assert_eq!(ret, "Hello, Valence!");
}

#[test]
fn deploy_storage() {
    let storage = get_library_bytes("storage");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVm;

    let library = ProgramData::default().with_lib(storage);
    let library = registry.register_program(&vm, &zkvm, library).unwrap();

    let ctx = Blake3Context::init(library, data, vm, MockZkVm);

    assert!(ctx.get_raw_storage().unwrap().is_none());

    ctx.execute_lib(&library, "storage", json!({"name": "Valence"}))
        .unwrap();

    let storage = ctx.get_raw_storage().unwrap().unwrap();

    assert_eq!(storage, b"Valence");
}

#[test]
fn deploy_program() {
    let library = get_library_bytes("program");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVm;

    let library = ProgramData::default().with_lib(library);
    let library = registry.register_program(&vm, &zkvm, library).unwrap();

    let ctx = Blake3Context::init(library, data, vm, MockZkVm);

    let ret: Vec<_> = ctx
        .execute_lib(&library, "program", json!({}))
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect();

    assert_eq!(&library, ret.as_slice());
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

    let library = get_library_bytes("http");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVm;

    let library = ProgramData::default().with_lib(library);
    let library = registry.register_program(&vm, &zkvm, library).unwrap();

    let ctx = Blake3Context::init(library, data, vm, MockZkVm);

    let ret = ctx
        .execute_lib(
            &library,
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
    let hello = get_library_bytes("log");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVm;

    let library = ProgramData::default().with_lib(hello);
    let library = registry.register_program(&vm, &zkvm, library).unwrap();

    let ctx = Blake3Context::init(library, data, vm, MockZkVm);

    ctx.execute_lib(&library, "log", json!({"name": "Valence"}))
        .unwrap();

    let mut log = ctx.get_log().unwrap();

    assert_eq!("Hello, Valence!", log.remove(0));
    assert_eq!("Multiple entries", log.remove(0));
}

#[test]
fn deploy_domain() {
    let library = get_library_bytes("domain");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();

    let name = "valence";
    let domain = DomainData::new(name.into()).with_lib(library);
    let library = registry.register_domain(&vm, domain).unwrap();

    let block = ValidatedBlock {
        number: 238972,
        root: array::from_fn(|i| i as u8),
        payload: name.as_bytes().to_vec(),
    };
    let block_json = serde_json::to_value(&block).unwrap();

    let ctx = Blake3Context::init(library, data, vm, MockZkVm);

    ctx.add_domain_block(name, block_json).unwrap();

    let latest = ctx.get_latest_block(name).unwrap().unwrap();

    assert_eq!(block, latest);

    let ret = ctx
        .execute_lib(&library, "get_block", Value::String(name.into()))
        .unwrap();
    let ret: ValidatedBlock = serde_json::from_value(ret).unwrap();

    assert_eq!(ret, block);
}

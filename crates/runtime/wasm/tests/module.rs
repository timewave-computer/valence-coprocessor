use std::{env, fs, path::PathBuf, process::Command, thread};

use serde_json::json;
use valence_coprocessor::{
    mocks::MockZkVm, Base64, Blake3Hasher, Blake3Historical, CompoundOpening, ControllerData,
    DomainData, Hash, Hasher as _, HistoricalUpdate, MemoryBackend, Registry, ValidatedDomainBlock,
};
use valence_coprocessor_wasm::host::ValenceWasm;

fn get_controller_bytes(name: &str) -> Vec<u8> {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = PathBuf::from(dir).join("contrib");

    assert!(Command::new("cargo")
        .current_dir(dir.join(name))
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
    let hello = get_controller_bytes("hello");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVm::default();

    let controller = ControllerData::default().with_controller(hello);
    let controller = registry
        .register_controller(&vm, &zkvm, controller)
        .unwrap();

    let ctx = Blake3Historical::load(data).unwrap().context(controller);

    let ret = ctx.entrypoint(&vm, json!({"name": "Valence"})).unwrap()["message"]
        .as_str()
        .unwrap()
        .to_string();

    assert_eq!(ret, "Hello, Valence!");
}

#[test]
fn deploy_storage() {
    let storage = get_controller_bytes("storage");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVm::default();

    let controller = ControllerData::default().with_controller(storage);
    let controller = registry
        .register_controller(&vm, &zkvm, controller)
        .unwrap();

    let ctx = Blake3Historical::load(data).unwrap().context(controller);

    let path = "/var/share/foo.bin";
    let contents = "Valence";

    let exists = ctx
        .entrypoint(&vm, json!({"cmd": "exists", "path": path}))
        .unwrap()["exists"]
        .as_bool()
        .unwrap();

    assert!(!exists);

    ctx.entrypoint(
        &vm,
        json!({"cmd": "set", "path": path, "contents": contents}),
    )
    .unwrap();

    assert_eq!(
        ctx.get_storage_file(path).unwrap(),
        Some(contents.as_bytes().to_vec())
    );

    let ret = ctx
        .entrypoint(&vm, json!({"cmd": "get", "path": path}))
        .unwrap()["b64"]
        .as_str()
        .unwrap()
        .to_string();

    let ret = Base64::decode(ret).unwrap();
    let ret = String::from_utf8(ret).unwrap();

    assert_eq!(ret, contents);

    let path = "/var/share/bar.bin";
    let byte = 0xfa;
    let count = 8 * 1024 * 1024;

    ctx.entrypoint(
        &vm,
        json!({"cmd": "set_large", "path": path, "byte": byte, "count": count}),
    )
    .unwrap();

    assert_eq!(ctx.get_storage_file(path).unwrap(), Some(vec![byte; count]));
}

#[test]
fn deploy_raw_storage() {
    let storage = get_controller_bytes("raw_storage");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVm::default();

    let controller = ControllerData::default().with_controller(storage);
    let controller = registry
        .register_controller(&vm, &zkvm, controller)
        .unwrap();

    let ctx = Blake3Historical::load(data).unwrap().context(controller);

    assert!(ctx.get_raw_storage().unwrap().is_none());

    ctx.entrypoint(&vm, json!({"name": "Valence"})).unwrap();

    let storage = ctx.get_raw_storage().unwrap().unwrap();

    assert_eq!(storage, b"Valence");
}

#[test]
fn deploy_controller() {
    let controller = get_controller_bytes("controller");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVm::default();

    let controller = ControllerData::default().with_controller(controller);
    let controller = registry
        .register_controller(&vm, &zkvm, controller)
        .unwrap();

    let ctx = Blake3Historical::load(data).unwrap().context(controller);

    let ret: Vec<_> = ctx
        .entrypoint(&vm, json!({}))
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect();

    assert_eq!(&controller, ret.as_slice());
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

    let controller = get_controller_bytes("http");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVm::default();

    let controller = ControllerData::default().with_controller(controller);
    let controller = registry
        .register_controller(&vm, &zkvm, controller)
        .unwrap();

    let ctx = Blake3Historical::load(data).unwrap().context(controller);

    let ret = ctx
        .entrypoint(
            &vm,
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
#[ignore = "ALCHEMY_API_KEY required"]
fn deploy_alchemy() {
    let alchemy = get_controller_bytes("alchemy");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVm::default();

    let controller = ControllerData::default().with_controller(alchemy);
    let controller = registry
        .register_controller(&vm, &zkvm, controller)
        .unwrap();

    let ctx = Blake3Historical::load(data).unwrap().context(controller);

    let ret = ctx
        .entrypoint(
            &vm,
            json!({
                "chain": "eth-mainnet",
                "method": "eth_getStorageAt",
                "params": [
              "0xf2B85C389A771035a9Bd147D4BF87987A7F9cf98",
              "0xa",
              "latest"
            ]
            }),
        )
        .unwrap();
    let ret = ret.as_str().unwrap().strip_prefix("0x").unwrap();

    hex::decode(ret).unwrap();
}

#[test]
fn deploy_log() {
    let hello = get_controller_bytes("log");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVm::default();

    let controller = ControllerData::default().with_controller(hello);
    let controller = registry
        .register_controller(&vm, &zkvm, controller)
        .unwrap();

    let ctx = Blake3Historical::load(data).unwrap().context(controller);

    ctx.entrypoint(&vm, json!({"name": "Valence"})).unwrap();

    let mut log = ctx.get_log().unwrap();

    assert_eq!("Hello, Valence!", log.remove(0));
    assert_eq!("Multiple entries", log.remove(0));
}

#[test]
fn deploy_historical() {
    let historical = get_controller_bytes("historical");
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm = ValenceWasm::new(capacity).unwrap();
    let zkvm = MockZkVm::default();

    let controller = ControllerData::default().with_controller(historical);
    let controller = registry
        .register_controller(&vm, &zkvm, controller)
        .unwrap();

    let historical = Blake3Historical::load(data).unwrap();

    let domain = "ethereum";
    let id = DomainData::identifier_from_parts("ethereum");

    let number = 4794837u64;
    let payload = number.to_le_bytes().to_vec();
    let state_root = Blake3Hasher::hash(&payload);
    let block = ValidatedDomainBlock {
        domain: id,
        number,
        root: state_root,
        payload,
    };

    let (previous, smt) = historical.add_validated_block(domain, &block).unwrap();
    let ctx = historical.context(controller);

    let ret = ctx
        .entrypoint(
            &vm,
            json!({
                "domain": domain,
                "number": number,
            }),
        )
        .unwrap();

    let root: Hash = serde_json::from_value(ret["root"].clone()).unwrap();
    let proof: CompoundOpening = serde_json::from_value(ret["proof"].clone()).unwrap();
    let update: HistoricalUpdate = serde_json::from_value(ret["update"].clone()).unwrap();

    assert_eq!(root, smt);
    assert_eq!(update.root, smt);
    assert_eq!(update.previous, previous);
    assert_eq!(update.block.number, number);
    assert_eq!(update.block.root, state_root);
    assert!(proof.verify::<Blake3Hasher>(&smt, &state_root));
}

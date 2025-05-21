use std::{env, fs, path::PathBuf, process::Command};

use valence_coprocessor::{
    mocks::MockVm, Historical, MemoryBackend, ProgramData, Registry, Witness,
};
use valence_coprocessor_sp1::{Mode, Sp1ZkVm};

fn get_hello_bytes() -> Vec<u8> {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = PathBuf::from(dir).join("contrib").join("hello");
    let path = dir.join("target").join("hello.elf");

    let program = dir.join("program");
    let script = dir.join("script");

    assert!(Command::new("cargo")
        .current_dir(&program)
        .args(["prove", "build"])
        .status()
        .unwrap()
        .success());

    assert!(Command::new("cargo")
        .current_dir(&script)
        .arg("run")
        .status()
        .unwrap()
        .success());

    fs::read(path).unwrap()
}

#[test]
#[ignore = "requires SP1 toolchain"]
fn deploy_hello() {
    let hello = get_hello_bytes();
    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 10;
    let mode = Mode::Mock;
    let vm = MockVm;
    let zkvm = Sp1ZkVm::new(mode, capacity).unwrap();

    let library = ProgramData::default().with_circuit(hello);
    let library = registry.register_program(&vm, &zkvm, library).unwrap();
    let ctx = Historical::load(data).unwrap().context(library);

    let witness = String::from("Valence");
    let witness = Witness::Data(witness.as_bytes().to_vec());
    let witness = serde_json::to_value(vec![witness]).unwrap();

    let proof = ctx.get_proof(&vm, &zkvm, witness).unwrap();
    let output: String = zkvm.outputs(&proof).unwrap();

    assert_eq!(output, "Hello, Valence!");
}

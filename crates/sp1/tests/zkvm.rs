use std::{env, fs, path::PathBuf, process::Command};

use valence_coprocessor::{
    mocks::MockModuleVM, Blake3Context, Blake3Hasher, MemoryBackend, ProgramData, Registry, Witness,
};
use valence_coprocessor_sp1::{Mode, Sp1ZkVM};

fn get_hello_bytes() -> Vec<u8> {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = PathBuf::from(dir).join("contrib").join("hello");
    let path = dir.join("target").join("hello.elf");

    if env::var("ZKVM_REBUILD").is_ok() || !path.is_file() {
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
    }

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
    let vm = MockModuleVM;
    let zkvm = Sp1ZkVM::new(mode, capacity).unwrap();

    let program = ProgramData::default().with_zkvm(hello);
    let program = registry
        .register_program::<_, Blake3Hasher, _>(&vm, &zkvm, program)
        .unwrap();

    let ctx = Blake3Context::init(program, data, MockModuleVM, zkvm);

    let witnesses = vec![Witness::Data(b"Valence".to_vec())];
    let proof = ctx.execute_proof(witnesses).unwrap();
    let output = String::from_utf8(proof.outputs).unwrap();

    assert_eq!(output, "Hello, Valence!");
}

use std::{env, fs, path::PathBuf, process::Command};

use msgpacker::{Packable as _, Unpackable as _};
use sp1_sdk::{SP1ProofWithPublicValues, SP1VerifyingKey};
use valence_coprocessor::{
    DomainCircuit as _, DomainLibrary as _, ExecutionContext, MemoryBackend, ProgramData, Registry,
    Witness,
};
use valence_coprocessor_ethereum::{Ethereum, EthereumStateProof};
use valence_coprocessor_sp1::{Mode, Sp1Hasher, Sp1ZkVm};
use valence_coprocessor_wasm::host::ValenceWasm;

type Context = ExecutionContext<
    Sp1Hasher,
    MemoryBackend,
    ValenceWasm<Sp1Hasher, MemoryBackend, Sp1ZkVm>,
    Sp1ZkVm,
>;

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
        .join(format!("valence_coprocessor_{name}_lib.wasm"));

    fs::read(dir).unwrap()
}

fn get_circuit_bytes() -> Vec<u8> {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = PathBuf::from(dir).join("contrib").join("circuit");
    let path = dir.join("target").join("state.elf");

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

fn read_account_proof_test_vector() -> EthereumStateProof {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = PathBuf::from(dir).join("assets").join("account-proof.bin");
    let bytes = fs::read(path).unwrap();

    EthereumStateProof::unpack(&bytes).unwrap().1
}

#[test]
fn check_account_proof_test_vector() {
    let proof = read_account_proof_test_vector();
    let bytes = proof.pack_to_vec();
    let proof = Witness::StateProof(bytes);

    Ethereum::verify(&proof).unwrap();
}

#[test]
#[ignore = "requires SP1 toolchain"]
fn ethereum_state_proof() {
    let domain = get_program_bytes("domain");
    let user = get_program_bytes("user");
    let circuit = get_circuit_bytes();

    let data = MemoryBackend::default();
    let registry = Registry::from(data.clone());

    let capacity = 500;
    let vm: ValenceWasm<Sp1Hasher, MemoryBackend, Sp1ZkVm> = ValenceWasm::new(capacity).unwrap();

    let capacity = 10;
    let mode = Mode::Mock;
    let zkvm = Sp1ZkVm::new(mode, capacity).unwrap();

    Ethereum::deploy(&registry, &vm, domain).unwrap();

    let program = ProgramData::default().with_lib(user).with_circuit(circuit);
    let program = registry.register_program(&vm, &zkvm, program).unwrap();

    let ctx = Context::init(program, data, vm, zkvm.clone());

    let proof = read_account_proof_test_vector();
    let proof = serde_json::to_value(proof).unwrap();
    let proven = ctx.get_program_proof(proof).unwrap();

    let proof: SP1ProofWithPublicValues = bincode::deserialize(&proven.proof).unwrap();

    let vk = ctx.get_program_verifying_key().unwrap();
    let vk: SP1VerifyingKey = bincode::deserialize(&vk).unwrap();

    assert!(zkvm.verify(&vk, &proof));
}

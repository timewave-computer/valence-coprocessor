use std::{env, fs, path::PathBuf};

use msgpacker::{Packable as _, Unpackable as _};
use valence_coprocessor::{DomainCircuit as _, DomainController as _, Hash, StateProof};
use valence_coprocessor_ethereum::{Ethereum, EthereumStateProof};

fn read_account_proof_test_vector() -> (Hash, EthereumStateProof) {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let path = PathBuf::from(dir).join("assets").join("account-proof.bin");
    let bytes = fs::read(path).unwrap();

    <(Hash, EthereumStateProof)>::unpack(&bytes).unwrap().1
}

#[test]
fn check_account_proof_test_vector() {
    let (root, proof) = read_account_proof_test_vector();
    let bytes = proof.pack_to_vec();
    let proof = StateProof {
        domain: Ethereum::ID.to_string(),
        root,
        payload: vec![],
        proof: bytes,
    };

    Ethereum::verify(&proof).unwrap();
}

use std::{env, fs, path::PathBuf};

use sp1_sdk::include_elf;

pub const STATE_PROOF_ELF: &[u8] = include_elf!("state-proof-circuit");

fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = PathBuf::from(dir).parent().unwrap().join("target");
    let path = dir.join("state.elf");

    fs::write(path, STATE_PROOF_ELF).unwrap();
}

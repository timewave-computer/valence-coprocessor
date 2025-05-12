use std::{env, fs, path::PathBuf};

use sp1_sdk::include_elf;

pub const CIRCUIT_ELF: &[u8] = include_elf!("program-circuit");

fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = PathBuf::from(dir).parent().unwrap().join("target");
    let path = dir.join("program.elf");

    fs::write(path, CIRCUIT_ELF).unwrap();
}

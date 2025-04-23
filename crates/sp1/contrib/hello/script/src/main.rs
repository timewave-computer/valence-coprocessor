use std::{env, fs, path::PathBuf};

use sp1_sdk::include_elf;

pub const HELLO_ELF: &[u8] = include_elf!("hello-program");

fn main() {
    let dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let dir = PathBuf::from(dir).parent().unwrap().join("target");
    let path = dir.join("hello.elf");

    fs::write(path, HELLO_ELF).unwrap();
}

#![no_main]
sp1_zkvm::entrypoint!(main);

pub fn main() {
    sp1_zkvm::io::commit_slice(&[]);
}

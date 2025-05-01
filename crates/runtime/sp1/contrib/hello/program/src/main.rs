#![no_main]
sp1_zkvm::entrypoint!(main);

use valence_coprocessor::Witness;

pub fn main() {
    let w = sp1_zkvm::io::read::<Vec<Witness>>();

    let w = match &w[0] {
        Witness::Data(d) => String::from_utf8(d.to_vec()).unwrap(),
        _ => panic!("unexpected data"),
    };
    let w = format!("Hello, {w}!");

    sp1_zkvm::io::commit_slice(w.as_bytes());
}

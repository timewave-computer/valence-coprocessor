use core::array;

use ark_bn254::{Bn254, Fr, G1Affine, G2Affine};
use ark_ff::PrimeField as _;
use ark_groth16::{Groth16, PreparedVerifyingKey, Proof, VerifyingKey};
use ark_serialize::{CanonicalDeserialize as _, Compress, Validate};
use sha2_v0_10_8::{Digest as _, Sha256};
use sp1_sdk::{HashableKey as _, SP1ProofWithPublicValues, SP1VerifyingKey};

mod consts;

/// Returns the prepared verifying key.
pub fn pvk() -> PreparedVerifyingKey<Bn254> {
    ark_groth16::prepare_verifying_key(&VerifyingKey {
        alpha_g1: consts::ALPHA,
        beta_g2: consts::BETA,
        gamma_g2: consts::GAMMA,
        delta_g2: consts::DELTA,
        gamma_abc_g1: consts::GAMMA_ABC.to_vec(),
    })
}

/// Converts the proof and key into a ark-groth16 verifiable artifact.
pub fn convert(vk: &SP1VerifyingKey, proof: &SP1ProofWithPublicValues) -> (Proof<Bn254>, [Fr; 2]) {
    let p = &proof.bytes()[4..];

    let ax: [u8; 32] = array::from_fn(|i| p[31 - i]);
    let ay: [u8; 32] = array::from_fn(|i| p[63 - i]);
    let a = [&ax, &ay, &[0][..]].concat();

    let bx: [u8; 64] = array::from_fn(|i| p[127 - i]);
    let by: [u8; 64] = array::from_fn(|i| p[191 - i]);
    let b = [&bx, &by, &[0][..]].concat();

    let cx: [u8; 32] = array::from_fn(|i| p[223 - i]);
    let cy: [u8; 32] = array::from_fn(|i| p[255 - i]);
    let c = [&cx, &cy, &[0][..]].concat();

    let a = G1Affine::deserialize_with_mode(a.as_slice(), Compress::No, Validate::No)
        .unwrap_or_default();
    let b = G2Affine::deserialize_with_mode(b.as_slice(), Compress::No, Validate::No)
        .unwrap_or_default();
    let c = G1Affine::deserialize_with_mode(c.as_slice(), Compress::No, Validate::No)
        .unwrap_or_default();

    let p = Proof { a, b, c };

    let a = vk.bytes32_raw();
    let a = Fr::from_be_bytes_mod_order(&a);

    let b = proof.public_values.to_vec();
    let mut b: [u8; 32] = Sha256::digest(b).into();
    b[0] &= 0x1F;
    let b = Fr::from_be_bytes_mod_order(&b);

    let public_inputs = [a, b];

    (p, public_inputs)
}

/// Verifies the given proof.
pub fn verify(
    pvk: &PreparedVerifyingKey<Bn254>,
    proof: &Proof<Bn254>,
    public_inputs: &[Fr],
) -> bool {
    match Groth16::<Bn254>::verify_proof(pvk, proof, public_inputs) {
        Ok(f) => f,
        Err(_) => false,
    }
}

#[test]
fn pvk_is_correct() {
    let sp1_vk = &sp1_verifier::GROTH16_VK_BYTES;
    let sp1_vk = sp1_verifier::load_ark_groth16_verifying_key_from_bytes(sp1_vk).unwrap();
    let sp1_vk: VerifyingKey<Bn254> = unsafe { core::mem::transmute(sp1_vk) };

    assert_eq!(sp1_vk, pvk().vk);
}

#[test]
#[cfg(feature = "std")]
fn proof_conversion_works() {
    use std::{fs, path::PathBuf};

    let path = env!("CARGO_MANIFEST_DIR");
    let sample = PathBuf::from(path)
        .join("assets")
        .join("sample-groth16-proof");

    let vk = fs::read(sample.join("hello.vk")).unwrap();
    let vk: SP1VerifyingKey = bincode::deserialize(&vk).unwrap();

    let proof = SP1ProofWithPublicValues::load(sample.join("hello.proof")).unwrap();

    let (proof, inputs) = convert(&vk, &proof);

    assert!(verify(&pvk(), &proof, &inputs));
}

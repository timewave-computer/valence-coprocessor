use valence_coprocessor_types::{Blake3Hasher, DomainData, ValidatedDomainBlock};

use crate::{Blake3Historical, MemoryBackend};

use super::*;

#[test]
fn crosschain_coprocessor_witness_validates() {
    let data = MemoryBackend::default();
    let historical = Blake3Historical::load(data).unwrap();

    let ethereum = "ethereum";
    let neutron = "neutron";
    let solana = "solana";

    let cases = vec![
        (ethereum, 238792),
        (ethereum, 238797),
        (ethereum, 238798),
        (neutron, 832348),
        (neutron, 4839),
        (ethereum, 238799),
        (solana, 238794),
        (solana, 238795),
        (ethereum, 238550),
        (solana, 238910),
    ];

    for (d, n) in &cases {
        create_block(&historical, d, *n);
    }

    let data = historical.data().clone();
    let root = historical.current();
    let witnesses = cases.into_iter().map(create_state_proof).collect();

    let witnesses = WitnessCoprocessor::try_from_witnesses::<Blake3Hasher, MemoryBackend>(
        data, root, witnesses,
    )
    .unwrap()
    .validate::<Blake3Hasher>()
    .unwrap();

    assert_eq!(witnesses.root, root);
}

fn create_state_root(number: u64) -> Hash {
    let payload = number.to_le_bytes().to_vec();

    Blake3Hasher::hash(&payload)
}

fn create_block(historical: &Blake3Historical<MemoryBackend>, domain: &str, number: u64) {
    let id = DomainData::identifier_from_parts(domain);
    let payload = number.to_le_bytes().to_vec();
    let root = create_state_root(number);
    let block = ValidatedDomainBlock {
        domain: id,
        number,
        root,
        payload,
    };

    historical.add_validated_block(domain, &block).unwrap();
}

fn create_state_proof(arg: (&str, u64)) -> Witness {
    let (domain, number) = arg;

    let domain = DomainData::identifier_from_parts(domain);
    let state_root = create_state_root(number);
    let payload = Vec::new();
    let proof = Vec::new();

    StateProof {
        domain,
        number,
        state_root,
        payload,
        proof,
    }
    .into()
}

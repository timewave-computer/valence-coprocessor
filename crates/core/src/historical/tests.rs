use proptest::prelude::*;
use rand::{rngs::StdRng, SeedableRng as _};
use valence_coprocessor_types::Hash;

use crate::MemoryBackend;

use super::*;

#[test]
fn historical_tree_compound_domain_works() {
    let data = MemoryBackend::default();
    let historical = Blake3Historical::load(data).unwrap();

    validate_block_creation(&historical, "ethereum", 238792);
    validate_block_creation(&historical, "ethereum", 238797);
    validate_block_creation(&historical, "ethereum", 238798);
    validate_block_creation(&historical, "ethereum", 238799);
    validate_block_creation(&historical, "solana", 238794);
    validate_block_creation(&historical, "solana", 238795);
    validate_block_creation(&historical, "ethereum", 238550);
    validate_block_creation(&historical, "solana", 238910);
}

proptest! {
    #[test]
    fn historical_tree_property_check(seed: u64, count: u8, domains: u8) {
        let data = MemoryBackend::default();
        let historical = Blake3Historical::load(data).unwrap();
        let domains = domains.max(1);

        let rng = &mut StdRng::seed_from_u64(seed);
        let values: Vec<String> = (0..domains).map(|_| {
            let mut d = Hash::default();

            rng.fill_bytes(&mut d);

            hex::encode(d)
        }).collect();

        for _ in 0..count {
            let d = (rng.next_u32() & (u8::MAX as u32)) as u8;
            let d = d % domains;

            let domain = &values[d as usize][..];
            let number = rng.next_u64();
            let id = DomainData::identifier_from_parts(domain);

            if !historical.block_exists(&id, number).unwrap() {
                validate_block_creation(&historical, domain, number);
            }
        }
    }
}

fn validate_block_creation<D: DataBackend>(
    historical: &Blake3Historical<D>,
    domain: &str,
    number: u64,
) {
    let payload = number.to_le_bytes().to_vec();
    let root = Blake3Hasher::hash(&payload);
    let block = ValidatedDomainBlock {
        domain: DomainData::identifier_from_parts(domain),
        number,
        root,
        payload,
    };

    let root = block.root;

    if !historical
        .block_exists(&block.domain, block.number)
        .unwrap()
    {
        let proof = historical
            .get_historical_non_membership_proof(&block.domain, block.number)
            .unwrap();

        assert!(historical.verify_non_membership(&proof, &block.domain, block.number, &block.root));
    }

    let (previous, smt) = historical.add_validated_block(domain, &block).unwrap();

    let proof = historical.get_latest_historical_transition_proof().unwrap();
    let update = proof.verify::<Blake3Hasher>().unwrap();

    assert_eq!(update.root, smt);
    assert_eq!(update.previous, previous);
    assert_eq!(update.block, block);

    let proof = historical.get_block_proof(block.domain, number).unwrap();
    let smt_p = Blake3Historical::compute_root(&proof, &root);
    let domain_id_p = Blake3Historical::get_domain_id(&proof).unwrap();
    let number_p = Blake3Historical::get_block_number(&proof).unwrap();

    assert_eq!(smt_p, smt);
    assert_eq!(domain_id_p, block.domain);
    assert_eq!(number_p, number);

    let update = historical.get_historical_update(&smt).unwrap().unwrap();

    assert_eq!(smt, update.root);
    assert_eq!(previous, update.previous);
    assert_eq!(block, update.block);
}

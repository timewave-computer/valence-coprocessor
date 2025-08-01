use core::{array, marker::PhantomData};

#[cfg(feature = "std")]
use std::sync::{Arc, Mutex, RwLock};

use msgpacker::Unpackable as _;
use valence_coprocessor_merkle::{CompoundOpening, CompoundOpeningBuilder, Smt};
use valence_coprocessor_types::{DomainData, Hash, HistoricalUpdate, ValidatedDomainBlock};

use crate::{Blake3Hasher, DataBackend, Hasher};

#[cfg(feature = "std")]
mod use_std;

#[cfg(test)]
mod tests;

/// Historical tree with blake3 hasher.
pub type Blake3Historical<D> = Historical<Blake3Hasher, D>;

/// A historical SMT coordinator.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Historical<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    #[cfg(feature = "std")]
    current: Arc<RwLock<Hash>>,

    #[cfg(feature = "std")]
    history: Arc<RwLock<Hash>>,

    #[cfg(feature = "std")]
    next: Arc<Mutex<Hash>>,

    data: D,
    phantom: PhantomData<H>,
}

impl<H, D> Historical<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    /// Prefix for the current tree.
    pub const PREFIX_CURRENT: &[u8] = b"historical-current";

    /// Prefix for the root tree.
    pub const PREFIX_HISTORICAL: &[u8] = b"historical-root";

    /// Prefix for the latest block mapping.
    pub const PREFIX_LATEST: &[u8] = b"historical-latest";

    /// Prefix for the history tree.
    pub const PREFIX_HISTORY: &[u8] = b"historical-history";

    /// Returns the underlying data reference.
    pub fn data(&self) -> D {
        self.data.clone()
    }

    /// Returns the latest validated block for the provided domain.
    pub fn get_latest_block(
        data: &D,
        domain: &str,
    ) -> anyhow::Result<Option<ValidatedDomainBlock>> {
        let domain = DomainData::identifier_from_parts(domain);
        let block = data
            .get(Self::PREFIX_LATEST, &domain)?
            .and_then(|l| ValidatedDomainBlock::unpack(&l).map(|r| r.1).ok());

        Ok(block)
    }

    /// Returns a SMT associated with the data backend.
    pub fn smt(&self) -> Smt<D, H> {
        Smt::from(self.data.clone())
    }

    /// Get the block proof for the provided domain and block number.
    pub fn get_block_proof_with_historical(
        data: D,
        root: Hash,
        domain_id: Hash,
        number: u64,
    ) -> anyhow::Result<CompoundOpening> {
        let smt: Smt<D, H> = Smt::from(data).with_namespace(domain_id);
        let key = Historical::<H, ()>::block_number_to_key(number);

        CompoundOpeningBuilder::new(root)
            .with_tree(Self::PREFIX_HISTORICAL, domain_id)
            .with_tree(domain_id, key)
            .opening(smt)
    }

    /// Get the block proof for the provided domain and block number.
    pub fn get_block_proof_for_domain_with_historical(
        data: D,
        root: Hash,
        domain: &str,
        number: u64,
    ) -> anyhow::Result<CompoundOpening> {
        let domain = DomainData::identifier_from_parts(domain);

        Self::get_block_proof_with_historical(data, root, domain, number)
    }

    /// Get the historical update for the provided historical tree root.
    pub fn get_historical_update_with_tree(
        data: D,
        history: Hash,
        root: Hash,
    ) -> anyhow::Result<Option<HistoricalUpdate>> {
        let smt: Smt<D, H> = Smt::from(data).with_namespace(Self::PREFIX_HISTORY);

        let previous = smt
            .get_keyed_opening(history, &root)
            .ok()
            .filter(|k| k.key == root)
            .map(|k| k.node);

        let block = smt.get_key_data(&root)?;

        let (previous, block) = match previous.zip(block) {
            Some((p, b)) => (p, b),
            None => return Ok(None),
        };

        let block = ValidatedDomainBlock::unpack(&block)?.1;

        Ok(Some(HistoricalUpdate {
            root,
            previous,
            block,
        }))
    }
}

impl<H> Historical<H, ()>
where
    H: Hasher,
{
    /// Extracts the historical root from the provided opening.
    pub fn compute_root(proof: &CompoundOpening, state_root: &Hash) -> Hash {
        proof.root::<H>(state_root)
    }

    /// Extracts the block number from the compound opening.
    pub fn get_block_number(proof: &CompoundOpening) -> Option<u64> {
        (proof.trees.len() == 2).then(|| {
            let number = &proof.trees[0].key[..8];
            let number = number.try_into().unwrap();

            u64::from_be_bytes(number)
        })
    }

    /// Extracts the domain id from the compound opening.
    pub fn get_domain_id(proof: &CompoundOpening) -> Option<Hash> {
        (proof.trees.len() == 2).then_some(proof.trees[1].key)
    }

    /// Compute the historical Merkle key from the block number.
    pub fn block_number_to_key(number: u64) -> Hash {
        let key = number.to_be_bytes();

        array::from_fn(|i| key.get(i).copied().unwrap_or(0))
    }
}

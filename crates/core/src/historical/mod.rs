use core::marker::PhantomData;

#[cfg(feature = "std")]
use std::sync::{Arc, Mutex, RwLock};

use msgpacker::Unpackable as _;
use valence_coprocessor_merkle::{CompoundOpeningBuilder, Smt};
use valence_coprocessor_types::{
    CompoundOpening, DomainData, Hash, HistoricalNonMembership, HistoricalTransitionProof,
    HistoricalUpdate, Preimage, ValidatedDomainBlock,
};

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

    /// Prefix for the history tree indexed by previous root.
    pub const PREFIX_HISTORY_PREV: &[u8] = b"historical-history-prev";

    /// Prefix for the history tree indexed by current root.
    pub const PREFIX_HISTORY_CUR: &[u8] = b"historical-history-cur";

    /// Returns the underlying data reference.
    pub fn data(&self) -> &D {
        &self.data
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
        let key = HistoricalUpdate::block_number_to_key(number);

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
    pub fn get_historical_update(&self, root: &Hash) -> anyhow::Result<Option<HistoricalUpdate>> {
        Self::get_historical_update_with_data(&self.data, root)
    }

    /// Get the historical update for the provided historical tree root.
    pub fn get_historical_update_with_data(
        data: &D,
        root: &Hash,
    ) -> anyhow::Result<Option<HistoricalUpdate>> {
        let data = data.get(Self::PREFIX_HISTORY_CUR, root)?;
        let data = data.map(|d| HistoricalUpdate::unpack(&d)).transpose()?;

        Ok(data.map(|d| d.1))
    }

    /// Get the historical update for the provided historical tree root.
    pub fn get_historical_update_from_previous(
        &self,
        root: &Hash,
    ) -> anyhow::Result<Option<HistoricalUpdate>> {
        Self::get_historical_update_from_previous_with_data(&self.data, root)
    }

    /// Get the historical update for the provided historical tree root.
    pub fn get_historical_update_from_previous_with_data(
        data: &D,
        root: &Hash,
    ) -> anyhow::Result<Option<HistoricalUpdate>> {
        let data = data.get(Self::PREFIX_HISTORY_PREV, root)?;
        let data = data.map(|d| HistoricalUpdate::unpack(&d)).transpose()?;

        Ok(data.map(|d| d.1))
    }

    /// Computes a proof of non-membership of the provided block.
    pub fn get_historical_non_membership_proof_with_data(
        data: D,
        root: Hash,
        domain_id: &Hash,
        number: u64,
    ) -> anyhow::Result<HistoricalNonMembership> {
        let smt: Smt<D, H> = Smt::from(data).with_namespace(Self::PREFIX_HISTORICAL);
        let mut historical = smt.get_non_membership_opening(root, domain_id)?;

        let domain = match &historical.preimage {
            Preimage::Zero => None,
            Preimage::Node(_) => anyhow::bail!("unexpected pre-computed node value"),
            Preimage::Data(_) => {
                let domain = smt.get_keyed_opening(root, domain_id)?.node;

                // The pre-image of the compound tree is not the hash of data
                historical.preimage = Preimage::Node(domain);

                let smt = smt.with_namespace(domain_id);
                let key = HistoricalUpdate::block_number_to_key(number);
                let mut proof = smt.get_non_membership_opening(domain, &key)?;

                // the stored data is the arbitrary payload, and the leaf is the state root.
                let preimage = smt.get_keyed_opening(domain, &key)?.node;
                proof.preimage = Preimage::Node(preimage);

                Some(proof)
            }
        };

        Ok(HistoricalNonMembership { domain, historical })
    }

    /// Computes a historical tree transition proof for the provided root.
    pub fn get_historical_transition_proof_with_data(
        data: D,
        root: &Hash,
    ) -> anyhow::Result<HistoricalTransitionProof> {
        let update = data
            .get(Self::PREFIX_HISTORY_CUR, root)?
            .and_then(|c| HistoricalUpdate::unpack(&c).ok())
            .map(|(_, c)| c)
            .ok_or_else(|| anyhow::anyhow!("no chained data for the provided root"))?;

        let previous = Self::get_historical_non_membership_proof_with_data(
            data.clone(),
            update.previous,
            &update.block.domain,
            update.block.number,
        )?;

        let proof = Self::get_block_proof_with_historical(
            data.clone(),
            update.root,
            update.block.domain,
            update.block.number,
        )?;

        Ok(HistoricalTransitionProof {
            previous,
            update,
            proof,
        })
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
}

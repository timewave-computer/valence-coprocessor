use valence_coprocessor_types::{
    CompoundOpening, DataBackend, Hash, Hasher, HistoricalUpdate, ValidatedDomainBlock,
};

use crate::{ExecutionContext, Historical};

impl<H, D> ExecutionContext<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    /// Controller function name to validate blocks.
    pub const CONTROLLER_VALIDATE_BLOCK: &str = "validate_block";

    /// Returns the last included block for the provided domain.
    pub fn get_latest_block(&self, domain: &str) -> anyhow::Result<Option<ValidatedDomainBlock>> {
        Historical::<H, D>::get_latest_block(&self.data, domain)
    }

    /// Returns a Merkle proof that opens a block number to the historical root.
    pub fn get_block_proof(
        &self,
        domain: &str,
        block_number: u64,
    ) -> anyhow::Result<CompoundOpening> {
        Historical::<H, D>::get_block_proof_for_domain_with_historical(
            self.data.clone(),
            self.historical,
            domain,
            block_number,
        )
    }

    /// Returns the chained historical update from the current historical root.
    pub fn get_historical_update(&self, root: &Hash) -> anyhow::Result<Option<HistoricalUpdate>> {
        Historical::<H, D>::get_historical_update_with_data(&self.data, root)
    }

    /// Returns the chained historical update from the previous historical root.
    pub fn get_historical_update_from_previous(
        &self,
        root: &Hash,
    ) -> anyhow::Result<Option<HistoricalUpdate>> {
        Historical::<H, D>::get_historical_update_from_previous_with_data(&self.data, root)
    }
}

use core::marker::PhantomData;
use std::sync::{Arc, Mutex, RwLock};

use msgpacker::Packable as _;
use serde_json::Value;
use uuid::Uuid;
use valence_coprocessor_merkle::Smt;
use valence_coprocessor_types::{
    BlockAdded, CompoundOpening, DataBackend, DomainData, Hash, Hasher, HistoricalTransitionProof,
    HistoricalUpdate, ValidatedBlock, ValidatedDomainBlock,
};

use crate::{ExecutionContext, Historical, HistoricalNonMembership, Vm};

impl<H, D> Historical<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    /// Returns the latest SMT.
    pub fn current(&self) -> Hash {
        *self.current.read().expect("infallible lock")
    }

    /// Initializes a new context.
    pub fn context(&self, controller: Hash) -> ExecutionContext<H, D> {
        let current = self.current();

        ExecutionContext::init(controller, current, self.data.clone())
    }

    /// Initializes a new context with the provided historical root.
    pub fn context_with_root(&self, controller: Hash, root: Hash) -> ExecutionContext<H, D> {
        ExecutionContext::init(controller, root, self.data.clone())
    }

    /// Loads a new instance of the historical tree from the data backend.
    pub fn load(data: D) -> anyhow::Result<Self> {
        let empty = Smt::<D, H>::empty_tree_root();
        let current = data.get(Self::PREFIX_CURRENT, &[])?;
        let current = current
            .map(Hash::try_from)
            .transpose()
            .map_err(|_| anyhow::anyhow!("failed to load current tree from the database"))?
            .unwrap_or(empty);

        let next = Arc::new(Mutex::new(current));
        let current = Arc::new(RwLock::new(current));

        Ok(Self {
            current,
            next,
            data,
            phantom: PhantomData,
        })
    }

    /// Adds a validated block.
    ///
    /// Returns a tuple containing `(previous smt root, smt root)`.
    pub fn add_validated_block(
        &self,
        domain: &str,
        block: &ValidatedDomainBlock,
    ) -> anyhow::Result<(Hash, Hash)> {
        let prev_smt;
        let latest;
        let smt = {
            let mut next = self
                .next
                .lock()
                .map_err(|e| anyhow::anyhow!("historical lock is poisoned: {e}"))?;

            latest = Self::get_latest_block(&self.data, domain)?;

            prev_smt = *next;

            let smt = prev_smt;
            let tree = self.smt();
            let tree = tree.with_namespace(Self::PREFIX_HISTORICAL);
            let opening = tree.get_keyed_opening(smt, &block.domain)?;

            // Use the node value if matches; otherwise, create a new sub-tree.
            let leaf = if opening.key == Some(block.domain) {
                opening.node
            } else {
                Hash::default()
            };

            let tree = tree.with_namespace(block.domain);
            let key = HistoricalUpdate::block_number_to_key(block.number);
            let leaf = tree.insert_with_leaf(leaf, &key, block.root, &block.payload)?;

            let tree = tree.with_namespace(Self::PREFIX_HISTORICAL);
            let smt = tree.insert_compound(smt, &block.domain, leaf)?;

            // update chained history (must be infallible)

            // if repeated block, then don't update chain
            if smt != prev_smt {
                let uuid = Uuid::now_v7().as_u128().to_be_bytes();
                let chained = HistoricalUpdate {
                    uuid,
                    previous: prev_smt,
                    root: smt,
                    block: block.clone(),
                }
                .pack_to_vec();

                self.data
                    .set(Self::PREFIX_HISTORY_PREV, &prev_smt, &chained)?;

                self.data.set(Self::PREFIX_HISTORY_CUR, &smt, &chained)?;

                // update computed; override control vars & database

                match self.current.write() {
                    Ok(mut c) => *c = smt,
                    Err(e) => tracing::warn!("failed to update current historical: {e}"),
                }

                if let Err(e) = self.data.set(Self::PREFIX_CURRENT, &[], &smt) {
                    tracing::error!("failed to update current smt: {e}");
                }

                *next = smt;
            }

            smt
        };

        if latest.filter(|b| b.number > block.number).is_none() {
            if let Err(e) = self
                .data
                .set(Self::PREFIX_LATEST, &block.domain, &block.pack_to_vec())
            {
                tracing::error!("error updating latest block for domain `{domain}`: {e}");
            }
        }

        Ok((prev_smt, smt))
    }

    /// Adds a new block.
    ///
    /// It will be validated on the domain controller.
    pub fn add_domain_block<VM>(
        &self,
        vm: &VM,
        domain: &str,
        args: Value,
    ) -> anyhow::Result<BlockAdded>
    where
        VM: Vm<H, D>,
    {
        let id = DomainData::identifier_from_parts(domain);
        let ctx = self.context(id);

        tracing::debug!("calling domain controller for {}...", domain);

        let validated = vm.execute(
            &ctx,
            &id,
            ExecutionContext::<H, D>::CONTROLLER_VALIDATE_BLOCK,
            args,
        )?;

        tracing::debug!("block validated for domain {}...", domain);

        let ValidatedBlock {
            number,
            root,
            payload,
        } = serde_json::from_value(validated)?;

        let exists = self.block_exists(&id, number)?;

        anyhow::ensure!(!exists, "cannot override blocks");

        let validated = ValidatedDomainBlock {
            domain: id,
            number,
            root,
            payload,
        };

        let (prev_smt, smt) = self.add_validated_block(domain, &validated)?;

        Ok(BlockAdded {
            domain: domain.into(),
            prev_smt,
            smt,
            log: ctx.get_log().unwrap_or_default(),
            block: validated,
        })
    }

    /// Get the block proof for the provided domain and block number.
    pub fn get_block_proof_for_domain(
        &self,
        domain: &str,
        number: u64,
    ) -> anyhow::Result<CompoundOpening> {
        let root = self.current();

        Self::get_block_proof_for_domain_with_historical(self.data.clone(), root, domain, number)
    }

    /// Get the block proof for the provided domain and block number.
    pub fn get_block_proof(&self, domain_id: Hash, number: u64) -> anyhow::Result<CompoundOpening> {
        let root = self.current();

        Self::get_block_proof_with_historical(self.data.clone(), root, domain_id, number)
    }

    /// Returns `true` if the provided block exists for the domain.
    pub fn block_exists(&self, domain_id: &Hash, number: u64) -> anyhow::Result<bool> {
        let key = HistoricalUpdate::block_number_to_key(number);

        Ok(self.data.get(domain_id, &key)?.is_some())
    }

    /// Computes a proof of non-membership of the provided block.
    pub fn get_historical_non_membership_proof(
        &self,
        domain_id: &Hash,
        number: u64,
    ) -> anyhow::Result<HistoricalNonMembership> {
        let root = self.current();

        Self::get_historical_non_membership_proof_with_data(
            self.data.clone(),
            root,
            domain_id,
            number,
        )
    }

    /// Verifies the non-membership proof of the block.
    pub fn verify_non_membership(
        &self,
        proof: &HistoricalNonMembership,
        domain_id: &Hash,
        number: u64,
        state_root: &Hash,
    ) -> bool {
        let root = self.current();

        proof.verify::<H>(&root, domain_id, number, state_root)
    }

    /// Computes a historical tree transition proof for the provided root.
    pub fn get_latest_historical_transition_proof(
        &self,
    ) -> anyhow::Result<HistoricalTransitionProof> {
        let root = self.current();

        Self::get_historical_transition_proof_with_data(self.data.clone(), &root)
    }
}

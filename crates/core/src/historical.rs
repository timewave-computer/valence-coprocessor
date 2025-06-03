use core::marker::PhantomData;
use std::sync::{Arc, Mutex, RwLock};

use msgpacker::Packable as _;
use serde_json::Value;

use crate::{
    Blake3Hasher, BlockAdded, DataBackend, DomainData, ExecutionContext, Hash, Hasher, Smt,
    ValidatedBlock, ValidatedDomainBlock, Vm,
};

/// Historical tree with blake3 hasher.
pub type Blake3Historical<D> = Historical<Blake3Hasher, D>;

/// A historical SMT coordinator.
#[derive(Debug, Clone)]
pub struct Historical<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    current: Arc<RwLock<Hash>>,
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

    /// Loads a new instance of the historical tree from the data backend.
    pub fn load(data: D) -> anyhow::Result<Self> {
        let empty = Smt::<D, H>::empty_tree_root();
        let current = data.get(Self::PREFIX_CURRENT, &[])?;
        let mut current = current
            .map(Hash::try_from)
            .transpose()
            .map_err(|_| anyhow::anyhow!("failed to load current tree from the database"))?
            .unwrap_or(empty);

        if current == empty {
            current =
                Smt::<D, H>::from(data.clone()).insert(empty, &H::hash(b"valence"), b"valence")?;
        }

        let next = Arc::new(Mutex::new(current));
        let current = Arc::new(RwLock::new(current));

        Ok(Self {
            current,
            next,
            data,
            phantom: PhantomData,
        })
    }

    /// Returns the current SMT.
    ///
    /// The context should be initialized from this value. It may be lagged, but is never locked.
    pub fn current(&self) -> Hash {
        *self
            .current
            .read()
            .expect("current is never mutably locked")
    }

    /// Initializes a new context.
    pub fn context(&self, controller: Hash) -> ExecutionContext<H, D> {
        let current = self.current();

        ExecutionContext::init(controller, current, self.data.clone())
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

        let validated = vm.execute(
            &ctx,
            &id,
            ExecutionContext::<H, D>::CONTROLLER_VALIDATE_BLOCK,
            args,
        )?;

        let validated: ValidatedBlock = serde_json::from_value(validated)?;
        let key = H::key(domain, &validated.root);
        let validated = ValidatedDomainBlock {
            domain: id,
            number: validated.number,
            root: validated.root,
            key,
            payload: validated.payload,
        };

        let packed = validated.pack_to_vec();
        let smt = Smt::<D, H>::from(self.data.clone());
        let prev_smt;
        let post_smt;

        // Everything is validated, lock the write

        {
            // lock other threads from adding blocks

            let mut next = self
                .next
                .lock()
                .map_err(|e| anyhow::anyhow!("error locking the historical update: {e}"))?;

            prev_smt = *next;

            *next = smt.insert(*next, &validated.key, &validated.payload)?;

            post_smt = *next;

            {
                match self.current.try_write() {
                    Ok(mut c) => *c = *next,
                    Err(e) => tracing::warn!("skipped locking current historical: {e}"),
                }
            }

            if let Err(e) = self.data.set(Self::PREFIX_CURRENT, &[], &*next) {
                tracing::error!("failed updating current tree: {e}");
            }

            // This update is not critical as the next deployed block will override an eventual
            // error.
            match ctx.get_latest_block(domain)? {
                None => {
                    if let Err(e) =
                        self.data
                            .set(ExecutionContext::<H, D>::PREFIX_BLOCK, &id, &packed)
                    {
                        tracing::error!(
                            "failed updating domain `{domain}` block {}: {e}",
                            validated.number
                        );
                    }
                }

                Some(b) if b.number < validated.number => {
                    if let Err(e) =
                        self.data
                            .set(ExecutionContext::<H, D>::PREFIX_BLOCK, &id, &packed)
                    {
                        tracing::error!(
                            "failed updating domain `{domain}` block {}: {e}",
                            validated.number
                        );
                    }
                }

                _ => (),
            }
        }

        Ok(BlockAdded {
            domain: domain.into(),
            prev_smt,
            smt: post_smt,
            log: ctx.get_log().unwrap_or_default(),
            block: validated,
        })
    }
}

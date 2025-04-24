use valence_coprocessor::Hash;

pub mod registry;

pub struct Api;

fn try_slice_to_hash(bytes: &[u8]) -> anyhow::Result<Hash> {
    Hash::try_from(bytes).map_err(|e| anyhow::anyhow!("error converting bytes to hash: {e}"))
}

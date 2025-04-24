use valence_coprocessor::Hash;

pub mod registry;

pub struct Api;

fn try_str_to_hash(hash: &str) -> anyhow::Result<Hash> {
    let bytes =
        hex::decode(hash).map_err(|e| anyhow::anyhow!("error converting str to hash: {e}"))?;

    Hash::try_from(bytes).map_err(|_| anyhow::anyhow!("error converting bytes to hash"))
}

use alloc::{string::String, vec::Vec};
use base64::{engine::general_purpose::STANDARD, Engine as _};

/// A base64 encoder.
#[derive(Debug, Default, Clone, Copy)]
pub struct Base64;

impl Base64 {
    /// Encodes the provided bytes into base64.
    pub fn encode<B: AsRef<[u8]>>(bytes: B) -> String {
        STANDARD.encode(bytes.as_ref())
    }

    /// Decodes the provided base64 into bytes.
    pub fn decode<B: AsRef<str>>(b64: B) -> anyhow::Result<Vec<u8>> {
        STANDARD
            .decode(b64.as_ref())
            .map_err(|e| anyhow::anyhow!("failed to decode base64: {e}"))
    }
}

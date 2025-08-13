use poem::{http::StatusCode, Endpoint, Error, Request};
use valence_coprocessor::Hash;

use crate::Historical;

pub async fn context<E: Endpoint>(next: E, mut req: Request) -> poem::Result<E::Output> {
    // currently, controller id is optional to prevent breaking changes
    let controller = req
        .header("valence-coprocessor-circuit")
        .map(try_str_to_hash)
        .transpose()?
        .unwrap_or_default();

    let root = req
        .header("valence-coprocessor-root")
        .map(try_str_to_hash)
        .transpose()?;

    let ext = req.extensions_mut();
    let historical: &Historical = ext
        .get()
        .ok_or_else(|| Error::from_status(StatusCode::INTERNAL_SERVER_ERROR))?;

    let ctx = match root {
        Some(root) => historical.context_with_root(controller, root),
        None => historical.context(controller),
    };

    ext.insert(ctx);

    next.call(req).await
}

fn try_str_to_hash(hash: &str) -> poem::Result<Hash> {
    let bytes = hex::decode(hash).map_err(|_| Error::from_status(StatusCode::BAD_REQUEST))?;

    Hash::try_from(bytes).map_err(|_| Error::from_status(StatusCode::BAD_REQUEST))
}

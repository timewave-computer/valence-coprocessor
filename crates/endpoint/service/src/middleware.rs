use poem::{
    http::{Method, StatusCode},
    Body, Endpoint, Error, Request,
};
use serde_json::Value;
use valence_coprocessor::Hash;
use valence_crypto_utils::Ecdsa;

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

    let signature = req
        .header("valence-coprocessor-signature")
        .map(String::from);

    let mut owner = None;
    if let Some(signature) = signature {
        let signature = const_hex::decode(signature)
            .map_err(|e| Error::from_string(e.to_string(), StatusCode::BAD_REQUEST))?;

        let data = match req.method() {
            &Method::GET => Value::Null,

            _ => {
                let body = req.take_body();
                let data: Value = body.into_json().await?;

                let body = Body::from_json(&data).map_err(|e| {
                    Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR)
                })?;
                req.set_body(body);

                data
            }
        };

        let message = serde_json::to_vec(&data)
            .map_err(|e| Error::from_string(e.to_string(), StatusCode::INTERNAL_SERVER_ERROR))?;

        let uuid = req
            .header("valence-coprocessor-uuid")
            .map(String::from)
            .unwrap_or_default();

        let recovered = Ecdsa::recover_from_json_with_id(&signature, uuid.as_bytes(), &message)
            .map_err(|e| Error::from_string(e.to_string(), StatusCode::BAD_REQUEST))?;

        owner.replace(recovered);
    }

    let ext = req.extensions_mut();
    let historical: &Historical = ext
        .get()
        .ok_or_else(|| Error::from_status(StatusCode::INTERNAL_SERVER_ERROR))?;

    let mut ctx = match root {
        Some(root) => historical.context_with_root(controller, root),
        None => historical.context(controller),
    };

    if let Some(owner) = owner {
        ctx = ctx.with_owner(owner);
    }

    ext.insert(ctx);

    next.call(req).await
}

fn try_str_to_hash(hash: &str) -> poem::Result<Hash> {
    let bytes = hex::decode(hash).map_err(|_| Error::from_status(StatusCode::BAD_REQUEST))?;

    Hash::try_from(bytes).map_err(|_| Error::from_status(StatusCode::BAD_REQUEST))
}

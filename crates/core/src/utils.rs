//! A collection of utils.

use std::time;

use reqwest::blocking::Client;
use serde_json::Value;

/// Perform a HTTP request.
///
/// # Example
///
/// ```rust,ignore
/// use valence_coprocessor::utils;
///
/// let ret = utils::http(&serde_json::json!({
///     "url": "https://httpbin.org/post",
///     "method": "post",
///     "headers": {
///         "Accept": "application/json",
///         "Content-Type": "text/html"
///     },
///     "body": "foo",
/// }))
/// .unwrap();
///
/// assert_eq!(ret["body"]["data"].as_str().unwrap(), "foo");
/// ```
pub fn http(args: &Value) -> anyhow::Result<Value> {
    let url = args
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("no provided url"))?;

    let method = match args.get("method").and_then(Value::as_str) {
        Some(m) => m.to_lowercase(),
        None => "post".into(),
    };

    let mut client = match method.as_str() {
        "delete" => Client::new().delete(url),
        "get" => Client::new().get(url),
        "head" => Client::new().head(url),
        "patch" => Client::new().patch(url),
        "post" => Client::new().post(url),
        "put" => Client::new().put(url),
        _ => anyhow::bail!("unknown method `{}`", method),
    };

    client = client.timeout(time::Duration::from_secs(5));

    if let Some(a) = args.get("basic_auth") {
        let username = match a.get("username").and_then(Value::as_str) {
            Some(u) => u,
            None => anyhow::bail!("invalid basic_auth username."),
        };

        let password = match a.get("password") {
            Some(Value::String(p)) => Some(p),
            None => None,
            _ => anyhow::bail!("invalid basic_auth password."),
        };

        client = client.basic_auth(username, password);
    }

    match args.get("bearer") {
        Some(Value::String(b)) => client = client.bearer_auth(b),
        None => (),
        _ => anyhow::bail!("invalid bearer argument"),
    }

    match args.get("body") {
        Some(Value::String(b)) => client = client.body(b.clone()),
        Some(Value::Array(b)) => {
            let b: Vec<u8> = b
                .iter()
                .map(|b| {
                    b.as_u64()
                        .map(|b| b as u8)
                        .ok_or_else(|| anyhow::anyhow!("invalid body element"))
                })
                .collect::<anyhow::Result<Vec<u8>>>()?;

            client = client.body(b);
        }
        None => (),
        _ => anyhow::bail!("invalid body"),
    }

    let mut wants = "data";

    match args.get("headers") {
        Some(Value::Object(h)) => {
            for (k, v) in h.iter() {
                match v.as_str() {
                    Some(v) => {
                        if k.to_lowercase() == "accept" && v.to_lowercase() == "application/json" {
                            wants = "json"
                        }
                        if k.to_lowercase() == "accept" && v.to_lowercase() == "text/html" {
                            wants = "text"
                        }

                        client = client.header(k, v)
                    }
                    None => anyhow::bail!("invalid header value"),
                }
            }
        }
        None => (),
        _ => anyhow::bail!("invalid header"),
    }

    if let Some(j) = args.get("json") {
        client = client.json(j);
    }

    match args.get("query") {
        Some(Value::Object(h)) => {
            let mut q = Vec::with_capacity(h.len());

            for (k, v) in h.iter() {
                match v.as_str() {
                    Some(v) => q.push((k, v)),
                    None => anyhow::bail!("invalid query element"),
                }
            }

            client = client.query(&q);
        }
        None => (),
        _ => anyhow::bail!("invalid query"),
    }

    let ret = client.send()?;
    let status = ret.status().as_u16();
    let headers: serde_json::Map<String, Value> = ret
        .headers()
        .iter()
        .filter_map(|(k, v)| v.to_str().map(|v| (k.to_string(), v.to_string())).ok())
        .map(|(k, v)| (k, Value::String(v)))
        .collect();

    let body: Value = match wants {
        "json" => ret.json().unwrap_or_default(),
        "text" => ret.text().map(Value::String).unwrap_or_default(),
        _ => match ret.bytes() {
            Ok(b) => serde_json::to_value(b.to_vec()).unwrap_or_default(),
            _ => Value::default(),
        },
    };

    Ok(serde_json::json!({
        "status": status,
        "headers": headers,
        "body": body,
    }))
}

use core::time;

use msgpacker::Packable;
use reqwest::blocking::Client;
use serde_json::Value;
use valence_coprocessor::{DataBackend, Hasher, ZkVM};
use wasmtime::{Caller, Extern, Memory};

use super::Runtime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ReturnCodes {
    Success = 0,
    MemoryExport = -1,
    MemoryCapacity = -2,
    MemoryWrite = -3,
    MemoryRead = -4,
    ReturnBytes = -5,
    BufferTooLarge = -6,
    ProgramStorage = -7,
    StringUtf8 = -8,
    DomainProof = -9,
    Serialization = -10,
    JsonValue = -11,
    StateProof = -12,
    HttpMethod = -13,
    HttpBasicAuth = -14,
    HttpBearer = -15,
    HttpBody = -16,
    HttpHeader = -17,
    HttpClient = -18,
    HttpResponseJson = -19,
    HttpResponse = -20,
}

/// Resolves a panic.
pub fn panic<H, D, Z>(mut caller: Caller<Runtime<H, D, Z>>, ptr: u32, len: u32)
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
        let capacity = mem.size(&caller) as usize * mem.page_size(&caller) as usize;
        if len as usize <= capacity {
            let mut bytes = vec![0; len as usize];

            if mem.read(&mut caller, ptr as usize, &mut bytes).is_ok() {
                if let Ok(m) = String::from_utf8(bytes) {
                    caller.data_mut().panic.replace(m);

                    return;
                }
            }
        }
    }

    caller
        .data_mut()
        .panic
        .replace(String::from("undefined panic"));
}

/// Writes the function arguments (JSON bytes) to `ptr`.
///
/// Returns an error if the maximum `capacity` of the buffer is smaller than the arguments length.
pub fn args<H, D, Z>(mut caller: Caller<Runtime<H, D, Z>>, ptr: u32) -> i32
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let args = caller.data().args.to_string();

    match write_buffer(&mut caller, &mem, ptr, args.as_bytes()) {
        Ok(len) => len,
        Err(e) => e,
    }
}

/// Reads the function return (JSON bytes) from `ptr`.
pub fn ret<H, D, Z>(mut caller: Caller<Runtime<H, D, Z>>, ptr: u32, len: u32) -> i32
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let bytes = match read_buffer(&mut caller, &mem, ptr, len) {
        Ok(b) => b,
        Err(e) => return e,
    };

    match serde_json::from_slice(&bytes) {
        Ok(v) => caller.data_mut().ret.replace(v),
        Err(_) => return ReturnCodes::ReturnBytes as i32,
    };

    ReturnCodes::Success as i32
}

/// Writes the program storage to `ptr`.
///
/// Returns an error if the maximum `capacity` of the buffer is smaller than the program storage
/// length.
pub fn get_program_storage<H, D, Z>(mut caller: Caller<Runtime<H, D, Z>>, ptr: u32) -> i32
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let bytes = match caller.data().ctx.get_program_storage() {
        Ok(s) => s.unwrap_or_default(),
        Err(_) => return ReturnCodes::ProgramStorage as i32,
    };

    match write_buffer(&mut caller, &mem, ptr, &bytes) {
        Ok(len) => len,
        Err(e) => e,
    }
}

/// Replace the program storage.
pub fn set_program_storage<H, D, Z>(mut caller: Caller<Runtime<H, D, Z>>, ptr: u32, len: u32) -> i32
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let bytes = match read_buffer(&mut caller, &mem, ptr, len) {
        Ok(b) => b,
        Err(e) => return e,
    };

    if caller.data_mut().ctx.set_program_storage(&bytes).is_err() {
        return ReturnCodes::ProgramStorage as i32;
    }

    ReturnCodes::Success as i32
}

/// Get the program identifier.
pub fn get_program<H, D, Z>(mut caller: Caller<Runtime<H, D, Z>>, ptr: u32) -> i32
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let program = *caller.data().ctx.program();

    match write_buffer(&mut caller, &mem, ptr, &program) {
        Ok(len) => len,
        Err(e) => e,
    }
}

/// Get a domain proof.
pub fn get_domain_proof<H, D, Z>(
    mut caller: Caller<Runtime<H, D, Z>>,
    domain_ptr: u32,
    domain_len: u32,
    ptr: u32,
) -> i32
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let domain = match read_string(&mut caller, &mem, domain_ptr, domain_len) {
        Ok(d) => d,
        Err(e) => return e,
    };

    let opening = match caller.data().ctx.get_domain_proof(&domain) {
        Ok(o) => o,
        Err(_) => return ReturnCodes::DomainProof as i32,
    };

    match serialize(&mut caller, &mem, ptr, &opening) {
        Ok(len) => len,
        Err(e) => e,
    }
}

/// Get a state proof.
pub fn get_state_proof<H, D, Z>(
    mut caller: Caller<Runtime<H, D, Z>>,
    domain_ptr: u32,
    domain_len: u32,
    args_ptr: u32,
    args_len: u32,
    ptr: u32,
) -> i32
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let domain = match read_string(&mut caller, &mem, domain_ptr, domain_len) {
        Ok(d) => d,
        Err(e) => return e,
    };

    let args = match read_json(&mut caller, &mem, args_ptr, args_len) {
        Ok(a) => a,
        Err(e) => return e,
    };

    let proof = match caller.data().ctx.get_state_proof(&domain, args) {
        Ok(p) => p,
        Err(_) => return ReturnCodes::StateProof as i32,
    };

    match write_buffer(&mut caller, &mem, ptr, &proof) {
        Ok(len) => len,
        Err(e) => e,
    }
}

/// Perform a HTTP request.
pub fn http<H, D, Z>(
    mut caller: Caller<Runtime<H, D, Z>>,
    args_ptr: u32,
    args_len: u32,
    ptr: u32,
) -> i32
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let args = match read_json(&mut caller, &mem, args_ptr, args_len) {
        Ok(a) => a,
        Err(e) => return e,
    };

    let url = match args.get("url").and_then(Value::as_str) {
        Some(u) => u,
        None => return ReturnCodes::HttpMethod as i32,
    };

    let method = match args.get("method").and_then(Value::as_str) {
        Some(m) => m.to_lowercase(),
        None => return ReturnCodes::HttpMethod as i32,
    };

    let mut client = match method.as_str() {
        "delete" => Client::new().delete(url),
        "get" => Client::new().get(url),
        "head" => Client::new().head(url),
        "patch" => Client::new().patch(url),
        "post" => Client::new().post(url),
        "put" => Client::new().put(url),
        _ => return ReturnCodes::HttpMethod as i32,
    };

    client = client.timeout(time::Duration::from_secs(5));

    if let Some(a) = args.get("basic_auth") {
        let username = match a.get("username").and_then(Value::as_str) {
            Some(u) => u,
            None => return ReturnCodes::HttpBasicAuth as i32,
        };

        let password = match a.get("password") {
            Some(Value::String(p)) => Some(p),
            None => None,
            _ => return ReturnCodes::HttpBasicAuth as i32,
        };

        client = client.basic_auth(username, password);
    }

    match args.get("bearer") {
        Some(Value::String(b)) => client = client.bearer_auth(b),
        None => (),
        _ => return ReturnCodes::HttpBearer as i32,
    }

    match args.get("body") {
        Some(Value::String(b)) => client = client.body(b.clone()),
        Some(Value::Array(b)) => {
            let b: Vec<u8> = match b
                .iter()
                .map(|b| {
                    b.as_u64()
                        .map(|b| b as u8)
                        .ok_or(ReturnCodes::HttpBody as i32)
                })
                .collect::<Result<Vec<u8>, i32>>()
            {
                Ok(b) => b,
                Err(e) => return e,
            };

            client = client.body(b);
        }
        None => (),
        _ => return ReturnCodes::HttpBody as i32,
    }

    match args.get("headers") {
        Some(Value::Object(h)) => {
            for (k, v) in h.iter() {
                match v.as_str() {
                    Some(v) => client = client.header(k, v),
                    None => return ReturnCodes::HttpHeader as i32,
                }
            }
        }
        None => (),
        _ => return ReturnCodes::HttpHeader as i32,
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
                    None => return ReturnCodes::HttpHeader as i32,
                }
            }

            client = client.query(&q);
        }
        None => (),
        _ => return ReturnCodes::HttpHeader as i32,
    }

    let ret = match client.send() {
        Ok(r) => r,
        Err(_) => return ReturnCodes::HttpClient as i32,
    };

    let status = ret.status().as_u16();
    let headers: serde_json::Map<String, Value> = ret
        .headers()
        .iter()
        .filter_map(|(k, v)| v.to_str().map(|v| (k.to_string(), v.to_string())).ok())
        .map(|(k, v)| (k, Value::String(v)))
        .collect();

    let body: Value = match args
        .get("response")
        .and_then(Value::as_str)
        .map(str::to_lowercase)
    {
        Some(v) if v.as_str() == "json" => match ret.json() {
            Ok(j) => j,
            Err(_) => return ReturnCodes::HttpResponseJson as i32,
        },
        Some(v) if v.as_str() == "text" => match ret.text() {
            Ok(j) => Value::String(j),
            Err(_) => return ReturnCodes::HttpResponseJson as i32,
        },
        _ => match ret.bytes() {
            Ok(b) => match serde_json::to_value(b.to_vec()) {
                Ok(b) => b,
                Err(_) => return ReturnCodes::HttpResponseJson as i32,
            },
            Err(_) => return ReturnCodes::HttpResponseJson as i32,
        },
    };

    let ret = serde_json::json!({
        "status": status,
        "headers": headers,
        "body": body,
    });
    let ret = match serde_json::to_vec(&ret) {
        Ok(r) => r,
        Err(_) => return ReturnCodes::HttpResponse as i32,
    };

    match write_buffer(&mut caller, &mem, ptr, &ret) {
        Ok(len) => len,
        Err(e) => e,
    }
}

fn read_buffer<H, D, Z>(
    caller: &mut Caller<Runtime<H, D, Z>>,
    mem: &Memory,
    ptr: u32,
    len: u32,
) -> Result<Vec<u8>, i32>
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    let capacity = mem.data_size(&caller);
    let capacity = capacity.saturating_sub(ptr as usize);

    if len as usize > capacity {
        return Err(ReturnCodes::BufferTooLarge as i32);
    }

    let mut bytes = vec![0; len as usize];

    if mem.read(caller, ptr as usize, &mut bytes).is_err() {
        return Err(ReturnCodes::MemoryRead as i32);
    }

    Ok(bytes)
}

fn read_string<H, D, Z>(
    caller: &mut Caller<Runtime<H, D, Z>>,
    mem: &Memory,
    ptr: u32,
    len: u32,
) -> Result<String, i32>
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    read_buffer(caller, mem, ptr, len)
        .and_then(|b| String::from_utf8(b).map_err(|_| ReturnCodes::StringUtf8 as i32))
}

fn read_json<H, D, Z>(
    caller: &mut Caller<Runtime<H, D, Z>>,
    mem: &Memory,
    ptr: u32,
    len: u32,
) -> Result<Value, i32>
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    read_buffer(caller, mem, ptr, len)
        .and_then(|b| serde_json::from_slice(&b).map_err(|_| ReturnCodes::JsonValue as i32))
}

fn write_buffer<H, D, Z>(
    caller: &mut Caller<Runtime<H, D, Z>>,
    mem: &Memory,
    ptr: u32,
    buf: &[u8],
) -> Result<i32, i32>
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
{
    let capacity = mem.data_size(&caller);
    let capacity = capacity.saturating_sub(ptr as usize);
    if capacity < buf.len() {
        return Err(ReturnCodes::MemoryCapacity as i32);
    }

    if mem.write(caller, ptr as usize, buf).is_err() {
        return Err(ReturnCodes::MemoryWrite as i32);
    }

    Ok(buf.len() as i32)
}

fn serialize<H, D, Z, T>(
    caller: &mut Caller<Runtime<H, D, Z>>,
    mem: &Memory,
    ptr: u32,
    data: &T,
) -> Result<i32, i32>
where
    H: Hasher,
    D: DataBackend,
    Z: ZkVM,
    T: Packable,
{
    let bytes = msgpacker::pack_to_vec(data);

    write_buffer(caller, mem, ptr, &bytes)
}

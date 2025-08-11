use std::env;

use msgpacker::Packable;
use serde_json::Value;
use valence_coprocessor::{utils, DataBackend, FileSystem, Hash, Hasher, Vm, HASH_LEN};
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
    ControllerRawStorage = -7,
    StringUtf8 = -8,
    DomainProof = -9,
    Serialization = -10,
    JsonValue = -11,
    StateProof = -12,
    Http = -13,
    LatestBlock = -14,
    ControllerStorage = -15,
    HistoricalOpening = -16,
    HistoricalPayload = -17,
    AlchemyApiKey = -18,
    AlchemyResult = -19,
    BlockProof = -20,
    HistoricalUpdate = -21,
}

/// Resolves a panic.
pub fn panic<H, D, VM>(mut caller: Caller<Runtime<H, D, VM>>, ptr: u32, len: u32)
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
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
pub fn args<H, D, VM>(mut caller: Caller<Runtime<H, D, VM>>, ptr: u32) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
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
pub fn ret<H, D, VM>(mut caller: Caller<Runtime<H, D, VM>>, ptr: u32, len: u32) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
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

/// Get the [`FileSystem`] storage object.
pub fn get_storage<H, D, VM>(mut caller: Caller<Runtime<H, D, VM>>, ptr: u32) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let fs = match caller.data().ctx.get_storage() {
        Ok(s) => s,
        Err(_) => return ReturnCodes::ControllerStorage as i32,
    };

    let bytes = match fs.try_to_raw_device() {
        Ok(s) => s,
        Err(_) => return ReturnCodes::ControllerStorage as i32,
    };

    match write_buffer(&mut caller, &mem, ptr, &bytes) {
        Ok(len) => len,
        Err(e) => e,
    }
}

/// Fetch the provided file from the storage.
pub fn get_storage_file<H, D, VM>(
    mut caller: Caller<Runtime<H, D, VM>>,
    path_ptr: u32,
    path_len: u32,
    ptr: u32,
) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let path = match read_string(&mut caller, &mem, path_ptr, path_len) {
        Ok(d) => d,
        Err(e) => return e,
    };

    let file = match caller.data().ctx.get_storage_file(&path) {
        Ok(s) => s,
        Err(_) => return ReturnCodes::ControllerStorage as i32,
    };

    match serialize(&mut caller, &mem, ptr, &file) {
        Ok(len) => len,
        Err(e) => e,
    }
}

/// Override the [`FileSystem`] storage object.
pub fn set_storage<H, D, VM>(mut caller: Caller<Runtime<H, D, VM>>, ptr: u32, len: u32) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let fs = match read_buffer(&mut caller, &mem, ptr, len) {
        Ok(b) => FileSystem::from_raw_device_unchecked(b),
        Err(e) => return e,
    };

    match caller.data().ctx.set_storage(&fs) {
        Ok(_) => (),
        Err(_) => return ReturnCodes::ControllerStorage as i32,
    }

    ReturnCodes::Success as i32
}

/// Set the provided file on the storage.
pub fn set_storage_file<H, D, VM>(
    mut caller: Caller<Runtime<H, D, VM>>,
    path_ptr: u32,
    path_len: u32,
    ptr: u32,
    len: u32,
) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let path = match read_string(&mut caller, &mem, path_ptr, path_len) {
        Ok(d) => d,
        Err(e) => return e,
    };

    let contents = match read_buffer(&mut caller, &mem, ptr, len) {
        Ok(b) => b,
        Err(e) => return e,
    };

    match caller.data().ctx.set_storage_file(&path, &contents) {
        Ok(_) => (),
        Err(_) => return ReturnCodes::ControllerStorage as i32,
    }

    ReturnCodes::Success as i32
}

/// Writes the controller raw storage to `ptr`.
///
/// Returns an error if the maximum `capacity` of the buffer is smaller than the controller raw
/// storage length.
pub fn get_raw_storage<H, D, VM>(mut caller: Caller<Runtime<H, D, VM>>, ptr: u32) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let bytes = match caller.data().ctx.get_raw_storage() {
        Ok(s) => s.unwrap_or_default(),
        Err(_) => return ReturnCodes::ControllerRawStorage as i32,
    };

    match write_buffer(&mut caller, &mem, ptr, &bytes) {
        Ok(len) => len,
        Err(e) => e,
    }
}

/// Replace the controller raw storage.
pub fn set_raw_storage<H, D, VM>(mut caller: Caller<Runtime<H, D, VM>>, ptr: u32, len: u32) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let bytes = match read_buffer(&mut caller, &mem, ptr, len) {
        Ok(b) => b,
        Err(e) => return e,
    };

    if caller.data_mut().ctx.set_raw_storage(&bytes).is_err() {
        return ReturnCodes::ControllerRawStorage as i32;
    }

    ReturnCodes::Success as i32
}

/// Get the controller identifier.
pub fn get_controller<H, D, VM>(mut caller: Caller<Runtime<H, D, VM>>, ptr: u32) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let controller = *caller.data().ctx.controller();

    match write_buffer(&mut caller, &mem, ptr, &controller) {
        Ok(len) => len,
        Err(e) => e,
    }
}

/// Returns the current historical tree root.
pub(super) fn get_historical<H, D, VM>(mut caller: Caller<Runtime<H, D, VM>>, ptr: u32) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let root = caller.data().ctx.get_historical();

    match write_buffer(&mut caller, &mem, ptr, &root) {
        Ok(len) => len,
        Err(e) => e,
    }
}

/// Returns the last included block for the provided domain.
pub fn get_latest_block<H, D, VM>(
    mut caller: Caller<Runtime<H, D, VM>>,
    domain_ptr: u32,
    domain_len: u32,
    ptr: u32,
) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let domain = match read_string(&mut caller, &mem, domain_ptr, domain_len) {
        Ok(d) => d,
        Err(e) => return e,
    };

    let block = match caller.data().ctx.get_latest_block(&domain) {
        Ok(block) => block,
        Err(_) => return ReturnCodes::LatestBlock as i32,
    };

    match serialize(&mut caller, &mem, ptr, &block) {
        Ok(len) => len,
        Err(e) => e,
    }
}

pub fn get_block_proof<H, D, VM>(
    mut caller: Caller<Runtime<H, D, VM>>,
    domain_ptr: u32,
    domain_len: u32,
    block_number_ptr: u32,
    ptr: u32,
) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let domain = match read_string(&mut caller, &mem, domain_ptr, domain_len) {
        Ok(d) => d,
        Err(e) => return e,
    };

    let block_number = match read_u64(&mut caller, &mem, block_number_ptr) {
        Ok(d) => d,
        Err(e) => return e,
    };

    let proof = match caller.data().ctx.get_block_proof(&domain, block_number) {
        Ok(p) => p,
        Err(_) => return ReturnCodes::BlockProof as i32,
    };

    match serialize(&mut caller, &mem, ptr, &proof) {
        Ok(len) => len,
        Err(e) => e,
    }
}

pub fn get_historical_update<H, D, VM>(
    mut caller: Caller<Runtime<H, D, VM>>,
    root_ptr: u32,
    ptr: u32,
) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let root = match read_hash(&mut caller, &mem, root_ptr) {
        Ok(d) => d,
        Err(e) => return e,
    };

    let update = match caller.data().ctx.get_historical_update(&root) {
        Ok(u) => u,
        Err(_) => return ReturnCodes::HistoricalUpdate as i32,
    };

    match serialize(&mut caller, &mem, ptr, &update) {
        Ok(len) => len,
        Err(e) => e,
    }
}

/// Get a state proof.
pub fn get_state_proof<H, D, VM>(
    mut caller: Caller<Runtime<H, D, VM>>,
    domain_ptr: u32,
    domain_len: u32,
    args_ptr: u32,
    args_len: u32,
    ptr: u32,
) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
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

    let proof = match caller
        .data()
        .ctx
        .get_state_proof(&caller.data().vm, &domain, args)
    {
        Ok(p) => p,
        Err(_) => return ReturnCodes::StateProof as i32,
    };

    match serialize(&mut caller, &mem, ptr, &proof) {
        Ok(len) => len,
        Err(e) => e,
    }
}

/// Perform a HTTP request.
pub fn http<H, D, VM>(
    mut caller: Caller<Runtime<H, D, VM>>,
    args_ptr: u32,
    args_len: u32,
    ptr: u32,
) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let args = match read_json(&mut caller, &mem, args_ptr, args_len) {
        Ok(a) => a,
        Err(e) => return e,
    };

    let ret = match utils::http(&args) {
        Ok(r) => r,
        Err(_) => return ReturnCodes::Http as i32,
    };

    let ret = match serde_json::to_vec(&ret) {
        Ok(r) => r,
        Err(_) => return ReturnCodes::Http as i32,
    };

    match write_buffer(&mut caller, &mem, ptr, &ret) {
        Ok(len) => len,
        Err(e) => e,
    }
}

/// Perform an Alchemy API request.
#[allow(clippy::too_many_arguments)]
pub fn alchemy<H, D, VM>(
    mut caller: Caller<Runtime<H, D, VM>>,
    chain_ptr: u32,
    chain_len: u32,
    method_ptr: u32,
    method_len: u32,
    params_ptr: u32,
    params_len: u32,
    ptr: u32,
) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let key = match env::var("ALCHEMY_API_KEY") {
        Ok(k) => k,
        Err(e) => {
            tracing::warn!("alchemy key not set: {e}");
            return ReturnCodes::AlchemyApiKey as i32;
        }
    };

    let chain = match read_string(&mut caller, &mem, chain_ptr, chain_len) {
        Ok(a) => a,
        Err(e) => return e,
    };

    let method = match read_string(&mut caller, &mem, method_ptr, method_len) {
        Ok(a) => a,
        Err(e) => return e,
    };

    let params = match read_json(&mut caller, &mem, params_ptr, params_len) {
        Ok(a) => a,
        Err(e) => return e,
    };

    let url = format!("https://{chain}.g.alchemy.com/v2/{key}");
    let args = serde_json::json!({
        "url": url,
        "method": "post",
        "headers": {
            "Content-Type": "application/json"
        },
        "json": {
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        }
    });

    let ret = match utils::http(&args) {
        Ok(r) => r,
        Err(_) => return ReturnCodes::AlchemyResult as i32,
    };

    let ret = match ret.get("body").cloned() {
        Some(r) => r,
        None => return ReturnCodes::AlchemyResult as i32,
    };

    let ret: Vec<u8> = match serde_json::from_value(ret) {
        Ok(r) => r,
        Err(_) => return ReturnCodes::AlchemyResult as i32,
    };

    let ret: Value = match serde_json::from_slice(&ret) {
        Ok(r) => r,
        Err(_) => return ReturnCodes::AlchemyResult as i32,
    };

    let ret = match ret.get("result") {
        Some(r) => r,
        None => return ReturnCodes::AlchemyResult as i32,
    };

    let ret = match serde_json::to_vec(&ret) {
        Ok(r) => r,
        Err(_) => return ReturnCodes::Http as i32,
    };

    match write_buffer(&mut caller, &mem, ptr, &ret) {
        Ok(len) => len,
        Err(e) => e,
    }
}

/// Logs a string.
pub fn log<H, D, VM>(mut caller: Caller<Runtime<H, D, VM>>, ptr: u32, len: u32) -> i32
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let log = match read_string(&mut caller, &mem, ptr, len) {
        Ok(d) => d,
        Err(e) => return e,
    };

    tracing::debug!("controller log: {log}");

    caller.data_mut().log.push(log);

    ReturnCodes::Success as i32
}

fn read_buffer<H, D, VM>(
    caller: &mut Caller<Runtime<H, D, VM>>,
    mem: &Memory,
    ptr: u32,
    len: u32,
) -> Result<Vec<u8>, i32>
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
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

fn read_hash<H, D, VM>(
    caller: &mut Caller<Runtime<H, D, VM>>,
    mem: &Memory,
    ptr: u32,
) -> Result<Hash, i32>
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    read_buffer(caller, mem, ptr, HASH_LEN as u32).and_then(|buffer| {
        Hash::try_from(buffer.as_slice()).map_err(|_| ReturnCodes::BufferTooLarge as i32)
    })
}

fn read_u64<H, D, VM>(
    caller: &mut Caller<Runtime<H, D, VM>>,
    mem: &Memory,
    ptr: u32,
) -> Result<u64, i32>
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    read_buffer(caller, mem, ptr, 8).and_then(|buffer| {
        <[u8; 8]>::try_from(buffer.as_slice())
            .map(u64::from_le_bytes)
            .map_err(|_| ReturnCodes::BufferTooLarge as i32)
    })
}

fn read_string<H, D, VM>(
    caller: &mut Caller<Runtime<H, D, VM>>,
    mem: &Memory,
    ptr: u32,
    len: u32,
) -> Result<String, i32>
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    read_buffer(caller, mem, ptr, len)
        .and_then(|b| String::from_utf8(b).map_err(|_| ReturnCodes::StringUtf8 as i32))
}

fn read_json<H, D, VM>(
    caller: &mut Caller<Runtime<H, D, VM>>,
    mem: &Memory,
    ptr: u32,
    len: u32,
) -> Result<Value, i32>
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
{
    read_buffer(caller, mem, ptr, len)
        .and_then(|b| serde_json::from_slice(&b).map_err(|_| ReturnCodes::JsonValue as i32))
}

fn write_buffer<H, D, VM>(
    caller: &mut Caller<Runtime<H, D, VM>>,
    mem: &Memory,
    ptr: u32,
    buf: &[u8],
) -> Result<i32, i32>
where
    H: Hasher,
    D: DataBackend,
    VM: Vm<H, D>,
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

fn serialize<H, D, VM, T>(
    caller: &mut Caller<Runtime<H, D, VM>>,
    mem: &Memory,
    ptr: u32,
    data: &T,
) -> Result<i32, i32>
where
    H: Hasher,
    D: DataBackend,
    T: Packable,
    VM: Vm<H, D>,
{
    let bytes = msgpacker::pack_to_vec(data);

    write_buffer(caller, mem, ptr, &bytes)
}

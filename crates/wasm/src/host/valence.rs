use msgpacker::Packable;
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

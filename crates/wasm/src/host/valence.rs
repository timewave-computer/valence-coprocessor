use wasmtime::{Caller, Extern};

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
}

/// Resolves a panic.
pub fn panic(mut caller: Caller<'_, Runtime>, ptr: u32, len: u32) {
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
pub fn args(mut caller: Caller<'_, Runtime>, ptr: u32) -> i32 {
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let args = caller.data().args.to_string();
    let bytes = args.as_bytes();
    let len = bytes.len() as i32;

    let capacity = mem.data_size(&caller);
    let capacity = capacity.saturating_sub(ptr as usize);

    if capacity < bytes.len() {
        return ReturnCodes::MemoryCapacity as i32;
    }

    if mem.write(&mut caller, ptr as usize, bytes).is_err() {
        return ReturnCodes::MemoryWrite as i32;
    }

    len
}

/// Reads the function return (JSON bytes) from `ptr`.
pub fn ret(mut caller: Caller<'_, Runtime>, ptr: u32, len: u32) -> i32 {
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let capacity = mem.data_size(&caller);
    let capacity = capacity.saturating_sub(ptr as usize);

    if len as usize > capacity {
        return ReturnCodes::BufferTooLarge as i32;
    }

    let mut bytes = vec![0; len as usize];

    if mem.read(&mut caller, ptr as usize, &mut bytes).is_err() {
        return ReturnCodes::MemoryRead as i32;
    }

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
pub fn get_program_storage(mut caller: Caller<'_, Runtime>, ptr: u32) -> i32 {
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let bytes = caller.data().storage.as_deref().unwrap_or(&[]);
    let len = bytes.len() as i32;

    let capacity = mem.data_size(&caller);
    let capacity = capacity.saturating_sub(ptr as usize);

    if capacity < bytes.len() {
        return ReturnCodes::MemoryCapacity as i32;
    }

    let bytes = bytes.to_vec();

    if mem.write(&mut caller, ptr as usize, &bytes).is_err() {
        return ReturnCodes::MemoryWrite as i32;
    }

    len
}

/// Replace the program storage.
pub fn set_program_storage(mut caller: Caller<'_, Runtime>, ptr: u32, len: u32) -> i32 {
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => return ReturnCodes::MemoryExport as i32,
    };

    let capacity = mem.data_size(&caller);
    let capacity = capacity.saturating_sub(ptr as usize);

    if len as usize > capacity {
        return ReturnCodes::BufferTooLarge as i32;
    }

    let mut bytes = vec![0; len as usize];

    if mem.read(&mut caller, ptr as usize, &mut bytes).is_err() {
        return ReturnCodes::MemoryRead as i32;
    }

    caller.data_mut().storage.replace(bytes);

    ReturnCodes::Success as i32
}

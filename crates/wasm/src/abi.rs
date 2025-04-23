#![allow(static_mut_refs)]

use alloc::vec::Vec;

use serde_json::Value;

mod host {
    #[link(wasm_import_module = "valence")]
    extern "C" {
        pub(super) fn args(ptr: u32) -> i32;
        pub(super) fn ret(ptr: u32, len: u32) -> i32;
        pub(super) fn get_program_storage(ptr: u32) -> i32;
        pub(super) fn set_program_storage(ptr: u32, len: u32) -> i32;
    }
}

pub const BUF_LEN: usize = 8192;

static mut BUF: &mut [u8] = &mut [0u8; BUF_LEN];

/// Fetch the arguments from the host.
pub fn args() -> anyhow::Result<Value> {
    unsafe {
        let ptr = BUF.as_ptr() as u32;
        let len = host::args(ptr) as usize;

        anyhow::ensure!(0 < len, "failed to fetch arguments");
        anyhow::ensure!(len <= BUF_LEN, "arguments too large");

        Ok(serde_json::from_slice(&BUF[..len])?)
    }
}

/// Set the return value to the host.
pub fn ret(value: &Value) -> anyhow::Result<()> {
    let value = serde_json::to_string(value)?;
    let len = value.len();

    anyhow::ensure!(len <= BUF_LEN, "return value too large");

    unsafe {
        BUF[..len].copy_from_slice(value.as_bytes());

        let ptr = BUF.as_ptr() as u32;
        let r = host::ret(ptr, len as u32);

        anyhow::ensure!(r >= 0, "failed to write return value");
    }

    Ok(())
}

/// Fetch the program storage.
pub fn get_program_storage() -> anyhow::Result<Vec<u8>> {
    unsafe {
        let ptr = BUF.as_ptr() as u32;
        let len = host::get_program_storage(ptr) as usize;

        anyhow::ensure!(len <= BUF_LEN, "program storage too large");

        Ok(BUF[..len].to_vec())
    }
}

/// Replace the program storage.
pub fn set_program_storage(storage: &[u8]) -> anyhow::Result<()> {
    let len = storage.len();

    anyhow::ensure!(len <= BUF_LEN, "storage value too large");

    unsafe {
        BUF[..len].copy_from_slice(storage);

        let ptr = BUF.as_ptr() as u32;
        let r = host::set_program_storage(ptr, len as u32);

        anyhow::ensure!(r >= 0, "failed to write program storage");
    }

    Ok(())
}

#[cfg(feature = "abi-handlers")]
mod handlers {
    use core::{cmp, panic::PanicInfo};

    #[global_allocator]
    static ALLOC: dlmalloc::GlobalDlmalloc = dlmalloc::GlobalDlmalloc;

    #[link(wasm_import_module = "valence")]
    extern "C" {
        fn panic(ptr: u32, len: u32);
    }

    #[panic_handler]
    unsafe fn handle_panic(info: &PanicInfo) -> ! {
        let msg = info.message().as_str().unwrap_or("invalid panic message");
        let len = cmp::min(super::BUF_LEN, msg.len());

        super::BUF[..len].copy_from_slice(msg.as_bytes());

        let ptr = super::BUF.as_ptr() as u32;
        let len = len as u32;

        panic(ptr, len);

        unreachable!()
    }
}

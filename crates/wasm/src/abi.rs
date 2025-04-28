#![allow(static_mut_refs)]

use alloc::vec::Vec;

use msgpacker::Unpackable;
use serde_json::Value;
use valence_coprocessor::{Hash, SmtOpening};

pub use alloc::format;

mod host {
    #[link(wasm_import_module = "valence")]
    extern "C" {
        pub(super) fn args(ptr: u32) -> i32;
        pub(super) fn ret(ptr: u32, len: u32) -> i32;
        pub(super) fn get_program_storage(ptr: u32) -> i32;
        pub(super) fn set_program_storage(ptr: u32, len: u32) -> i32;
        pub(super) fn get_program(ptr: u32) -> i32;
        pub(super) fn get_domain_proof(domain_ptr: u32, domain_len: u32, ptr: u32) -> i32;
        pub(super) fn get_state_proof(
            domain_ptr: u32,
            domain_len: u32,
            args_ptr: u32,
            args_len: u32,
            ptr: u32,
        ) -> i32;
        pub(super) fn http(args_ptr: u32, args_len: u32, ptr: u32) -> i32;
        pub(super) fn log(ptr: u32, len: u32) -> i32;
    }
}

pub const BUF_LEN: usize = 8192;

static mut BUF: &mut [u8] = &mut [0u8; BUF_LEN];

/// Fetch the arguments from the host.
pub fn args() -> anyhow::Result<Value> {
    unsafe {
        let ptr = BUF.as_ptr() as u32;
        let len = host::args(ptr);

        anyhow::ensure!(len >= 0, "failed to fetch args");
        anyhow::ensure!(len as usize <= BUF_LEN, "arguments too large");

        Ok(serde_json::from_slice(&BUF[..len as usize])?)
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
        let len = host::get_program_storage(ptr);

        anyhow::ensure!(len >= 0, "failed to fetch program storage");
        anyhow::ensure!(len as usize <= BUF_LEN, "program storage too large");

        Ok(BUF[..len as usize].to_vec())
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

/// Get the program identifier of the current context.
pub fn get_program() -> anyhow::Result<Hash> {
    unsafe {
        let ptr = BUF.as_ptr() as u32;
        let len = host::get_program(ptr);

        anyhow::ensure!(len >= 0, "failed to read program id");
        anyhow::ensure!(len as usize <= BUF_LEN, "program id too large");

        Ok(Hash::try_from(&BUF[..len as usize])?)
    }
}

/// Get the program identifier of the current context.
pub fn get_domain_proof(domain: &str) -> anyhow::Result<Option<SmtOpening>> {
    unsafe {
        let domain_ptr = domain.as_ptr() as u32;
        let domain_len = domain.len() as u32;
        let ptr = BUF.as_ptr() as u32;

        let len = host::get_domain_proof(domain_ptr, domain_len, ptr);

        anyhow::ensure!(len >= 0, "failed to read domain proof");
        anyhow::ensure!(len as usize <= BUF_LEN, "arguments too large");

        Option::unpack(&BUF[..len as usize])
            .map(|(_, o)| o)
            .map_err(|e| anyhow::anyhow!("error unpacking domain proof: {e}"))
    }
}

/// Get the program identifier of the current context.
pub fn get_state_proof(domain: &str, args: &Value) -> anyhow::Result<Vec<u8>> {
    unsafe {
        let domain_ptr = domain.as_ptr() as u32;
        let domain_len = domain.len() as u32;

        let args = serde_json::to_vec(args)?;
        let args_ptr = args.as_ptr() as u32;
        let args_len = args.len() as u32;

        let ptr = BUF.as_ptr() as u32;

        let len = host::get_state_proof(domain_ptr, domain_len, args_ptr, args_len, ptr);

        anyhow::ensure!(len >= 0, "failed to read state proof");
        anyhow::ensure!(len as usize <= BUF_LEN, "arguments too large");

        Ok(BUF[..len as usize].to_vec())
    }
}

/// Performs a HTTP request.
pub fn http(args: &Value) -> anyhow::Result<Value> {
    unsafe {
        let args = serde_json::to_vec(args)?;
        let args_ptr = args.as_ptr() as u32;
        let args_len = args.len() as u32;

        let ptr = BUF.as_ptr() as u32;

        let len = host::http(args_ptr, args_len, ptr);

        anyhow::ensure!(len >= 0, "failed to read state proof");
        anyhow::ensure!(len as usize <= BUF_LEN, "arguments too large");

        Ok(serde_json::from_slice(&BUF[..len as usize])?)
    }
}

/// Logs information to the host runtime.
pub fn log(log: &str) -> anyhow::Result<()> {
    unsafe {
        let ptr = log.as_ptr() as u32;
        let len = log.len() as u32;

        let ret = host::log(ptr, len);

        anyhow::ensure!(ret == 0, "failed to log information");

        Ok(())
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::abi::log(&$crate::abi::format!($($arg)*))
    }
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

        panic!("{msg}")
    }
}

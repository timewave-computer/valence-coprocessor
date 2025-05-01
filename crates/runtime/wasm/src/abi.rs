#![allow(static_mut_refs)]
// TODO implement std runtime
#![allow(dead_code)]

use alloc::vec::Vec;

use serde_json::Value;
use valence_coprocessor::{Hash, SmtOpening, ValidatedBlock, Witness};

pub use crate::__log as log;
pub use alloc::format;

#[cfg(not(feature = "std"))]
mod host {
    #[link(wasm_import_module = "valence")]
    extern "C" {
        pub(super) fn args(ptr: u32) -> i32;
        pub(super) fn ret(ptr: u32, len: u32) -> i32;
        pub(super) fn get_storage(ptr: u32) -> i32;
        pub(super) fn set_storage(ptr: u32, len: u32) -> i32;
        pub(super) fn get_library(ptr: u32) -> i32;
        pub(super) fn get_domain_proof(domain_ptr: u32, domain_len: u32, ptr: u32) -> i32;
        pub(super) fn get_latest_block(domain_ptr: u32, domain_len: u32, ptr: u32) -> i32;
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

#[cfg(feature = "std")]
pub(crate) mod use_std {
    use std::sync::{LazyLock, Mutex};

    use super::*;

    static RUNTIME: LazyLock<Mutex<Runtime>> = LazyLock::new(|| Mutex::new(Runtime::default()));

    /// Initializes the runtime with default values.
    pub fn initialize_default_runtime() {
        initialize_runtime(Default::default(), Default::default())
    }

    /// Initializes the runtime.
    pub fn initialize_runtime(library: Hash, storage: Vec<u8>) {
        let mut runtime = RUNTIME.lock().unwrap();

        runtime.storage = storage;
        runtime.library = library;
    }

    pub fn runtime() -> Runtime {
        RUNTIME.lock().unwrap().clone()
    }

    /// A virtual runtime.
    #[derive(Debug, Default, Clone)]
    pub struct Runtime {
        /// Execution arguments.
        pub args: Value,

        /// Computation result.
        pub ret: Value,

        /// library storage.
        pub storage: Vec<u8>,

        /// library identifier
        pub library: Hash,

        /// Execution logs.
        pub log: Vec<String>,
    }

    pub fn args() -> anyhow::Result<Value> {
        Ok(RUNTIME.lock().unwrap().args.clone())
    }

    pub fn ret(value: &Value) -> anyhow::Result<()> {
        RUNTIME.lock().unwrap().ret = value.clone();

        Ok(())
    }

    pub fn get_storage() -> anyhow::Result<Vec<u8>> {
        Ok(RUNTIME.lock().unwrap().storage.clone())
    }

    pub fn set_storage(storage: &[u8]) -> anyhow::Result<()> {
        RUNTIME.lock().unwrap().storage = storage.to_vec();

        Ok(())
    }

    pub fn get_library() -> anyhow::Result<Hash> {
        Ok(RUNTIME.lock().unwrap().library)
    }

    pub fn get_domain_proof(_domain: &str) -> anyhow::Result<Option<SmtOpening>> {
        todo!()
    }

    pub fn get_latest_block(_domain: &str) -> anyhow::Result<Option<ValidatedBlock>> {
        todo!()
    }

    pub fn get_state_proof(_domain: &str, _args: &Value) -> anyhow::Result<Vec<u8>> {
        todo!()
    }

    pub fn http(args: &Value) -> anyhow::Result<Value> {
        crate::host::valence::http_host(args)
            .map_err(|e| anyhow::anyhow!("error computing request: {e}"))
    }

    pub fn __value_to_context_log(log: &str) -> anyhow::Result<()> {
        RUNTIME.lock().unwrap().log.push(log.to_string());

        Ok(())
    }
}

#[cfg(feature = "tests-runtime")]
pub use use_std::{initialize_default_runtime, initialize_runtime, runtime};

pub const BUF_LEN: usize = 1024 * 1024;

static mut BUF: &mut [u8] = &mut [0u8; BUF_LEN];

/// Fetch the arguments from the host.
pub fn args() -> anyhow::Result<Value> {
    #[cfg(feature = "std")]
    return use_std::args();

    #[cfg(not(feature = "std"))]
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
    #[cfg(feature = "std")]
    return use_std::ret(value);

    #[cfg(not(feature = "std"))]
    unsafe {
        let value = serde_json::to_string(value)?;
        let len = value.len();

        anyhow::ensure!(len <= BUF_LEN, "return value too large");

        BUF[..len].copy_from_slice(value.as_bytes());

        let ptr = BUF.as_ptr() as u32;
        let r = host::ret(ptr, len as u32);

        anyhow::ensure!(r >= 0, "failed to write return value");

        return Ok(());
    }
}

/// Fetch the library storage.
pub fn get_storage() -> anyhow::Result<Vec<u8>> {
    #[cfg(feature = "std")]
    return use_std::get_storage();

    #[cfg(not(feature = "std"))]
    unsafe {
        let ptr = BUF.as_ptr() as u32;
        let len = host::get_storage(ptr);

        anyhow::ensure!(len >= 0, "failed to fetch library storage");
        anyhow::ensure!(len as usize <= BUF_LEN, "library storage too large");

        Ok(BUF[..len as usize].to_vec())
    }
}

/// Replace the library storage.
pub fn set_storage(storage: &[u8]) -> anyhow::Result<()> {
    #[cfg(feature = "std")]
    return use_std::set_storage(storage);

    #[cfg(not(feature = "std"))]
    unsafe {
        let len = storage.len();

        anyhow::ensure!(len <= BUF_LEN, "storage value too large");

        BUF[..len].copy_from_slice(storage);

        let ptr = BUF.as_ptr() as u32;
        let r = host::set_storage(ptr, len as u32);

        anyhow::ensure!(r >= 0, "failed to write library storage");

        return Ok(());
    }
}

/// Get the library identifier of the current context.
pub fn get_library() -> anyhow::Result<Hash> {
    #[cfg(feature = "std")]
    return use_std::get_library();

    #[cfg(not(feature = "std"))]
    unsafe {
        let ptr = BUF.as_ptr() as u32;
        let len = host::get_library(ptr);

        anyhow::ensure!(len >= 0, "failed to read library id");
        anyhow::ensure!(len as usize <= BUF_LEN, "library id too large");

        Ok(Hash::try_from(&BUF[..len as usize])?)
    }
}

/// Get the library identifier of the current context.
pub fn get_domain_proof(domain: &str) -> anyhow::Result<Option<SmtOpening>> {
    #[cfg(feature = "std")]
    return use_std::get_domain_proof(domain);

    #[cfg(not(feature = "std"))]
    unsafe {
        use msgpacker::Unpackable as _;

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

/// Returns the last included block for the provided domain.
pub fn get_latest_block(domain: &str) -> anyhow::Result<Option<ValidatedBlock>> {
    #[cfg(feature = "std")]
    return use_std::get_latest_block(domain);

    #[cfg(not(feature = "std"))]
    unsafe {
        use msgpacker::Unpackable as _;

        let domain_ptr = domain.as_ptr() as u32;
        let domain_len = domain.len() as u32;
        let ptr = BUF.as_ptr() as u32;

        let len = host::get_latest_block(domain_ptr, domain_len, ptr);

        anyhow::ensure!(len >= 0, "failed to read latest block");
        anyhow::ensure!(len as usize <= BUF_LEN, "arguments too large");

        Option::unpack(&BUF[..len as usize])
            .map(|(_, o)| o)
            .map_err(|e| anyhow::anyhow!("error unpacking latest block: {e}"))
    }
}

/// Get the library identifier of the current context.
pub fn get_state_proof(domain: &str, args: &Value) -> anyhow::Result<Vec<u8>> {
    #[cfg(feature = "std")]
    return use_std::get_state_proof(domain, args);

    #[cfg(not(feature = "std"))]
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
    #[cfg(feature = "std")]
    return use_std::http(args);

    #[cfg(not(feature = "std"))]
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

/// Returns the provided witnesses to the context.
pub fn ret_witnesses(witnesses: Vec<Witness>) -> anyhow::Result<()> {
    let witnesses = serde_json::to_value(witnesses)?;

    ret(&witnesses)
}

/// Logs a value into the context.
pub fn __value_to_context_log(log: &str) -> anyhow::Result<()> {
    #[cfg(feature = "std")]
    return use_std::__value_to_context_log(log);

    #[cfg(not(feature = "std"))]
    unsafe {
        let ptr = log.as_ptr() as u32;
        let len = log.len() as u32;

        let ret = host::log(ptr, len);

        anyhow::ensure!(ret == 0, "failed to log information");

        Ok(())
    }
}

#[macro_export]
macro_rules! __log {
    ($($arg:tt)*) => {
        $crate::abi::__value_to_context_log(&$crate::abi::format!($($arg)*))
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

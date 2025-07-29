#![allow(static_mut_refs)]
#![allow(dead_code)]

use alloc::vec::Vec;

use serde_json::Value;
use valence_coprocessor::{FileSystem, Hash, Opening, StateProof, ValidatedDomainBlock, Witness};

#[cfg(not(feature = "std"))]
use msgpacker::Unpackable as _;

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
        pub(super) fn get_storage_file(path_ptr: u32, path_len: u32, ptr: u32) -> i32;
        pub(super) fn set_storage_file(path_ptr: u32, path_len: u32, ptr: u32, len: u32) -> i32;
        pub(super) fn get_raw_storage(ptr: u32) -> i32;
        pub(super) fn set_raw_storage(ptr: u32, len: u32) -> i32;
        pub(super) fn get_controller(ptr: u32) -> i32;
        pub(super) fn get_historical(ptr: u32) -> i32;
        pub(super) fn get_historical_opening(
            tree_ptr: u32,
            domain_ptr: u32,
            domain_len: u32,
            root_ptr: u32,
            root_len: u32,
            ptr: u32,
        ) -> i32;
        pub(super) fn get_historical_payload(
            domain_ptr: u32,
            domain_len: u32,
            root_ptr: u32,
            root_len: u32,
            ptr: u32,
        ) -> i32;
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
        pub(super) fn alchemy(
            chain_ptr: u32,
            chain_len: u32,
            method_ptr: u32,
            method_len: u32,
            params_ptr: u32,
            params_len: u32,
            ptr: u32,
        ) -> i32;
    }
}

#[cfg(feature = "std")]
pub(crate) mod use_std {
    use std::sync::{LazyLock, Mutex};

    use valence_coprocessor::{File, Opening, StateProof};

    use super::*;

    static RUNTIME: LazyLock<Mutex<Runtime>> = LazyLock::new(|| Mutex::new(Runtime::default()));

    /// Initializes the runtime with default values.
    pub fn initialize_default_runtime() {
        initialize_runtime(Default::default(), Default::default())
    }

    /// Initializes the runtime.
    pub fn initialize_runtime(controller: Hash, raw_storage: Vec<u8>) {
        let mut runtime = RUNTIME.lock().unwrap();

        runtime.raw_storage = raw_storage;
        runtime.controller = controller;
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

        /// controller raw storage.
        pub raw_storage: Vec<u8>,

        /// controller identifier
        pub controller: Hash,

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

    pub fn get_storage() -> anyhow::Result<FileSystem> {
        get_raw_storage().map(FileSystem::from_raw_device_unchecked)
    }

    pub fn set_storage(fs: &FileSystem) -> anyhow::Result<()> {
        set_raw_storage(&fs.try_to_raw_device()?)
    }

    pub fn get_storage_file(path: &str) -> anyhow::Result<Vec<u8>> {
        Ok(get_storage()?.open(path)?.contents)
    }

    pub fn set_storage_file(path: &str, contents: &[u8]) -> anyhow::Result<()> {
        let mut fs = get_storage()?;

        fs.save(File::new(path.into(), contents.to_vec(), true))?;

        set_raw_storage(&fs.try_to_raw_device()?)
    }

    pub fn get_raw_storage() -> anyhow::Result<Vec<u8>> {
        Ok(RUNTIME.lock().unwrap().raw_storage.clone())
    }

    pub fn set_raw_storage(raw_storage: &[u8]) -> anyhow::Result<()> {
        RUNTIME.lock().unwrap().raw_storage = raw_storage.to_vec();

        Ok(())
    }

    pub fn get_controller() -> anyhow::Result<Hash> {
        Ok(RUNTIME.lock().unwrap().controller)
    }

    pub fn get_historical() -> anyhow::Result<Hash> {
        todo!()
    }

    pub fn get_historical_opening(
        _tree: &Hash,
        _domain: &str,
        _root: &[u8],
    ) -> anyhow::Result<Option<Opening>> {
        todo!()
    }

    pub fn get_historical_payload(_domain: &str, _root: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        todo!()
    }

    pub fn get_latest_block(_domain: &str) -> anyhow::Result<Option<ValidatedDomainBlock>> {
        todo!()
    }

    pub fn get_state_proof(_domain: &str, _args: &Value) -> anyhow::Result<StateProof> {
        todo!()
    }

    pub fn http(args: &Value) -> anyhow::Result<Value> {
        valence_coprocessor::utils::http(args)
    }

    pub fn alchemy(_chain: &str, _method: &str, _params: &Value) -> anyhow::Result<Value> {
        todo!()
    }

    pub fn __value_to_context_log(log: &str) -> anyhow::Result<()> {
        RUNTIME.lock().unwrap().log.push(log.to_string());

        Ok(())
    }
}

#[cfg(feature = "tests-runtime")]
pub use use_std::{initialize_default_runtime, initialize_runtime, runtime};

pub const BUF_LEN: usize = 16 * 1024 * 1024;

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

pub fn get_storage() -> anyhow::Result<FileSystem> {
    #[cfg(feature = "std")]
    return use_std::get_storage();

    #[cfg(not(feature = "std"))]
    unsafe {
        let ptr = BUF.as_ptr() as u32;
        let len = host::get_storage(ptr);

        anyhow::ensure!(len >= 0, "failed to fetch controller storage");
        anyhow::ensure!(len as usize <= BUF_LEN, "controller storage too large");

        let fs = FileSystem::from_raw_device_unchecked(BUF[..len as usize].to_vec());

        Ok(fs)
    }
}

pub fn set_storage(fs: &FileSystem) -> anyhow::Result<()> {
    #[cfg(feature = "std")]
    return use_std::set_storage(fs);

    #[cfg(not(feature = "std"))]
    unsafe {
        let bytes = fs.try_to_raw_device()?;
        let ptr = bytes.as_ptr() as u32;
        let len = bytes.len() as u32;

        let r = host::set_storage(ptr, len);

        anyhow::ensure!(r >= 0, "failed to write controller storage");

        return Ok(());
    }
}

pub fn get_storage_file(path: &str) -> anyhow::Result<Vec<u8>> {
    #[cfg(feature = "std")]
    return use_std::get_storage_file(path);

    #[cfg(not(feature = "std"))]
    unsafe {
        let path_ptr = path.as_ptr() as u32;
        let path_len = path.len() as u32;
        let ptr = BUF.as_ptr() as u32;

        let len = host::get_storage_file(path_ptr, path_len, ptr);

        anyhow::ensure!(len >= 0, "failed to fetch controller storage file");
        anyhow::ensure!(len as usize <= BUF_LEN, "controller storage file too large");

        Ok(BUF[..len as usize].to_vec())
    }
}

pub fn set_storage_file(path: &str, contents: &[u8]) -> anyhow::Result<()> {
    #[cfg(feature = "std")]
    return use_std::set_storage_file(path, contents);

    #[cfg(not(feature = "std"))]
    unsafe {
        let path_ptr = path.as_ptr() as u32;
        let path_len = path.len() as u32;
        let ptr = contents.as_ptr() as u32;
        let len = contents.len() as u32;

        let r = host::set_storage_file(path_ptr, path_len, ptr, len);

        anyhow::ensure!(r >= 0, "failed to write controller storage file");

        Ok(())
    }
}

/// Fetch the controller raw storage.
pub fn get_raw_storage() -> anyhow::Result<Vec<u8>> {
    #[cfg(feature = "std")]
    return use_std::get_raw_storage();

    #[cfg(not(feature = "std"))]
    unsafe {
        let ptr = BUF.as_ptr() as u32;
        let len = host::get_raw_storage(ptr);

        anyhow::ensure!(len >= 0, "failed to fetch controller raw storage");
        anyhow::ensure!(len as usize <= BUF_LEN, "controller raw storage too large");

        Ok(BUF[..len as usize].to_vec())
    }
}

/// Replace the controller raw storage.
pub fn set_raw_storage(raw_storage: &[u8]) -> anyhow::Result<()> {
    #[cfg(feature = "std")]
    return use_std::set_raw_storage(raw_storage);

    #[cfg(not(feature = "std"))]
    unsafe {
        let ptr = raw_storage.as_ptr() as u32;
        let len = raw_storage.len();

        let r = host::set_raw_storage(ptr, len as u32);

        anyhow::ensure!(r >= 0, "failed to write controller raw storage");

        return Ok(());
    }
}

/// Get the controller identifier of the current context.
pub fn get_controller() -> anyhow::Result<Hash> {
    #[cfg(feature = "std")]
    return use_std::get_controller();

    #[cfg(not(feature = "std"))]
    unsafe {
        let ptr = BUF.as_ptr() as u32;
        let len = host::get_controller(ptr);

        anyhow::ensure!(len >= 0, "failed to read controller id");
        anyhow::ensure!(len as usize <= BUF_LEN, "controller id too large");

        Ok(Hash::try_from(&BUF[..len as usize])?)
    }
}

/// Get the opening to the provided root on the historical SMT.
pub fn get_historical() -> anyhow::Result<Hash> {
    #[cfg(feature = "std")]
    return use_std::get_historical();

    #[cfg(not(feature = "std"))]
    unsafe {
        let ptr = BUF.as_ptr() as u32;
        let len = host::get_historical(ptr);

        anyhow::ensure!(len >= 0, "failed to read historical root");

        Ok(Hash::try_from(&BUF[..len as usize])?)
    }
}

/// Get the opening to the provided root on the historical SMT.
pub fn get_historical_opening(
    tree: &Hash,
    domain: &str,
    root: &[u8],
) -> anyhow::Result<Option<Opening>> {
    #[cfg(feature = "std")]
    return use_std::get_historical_opening(tree, domain, root);

    #[cfg(not(feature = "std"))]
    unsafe {
        let tree_ptr = tree.as_ptr() as u32;

        let domain_ptr = domain.as_ptr() as u32;
        let domain_len = domain.len() as u32;

        let root_ptr = root.as_ptr() as u32;
        let root_len = root.len() as u32;

        let ptr = BUF.as_ptr() as u32;
        let len =
            host::get_historical_opening(tree_ptr, domain_ptr, domain_len, root_ptr, root_len, ptr);

        anyhow::ensure!(len >= 0, "failed to read historical opening");
        anyhow::ensure!(len as usize <= BUF_LEN, "arguments too large");

        Option::unpack(&BUF[..len as usize])
            .map(|(_, o)| o)
            .map_err(|e| anyhow::anyhow!("error unpacking historical opening: {e}"))
    }
}

/// Get the payload of the provided domain root on the historical SMT.
pub fn get_historical_payload(domain: &str, root: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
    #[cfg(feature = "std")]
    return use_std::get_historical_payload(domain, root);

    #[cfg(not(feature = "std"))]
    unsafe {
        let domain_ptr = domain.as_ptr() as u32;
        let domain_len = domain.len() as u32;

        let root_ptr = root.as_ptr() as u32;
        let root_len = root.len() as u32;

        let ptr = BUF.as_ptr() as u32;
        let len = host::get_historical_payload(domain_ptr, domain_len, root_ptr, root_len, ptr);

        anyhow::ensure!(len >= 0, "failed to read historical payload");
        anyhow::ensure!(len as usize <= BUF_LEN, "arguments too large");

        Option::unpack(&BUF[..len as usize])
            .map(|(_, o)| o)
            .map_err(|e| anyhow::anyhow!("error unpacking historical payload: {e}"))
    }
}

/// Returns the last included block for the provided domain.
pub fn get_latest_block(domain: &str) -> anyhow::Result<Option<ValidatedDomainBlock>> {
    #[cfg(feature = "std")]
    return use_std::get_latest_block(domain);

    #[cfg(not(feature = "std"))]
    unsafe {
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

/// Get the controller identifier of the current context.
pub fn get_state_proof(domain: &str, args: &Value) -> anyhow::Result<StateProof> {
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

        StateProof::unpack(&BUF[..len as usize])
            .map(|(_, o)| o)
            .map_err(|e| anyhow::anyhow!("error unpacking state proof: {e}"))
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

        anyhow::ensure!(len >= 0, "failed to read http response");
        anyhow::ensure!(len as usize <= BUF_LEN, "arguments too large");

        Ok(serde_json::from_slice(&BUF[..len as usize])?)
    }
}

/// Performs an Alchemy API request.
///
/// # Params
///
/// - `chain`: the chain to be called on alchemy (ex: `eth-mainnet`)
/// - `method`: the method to be called for the node (ex: `eth_getProof`)
/// - `params`: the parameters to be passed to the request.
pub fn alchemy(chain: &str, method: &str, params: &Value) -> anyhow::Result<Value> {
    #[cfg(feature = "std")]
    return use_std::alchemy(chain, method, params);

    #[cfg(not(feature = "std"))]
    unsafe {
        let chain_ptr = chain.as_ptr() as u32;
        let chain_len = chain.len() as u32;

        let method_ptr = method.as_ptr() as u32;
        let method_len = method.len() as u32;

        let params = serde_json::to_vec(params)?;
        let params_ptr = params.as_ptr() as u32;
        let params_len = params.len() as u32;

        let ptr = BUF.as_ptr() as u32;

        let len = host::alchemy(
            chain_ptr, chain_len, method_ptr, method_len, params_ptr, params_len, ptr,
        );

        anyhow::ensure!(len >= 0, "failed to read alchemy result");
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

        core::arch::wasm32::unreachable();
    }
}

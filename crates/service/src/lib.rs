use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use valence_coprocessor::ExecutionContext;
use valence_coprocessor_rocksdb::RocksBackend;
use valence_coprocessor_sp1::{Sp1Hasher, Sp1ZkVM};
use valence_coprocessor_wasm::host::ValenceWasm;

pub type Context = ExecutionContext<
    Sp1Hasher,
    RocksBackend,
    ValenceWasm<Sp1Hasher, RocksBackend, Sp1ZkVM>,
    Sp1ZkVM,
>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub data_dir: PathBuf,
    pub socket: String,
}

impl Config {
    pub fn create_or_read_default() -> anyhow::Result<(PathBuf, Self)> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("failed to compute config dir"))?
            .join(env!("CARGO_PKG_NAME"));

        let config = config_dir.join("config.toml");
        if config.exists() && !config.is_file() {
            anyhow::bail!(
                "the provided config path `{}` is not a valid path",
                config.display()
            );
        } else if config.is_file() {
            return Self::from_path(&config);
        }

        let data_dir = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("failed to compute data dir"))?
            .join(env!("CARGO_PKG_NAME"));

        let create_dir = &[&config_dir, &data_dir];

        for c in create_dir {
            anyhow::ensure!(!c.exists() || c.is_dir());

            if c.exists() && c.is_dir() {
            } else if c.exists() && !c.is_dir() {
                anyhow::bail!(
                    "the provided config path `{}` is not a valid directory",
                    c.display()
                );
            } else {
                fs::create_dir_all(c)?;
            }
        }

        let socket = "0.0.0.0:37281".to_string();

        let slf = Self { data_dir, socket };

        fs::write(&config, toml::to_string(&slf)?)?;

        Ok((config, slf))
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<(PathBuf, Self)> {
        let toml_str = fs::read_to_string(path.as_ref())?;

        Ok((path.as_ref().to_path_buf(), toml::from_str(&toml_str)?))
    }
}

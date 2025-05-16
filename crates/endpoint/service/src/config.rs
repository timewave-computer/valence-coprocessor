use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub redis: Option<String>,
    pub socket: String,
    pub module_cache_capacity: usize,
    pub zkvm_mode: String,
    pub zkvm_cache_capacity: usize,
}

impl Config {
    pub fn create_or_read_default() -> anyhow::Result<(PathBuf, Self)> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("failed to compute config dir"))?
            .join(env!("CARGO_PKG_NAME"));

        fs::create_dir_all(&config_dir).ok();

        let config = config_dir.join("config.toml");
        if config.exists() && !config.is_file() {
            anyhow::bail!(
                "the provided config path `{}` is not a valid path",
                config.display()
            );
        } else if config.is_file() {
            return Self::from_path(&config);
        }

        let redis = None;
        let socket = "0.0.0.0:37281".to_string();
        let module_cache_capacity = 100;
        let zkvm_mode = String::from("mock");
        let zkvm_cache_capacity = 10;

        let slf = Self {
            redis,
            socket,
            module_cache_capacity,
            zkvm_mode,
            zkvm_cache_capacity,
        };

        fs::write(&config, toml::to_string(&slf)?)?;

        Ok((config, slf))
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<(PathBuf, Self)> {
        let toml_str = fs::read_to_string(path.as_ref())?;

        Ok((path.as_ref().to_path_buf(), toml::from_str(&toml_str)?))
    }
}

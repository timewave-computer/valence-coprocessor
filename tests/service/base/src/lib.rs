use std::{
    fs,
    path::PathBuf,
    process::{Child, Command},
    thread,
    time::Duration,
};

use base64::{engine::general_purpose::STANDARD as Base64, Engine as _};
use serde_json::Value;
use valence_coprocessor::Hash;

pub struct Tester {
    pub ws: PathBuf,
    pub project: PathBuf,
    pub uri: String,
    pub service: Child,
}

impl Tester {
    pub fn retry_until<T, E, F: Fn() -> Result<T, E>>(
        frequency_ms: u64,
        mut attempts: usize,
        f: F,
    ) -> anyhow::Result<T> {
        let frequency = Duration::from_millis(frequency_ms);

        while attempts > 0 {
            thread::sleep(frequency);

            if let Ok(t) = f() {
                return Ok(t);
            }

            attempts -= 1;
        }

        anyhow::bail!("failed to produce result");
    }

    pub fn build_wasm(&self, project: &str) -> anyhow::Result<Vec<u8>> {
        assert!(Command::new("cargo")
            .current_dir(&self.project)
            .args([
                "build",
                "-p",
                project,
                "--target",
                "wasm32-unknown-unknown",
                "--release",
                "--no-default-features",
            ])
            .status()
            .unwrap()
            .success());

        let mut target = project.replace("-", "_");

        target.push_str(".wasm");

        let target = self
            .project
            .join("target")
            .join("wasm32-unknown-unknown")
            .join("release")
            .join(target)
            .canonicalize()?;

        Ok(fs::read(target)?)
    }

    pub fn build_circuit(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        let base = self.project.join(path);

        assert!(Command::new("cargo")
            .current_dir(base.join("program"))
            .args(["prove", "build",])
            .status()
            .unwrap()
            .success());

        assert!(Command::new("cargo")
            .current_dir(base.join("script"))
            .args(["run"])
            .status()
            .unwrap()
            .success());

        Ok(fs::read(base.join("target").join("program.elf"))?)
    }

    pub fn get<P: AsRef<str>>(&self, path: P) -> anyhow::Result<Value> {
        let response = reqwest::blocking::get(format!("{}{}", self.uri, path.as_ref()))?;

        Ok(response.json()?)
    }

    pub fn post<P: AsRef<str>>(&self, path: P, args: Value) -> anyhow::Result<Value> {
        let response = reqwest::blocking::Client::new()
            .post(format!("{}{}", self.uri, path.as_ref()))
            .json(&args)
            .send()?;

        Ok(response.json()?)
    }

    pub fn hex_to_hash(&self, h: &Value) -> anyhow::Result<Hash> {
        let h = h
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("value is not string"))?;

        let bytes =
            hex::decode(h).map_err(|e| anyhow::anyhow!("could not convert from hex: {e}"))?;

        Ok(Hash::try_from(bytes.as_slice())?)
    }

    pub fn b64_to_hash(&self, b64: &Value) -> anyhow::Result<Hash> {
        let b64 = b64
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("value is not string"))?;
        let bytes = Base64.decode(b64)?;

        Ok(Hash::try_from(bytes.as_slice())?)
    }
}

impl Default for Tester {
    fn default() -> Self {
        let uri = String::from("http://127.0.0.1:37281/api");
        let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .canonicalize()
            .unwrap();

        let ws = project.join("..").canonicalize().unwrap();

        assert!(Command::new("cargo")
            .current_dir(&ws)
            .args(["build", "-p", "valence-coprocessor-service", "--release",])
            .status()
            .unwrap()
            .success());

        let service = Command::new("cargo")
            .current_dir(&ws)
            .args(["run", "-p", "valence-coprocessor-service", "--release"])
            .spawn()
            .unwrap();

        let u = uri.clone();
        Self::retry_until(2000, 10, move || {
            reqwest::blocking::get(format!("{u}/status"))
        })
        .unwrap();

        Self {
            ws,
            project,
            uri,
            service,
        }
    }
}

impl Drop for Tester {
    fn drop(&mut self) {
        self.service.kill().ok();
    }
}

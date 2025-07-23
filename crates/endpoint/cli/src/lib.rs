mod cli;

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

pub use cli::*;
use serde_json::{json, Value};
use valence_coprocessor::{Base64, Proof};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    /// Docker image
    pub docker: String,
    pub socket: String,
    pub tag: String,
    pub docker_host: bool,
}

struct ProjectStructure {
    pub metadata: Value,
    pub wsroot: String,
    pub pkrelative: String,
    pub package: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            docker: Self::DEFAULT_DOCKER.into(),
            socket: Self::DEFAULT_SOCKET.into(),
            tag: Self::DEFAULT_TAG.into(),
            docker_host: Self::DEFAULT_DOCKER_HOST,
        }
    }
}

impl App {
    pub const DEFAULT_DOCKER: &str = concat!("vtw11/valence:", env!("CARGO_PKG_VERSION"));
    pub const DEFAULT_SOCKET: &str = "https://service.coprocessor.valence.zone";
    pub const DEFAULT_TAG: &str = concat!("v", env!("CARGO_PKG_VERSION"));
    pub const DEFAULT_DOCKER_HOST: bool = false;

    pub fn with_docker<V: AsRef<str>>(mut self, docker: V) -> Self {
        self.docker = docker.as_ref().into();
        self
    }

    pub fn with_socket<V: AsRef<str>>(mut self, socket: V) -> Self {
        self.socket = socket.as_ref().into();
        self
    }

    pub fn with_tag<V: AsRef<str>>(mut self, tag: V) -> Self {
        self.tag = tag.as_ref().into();
        self
    }

    pub fn with_docker_host(mut self, docker_host: bool) -> Self {
        self.docker_host = docker_host;
        self
    }

    fn run_docker(
        &self,
        cmd: &str,
        wsroot: &str,
        package: &str,
        pkrelative: &str,
        args: &[&str],
    ) -> anyhow::Result<Output> {
        let mut command = Command::new("docker");

        command.args(["run", "--rm", "-i"]);

        if self.docker_host {
            command.args(["--network", "host"]);
        }

        command.args([
            "-v",
            format!("{wsroot}:/mnt").as_str(),
            &self.docker,
            cmd,
            &self.tag,
            package,
            format!("/mnt{pkrelative}").as_str(),
            &self.socket,
        ]);

        for a in args {
            command.arg(a);
        }

        let output = command.stderr(Stdio::inherit()).output()?;

        Ok(output)
    }

    /// Deploys a domain.
    ///
    /// Returns the deployed ID.
    pub fn deploy_domain<P, N>(&self, path: Option<P>, name: N) -> anyhow::Result<Value>
    where
        P: AsRef<Path>,
        N: AsRef<str>,
    {
        let ProjectStructure {
            wsroot,
            pkrelative,
            package,
            ..
        } = TryFrom::try_from(path)?;

        let output = self.run_docker("domain", &wsroot, &package, &pkrelative, &[name.as_ref()])?;

        anyhow::ensure!(output.status.success(), "failed to deploy domain");

        Ok(serde_json::from_slice(&output.stdout)?)
    }

    /// Deploys a circuit with its controller.
    ///
    /// Returns the deployed ID.
    pub fn deploy_circuit<P, C>(&self, controller: Option<P>, circuit: C) -> anyhow::Result<Value>
    where
        P: AsRef<Path>,
        C: AsRef<str>,
    {
        let ProjectStructure {
            metadata,
            wsroot,
            pkrelative,
            package,
        } = TryFrom::try_from(controller)?;

        let circuit = circuit.as_ref();
        let circuit_dir = metadata
            .get("packages")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow::anyhow!("failed to get packages from metadata"))?
            .iter()
            .find_map(|p| {
                p.get("name")
                    .and_then(Value::as_str)
                    .filter(|&n| n == circuit)
                    .and_then(|_| p.get("manifest_path").and_then(Value::as_str))
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "could not find the circuit `{circuit}` as workspace member of `{wsroot}`."
                )
            })
            .map(PathBuf::from)?
            .parent()
            .ok_or_else(|| {
                anyhow::anyhow!("could not define the crate directory of the circuit package.")
            })?
            .display()
            .to_string()
            .split_off(wsroot.len());

        let output = self.run_docker(
            "controller",
            &wsroot,
            &package,
            &pkrelative,
            &[circuit, format!("/mnt{}", circuit_dir.as_str()).as_str()],
        )?;

        anyhow::ensure!(output.status.success(), "failed to deploy circuit");

        Ok(serde_json::from_slice(&output.stdout)?)
    }

    /// Submits a proof to the co-processor queue.
    pub fn prove<C, P, A>(&self, circuit: C, output: P, args: Option<A>) -> anyhow::Result<Value>
    where
        C: AsRef<str>,
        P: AsRef<Path>,
        A: AsRef<str>,
    {
        let args: Value = match args {
            Some(a) => serde_json::from_str(a.as_ref())?,
            None => Value::Null,
        };
        let uri = format!(
            "{}/api/registry/controller/{}/prove",
            self.socket,
            circuit.as_ref(),
        );

        let response = reqwest::blocking::Client::new()
            .post(uri)
            .json(&json!({
                "args": args,
                "payload": {
                    "cmd": "store",
                    "path": output.as_ref()
                }
            }))
            .send()?
            .text()?;

        Ok(serde_json::from_str(&response)?)
    }

    pub fn storage<C, P>(&self, circuit: C, path: P) -> anyhow::Result<Value>
    where
        C: AsRef<str>,
        P: AsRef<Path>,
    {
        let uri = format!(
            "{}/api/registry/controller/{}/storage/fs",
            self.socket,
            circuit.as_ref()
        );

        let response = reqwest::blocking::Client::new()
            .post(uri)
            .json(&json!({
                "path": path.as_ref()
            }))
            .send()?
            .json::<Value>()?
            .get("data")
            .ok_or_else(|| anyhow::anyhow!("no data received"))?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("invalid data received"))?
            .to_string();

        Ok(json!({"data": response}))
    }

    /// Returns the verifying key of a circuit.
    pub fn vk<C>(&self, circuit: C) -> anyhow::Result<Value>
    where
        C: AsRef<str>,
    {
        let uri = format!(
            "{}/api/registry/controller/{}/vk",
            self.socket,
            circuit.as_ref()
        );

        let response = reqwest::blocking::Client::new()
            .get(uri)
            .send()?
            .json::<Value>()?
            .get("base64")
            .ok_or_else(|| anyhow::anyhow!("no data received"))?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("invalid data received"))?
            .to_string();

        Ok(json!({"vk": response}))
    }

    /// Returns the proof inputs of a proven circuit.
    pub fn proof_inputs<C, P>(&self, circuit: C, path: P) -> anyhow::Result<Value>
    where
        C: AsRef<str>,
        P: AsRef<Path>,
    {
        let uri = format!(
            "{}/api/registry/controller/{}/storage/fs",
            self.socket,
            circuit.as_ref()
        );

        let response = reqwest::blocking::Client::new()
            .post(uri)
            .json(&json!({
                "path": path.as_ref()
            }))
            .send()?
            .json::<Value>()?
            .get("data")
            .ok_or_else(|| anyhow::anyhow!("no data received"))?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("invalid data received"))?
            .to_string();

        let response = Base64::decode(response)?;
        let response: Value = serde_json::from_slice(&response)?;

        let proof = response
            .get("proof")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("unexpected data format for proof"))?;
        let inputs = Proof::try_from_base64(proof)?.inputs;

        Ok(json!({"inputs": inputs}))
    }
}

impl<P: AsRef<Path>> TryFrom<Option<P>> for ProjectStructure {
    type Error = anyhow::Error;

    fn try_from(path: Option<P>) -> anyhow::Result<Self> {
        let mut path = path
            .map(|p| p.as_ref().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
            .canonicalize()?;

        if path.is_dir() {
            path = path.join("Cargo.toml");
        }

        let metadata = Command::new("cargo")
            .args([
                "metadata",
                "--no-deps",
                "--format-version",
                "1",
                "--manifest-path",
                path.display().to_string().as_str(),
            ])
            .output()?
            .stdout;

        let metadata: Value = serde_json::from_slice(&metadata)?;
        let wsroot = metadata
            .get("workspace_root")
            .and_then(Value::as_str)
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("failed to get workspace root from cargo metadata"))?;

        let pkrelative = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("failed to get parent dir of manifest file"))?
            .display()
            .to_string()
            .split_off(wsroot.len());

        let manifest = fs::read_to_string(&path)?;
        let manifest: toml::Value = toml::from_str(&manifest)?;
        let package = manifest
            .get("package")
            .and_then(toml::Value::as_table)
            .and_then(|t| t.get("name"))
            .and_then(toml::Value::as_str)
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("failed to read package name from manifest"))?;

        Ok(Self {
            metadata,
            wsroot,
            pkrelative,
            package,
        })
    }
}

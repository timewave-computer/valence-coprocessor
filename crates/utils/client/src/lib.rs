use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::time::{self, Duration};
use uuid::Uuid;
use valence_coprocessor::{Base64, Hash, Proof, ValidatedDomainBlock, WitnessCoprocessor};

/// A co-processor client.
#[derive(Debug, Clone)]
pub struct Client {
    /// The co-processor address.
    pub coprocessor: String,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            coprocessor: Self::DEFAULT_COPROCESSOR.into(),
        }
    }
}

impl Client {
    /// The default co-processor public address.
    pub const DEFAULT_COPROCESSOR: &str = "prover.timewave.computer:37281";

    /// Creates a client with a localhost co-processor.
    pub fn local() -> Self {
        Self {
            coprocessor: "127.0.0.1:37281".into(),
        }
    }

    /// Creates a client with the provided co-processor address.
    pub fn with_coprocessor<C: AsRef<str>>(mut self, coprocessor: C) -> Self {
        self.coprocessor = coprocessor.as_ref().into();
        self
    }

    /// Computes the URI of the co-processor.
    pub fn uri<P: AsRef<str>>(&self, path: P) -> String {
        format!("http://{}/api/{}", self.coprocessor, path.as_ref(),)
    }

    /// Return the status of the co-processor.
    pub async fn stats(&self) -> anyhow::Result<Value> {
        let uri = self.uri("stats");

        Ok(reqwest::Client::new().get(uri).send().await?.json().await?)
    }

    /// Deploy a controller.
    ///
    /// Returns the allocated Id.
    ///
    /// # Arguments
    ///
    /// - `controller`: the runtime controller code.
    /// - `circuit`: the ELF circuit code.
    /// - `nonce`: a nonce to compose the computed Id.
    pub async fn deploy_controller<T, C>(
        &self,
        controller: T,
        circuit: C,
        nonce: Option<u64>,
    ) -> anyhow::Result<String>
    where
        T: AsRef<[u8]>,
        C: AsRef<[u8]>,
    {
        let uri = self.uri("registry/controller");

        reqwest::Client::new()
            .post(uri)
            .json(&json!({
                "controller": Base64::encode(controller),
                "circuit": Base64::encode(circuit),
                "nonce": nonce,
            }))
            .send()
            .await?
            .json::<Value>()
            .await?
            .get("controller")
            .and_then(Value::as_str)
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("invalid response"))
    }

    /// Deploy a domain.
    ///
    /// Returns the allocated Id.
    ///
    /// # Arguments
    ///
    /// - `domain`: the domain name.
    /// - `controller`: the runtime controller code.
    pub async fn deploy_domain<D, T>(&self, domain: D, controller: T) -> anyhow::Result<String>
    where
        D: AsRef<str>,
        T: AsRef<[u8]>,
    {
        let uri = self.uri("registry/domain");

        reqwest::Client::new()
            .post(uri)
            .json(&json!({
                "name": domain.as_ref(),
                "controller": Base64::encode(controller),
            }))
            .send()
            .await?
            .json::<Value>()
            .await?
            .get("domain")
            .and_then(Value::as_str)
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("invalid response"))
    }

    /// Fetch a storage file, returning its contents.
    ///
    /// The co-processor storage is a FAT-16 virtual filesystem, and bound to its limitations.
    ///
    /// - filenames limited to 8 characters, and 3 for the extension
    /// - case insensitive
    pub async fn get_storage_file<T, P>(&self, controller: T, path: P) -> anyhow::Result<Vec<u8>>
    where
        T: AsRef<str>,
        P: AsRef<str>,
    {
        let uri = format!("registry/controller/{}/storage/fs", controller.as_ref());
        let uri = self.uri(uri);

        reqwest::Client::new()
            .post(uri)
            .json(&json!({
                "path": path.as_ref(),
            }))
            .send()
            .await?
            .json::<Value>()
            .await?
            .get("data")
            .and_then(Value::as_str)
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("invalid response"))
            .and_then(Base64::decode)
    }

    /// Computes the witnesses of a controller for the provided arguments.
    ///
    /// This is a dry-run for the prove call, that will use the same components to compute the
    /// witnesses.
    ///
    /// # Arguments
    ///
    /// - `circuit`: the deployed circuit ID.
    /// - `args`: the arguments passed to the controlller.
    pub async fn get_witnesses<C: AsRef<str>>(
        &self,
        circuit: C,
        args: &Value,
    ) -> anyhow::Result<WitnessCoprocessor> {
        let uri = format!("registry/controller/{}/witnesses", circuit.as_ref());
        let uri = self.uri(uri);

        let data = reqwest::Client::new()
            .post(uri)
            .json(&json!({
                "args": args
            }))
            .send()
            .await?
            .json::<Value>()
            .await?;

        if let Some(log) = data.get("log").and_then(Value::as_array) {
            for l in log {
                if let Some(l) = l.as_str() {
                    tracing::debug!("{l}");
                }
            }
        }

        data.get("witnesses")
            .cloned()
            .and_then(|w| serde_json::from_value(w).ok())
            .ok_or_else(|| anyhow::anyhow!("invalid witnesses response"))
    }

    /// Queues a co-processor circuit proof request.
    ///
    /// # Arguments
    ///
    /// - `circuit`: the deployed circuit ID.
    /// - `args`: the arguments passed to the controlller.
    ///
    /// # Return
    ///
    /// Returns the path allocated for this proof.
    pub async fn queue_proof<C, R>(
        &self,
        circuit: C,
        root: Option<R>,
        args: &Value,
    ) -> anyhow::Result<String>
    where
        C: AsRef<str>,
        R: AsRef<str>,
    {
        let uri = root.map(|r| format!("/{}", r.as_ref())).unwrap_or_default();
        let uri = format!("registry/controller/{}/prove{}", circuit.as_ref(), uri);
        let uri = self.uri(uri);

        let output = Uuid::new_v4();
        let output = output.as_u128().to_le_bytes();
        let output = hex::encode(output);
        let output = format!("/var/share/proofs/{}.bin", &output[..8]);

        reqwest::Client::new()
            .post(uri)
            .json(&json!({
                "args": args,
                "payload": {
                    "cmd": "store",
                    "path": &output
                }
            }))
            .send()
            .await?
            .text()
            .await?;

        Ok(output)
    }

    /// Fetches a proof from the queue, returning if present.
    pub async fn get_proof<C: AsRef<str>, P: AsRef<str>>(
        &self,
        circuit: C,
        path: P,
    ) -> anyhow::Result<Option<Proof>> {
        let uri = format!("registry/controller/{}/storage/fs", circuit.as_ref());
        let uri = self.uri(uri);

        let response = reqwest::Client::new()
            .post(uri)
            .json(&json!({
                "path": path.as_ref()
            }))
            .send()
            .await?
            .json::<Value>()
            .await?;

        let data = match response.get("data") {
            Some(d) => d,
            _ => return Ok(None),
        };

        let data = data
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("unexpected data format"))?;

        if data.is_empty() {
            return Ok(None);
        }

        let data = Base64::decode(data)?;
        let data: Value = serde_json::from_slice(&data)?;

        if let Some(log) = data.get("log").and_then(Value::as_array) {
            for l in log {
                if let Some(l) = l.as_str() {
                    tracing::debug!("{l}");
                }
            }
        }

        anyhow::ensure!(
            data.get("success")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            "the proof was computed incorrectly"
        );

        let proof = data
            .get("proof")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("failed to get proof from response"))?;

        Proof::try_from_base64(proof).map(Some)
    }

    async fn _prove(&self, circuit: &str, path: &str, frequency: u64) -> anyhow::Result<Proof> {
        let frequency = Duration::from_millis(frequency);

        loop {
            if let Some(p) = self.get_proof(circuit, path).await? {
                return Ok(p);
            }

            time::sleep(frequency).await;
        }
    }

    /// Computes a proof for the given circuit, with the provided controller arguments.
    ///
    /// Uses the default retries & frequency arguments.
    pub async fn prove<C: AsRef<str>>(&self, circuit: C, args: &Value) -> anyhow::Result<Proof> {
        let retries = 25;
        let frequency = 2000;

        self.prove_with_params::<_, String>(circuit, None, retries, frequency, args)
            .await
    }

    /// Computes a proof for the given circuit, with the provided controller arguments.
    ///
    /// Overrides the latest co-processor root with the provided root.
    pub async fn prove_with_root<C, R>(
        &self,
        circuit: C,
        root: R,
        args: &Value,
    ) -> anyhow::Result<Proof>
    where
        C: AsRef<str>,
        R: AsRef<str>,
    {
        let retries = 25;
        let frequency = 2000;

        self.prove_with_params(circuit, Some(root), retries, frequency, args)
            .await
    }

    /// Computes a proof for the given circuit, with the provided controller arguments.
    pub async fn prove_with_params<C, R>(
        &self,
        circuit: C,
        root: Option<R>,
        retries: u64,
        frequency: u64,
        args: &Value,
    ) -> anyhow::Result<Proof>
    where
        C: AsRef<str>,
        R: AsRef<str>,
    {
        let circuit = circuit.as_ref();
        let path = self.queue_proof(circuit, root, args).await?;

        let duration = retries * frequency;
        let duration = Duration::from_millis(duration);
        let duration = time::sleep(duration);

        tokio::pin!(duration);

        tokio::select! {
            r = self._prove(circuit, &path, frequency) => {
                r
            }

            _ = &mut duration => {
                anyhow::bail!("proof timeout exceeded");
            }
        }
    }

    /// Get the verifying key for the provided circuit
    pub async fn get_vk<C: AsRef<str>>(&self, circuit: C) -> anyhow::Result<Vec<u8>> {
        let uri = format!("registry/controller/{}/vk", circuit.as_ref());
        let uri = self.uri(uri);

        let data = reqwest::Client::new()
            .get(uri)
            .send()
            .await?
            .json::<Value>()
            .await?;

        if let Some(log) = data.get("log").and_then(Value::as_array) {
            for l in log {
                if let Some(l) = l.as_str() {
                    tracing::debug!("{l}");
                }
            }
        }

        data.get("base64")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("invalid vk response"))
            .and_then(Base64::decode)
    }

    /// Get the circuit bytecode.
    pub async fn get_circuit<C: AsRef<str>>(&self, circuit: C) -> anyhow::Result<Vec<u8>> {
        let uri = format!("registry/controller/{}/circuit", circuit.as_ref());
        let uri = self.uri(uri);

        reqwest::Client::new()
            .get(uri)
            .send()
            .await?
            .json::<Value>()
            .await?
            .get("base64")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("invalid circuit response"))
            .and_then(Base64::decode)
    }

    /// Calls the controller entrypoint
    pub async fn entrypoint<T: AsRef<str>>(
        &self,
        controller: T,
        args: &Value,
    ) -> anyhow::Result<Value> {
        let uri = format!("registry/controller/{}/entrypoint", controller.as_ref());
        let uri = self.uri(uri);

        let data = reqwest::Client::new()
            .post(uri)
            .json(args)
            .send()
            .await?
            .json::<Value>()
            .await?;

        if let Some(log) = data.get("log").and_then(Value::as_array) {
            for l in log {
                if let Some(l) = l.as_str() {
                    tracing::debug!("{l}");
                }
            }
        }

        data.get("ret")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("no response provided"))
    }

    /// Returns the latest validated domain block.
    pub async fn get_latest_domain_block<D: AsRef<str>>(
        &self,
        domain: D,
    ) -> anyhow::Result<ValidatedDomainBlock> {
        let uri = format!("registry/domain/{}/latest", domain.as_ref());
        let uri = self.uri(uri);

        Ok(reqwest::Client::new().get(uri).send().await?.json().await?)
    }

    /// Appends a block to the domain, validating it with the controller.
    pub async fn add_domain_block<D: AsRef<str>>(
        &self,
        domain: D,
        args: &Value,
    ) -> anyhow::Result<AddedDomainBlock> {
        let uri = format!("registry/domain/{}", domain.as_ref());
        let uri = self.uri(uri);

        Ok(reqwest::Client::new()
            .post(uri)
            .json(&args)
            .send()
            .await?
            .json()
            .await?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddedDomainBlock {
    /// Domain to which the block was added.
    pub domain: String,
    /// Historical SMT root prior to the mutation.
    pub prev_smt: Hash,
    /// Historical SMT root after the mutation.
    pub smt: Hash,
    /// Controller execution log.
    pub log: Vec<String>,
    /// A block associated number.
    pub number: u64,
    /// The hash root of the block.
    pub root: Hash,
    /// SMT key to index the payload.
    pub key: Hash,
    /// Block blob payload.
    pub payload: Vec<u8>,
}

#[tokio::test]
#[ignore = "depends on remote service"]
async fn stats_works() {
    Client::default().stats().await.unwrap();
}

#[tokio::test]
#[ignore = "depends on remote service"]
async fn deploy_controller_works() {
    Client::default()
        .deploy_controller(b"foo", b"bar", Some(15))
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "depends on remote service"]
async fn deploy_domain_works() {
    Client::default()
        .deploy_domain("foo", b"bar")
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "depends on remote service and deployed circuit"]
async fn get_storage_file_works() {
    let controller = "d840ffde7bc7ad6004b4b0c2a7d66f5f87d5f9d7b649a9e75ab55becf55609c8";
    let path = "/var/share/proof.bin";

    Client::default()
        .get_storage_file(controller, path)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "depends on remote service and deployed circuit"]
async fn get_witnesses_works() {
    let circuit = "d840ffde7bc7ad6004b4b0c2a7d66f5f87d5f9d7b649a9e75ab55becf55609c8";
    let args = json!({"value": 42});

    Client::default()
        .get_witnesses(circuit, &args)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "depends on remote service and deployed circuit"]
async fn prove_works() {
    let circuit = "d840ffde7bc7ad6004b4b0c2a7d66f5f87d5f9d7b649a9e75ab55becf55609c8";
    let args = json!({"value": 42});

    Client::default().prove(circuit, &args).await.unwrap();
}


#[tokio::test]
#[ignore = "depends on remote service and deployed circuit"]
async fn prove_with_root_works() {
    let circuit = "d840ffde7bc7ad6004b4b0c2a7d66f5f87d5f9d7b649a9e75ab55becf55609c8";
    let args = json!({"value": 42});

    Client::default().prove(circuit, &args).await.unwrap();
}

#[tokio::test]
#[ignore = "depends on remote service and deployed circuit"]
async fn get_vk_works() {
    let circuit = "d840ffde7bc7ad6004b4b0c2a7d66f5f87d5f9d7b649a9e75ab55becf55609c8";

    Client::default().get_vk(circuit).await.unwrap();
}

#[tokio::test]
#[ignore = "depends on remote service and deployed circuit"]
async fn get_circuit_works() {
    let circuit = "d840ffde7bc7ad6004b4b0c2a7d66f5f87d5f9d7b649a9e75ab55becf55609c8";

    Client::default().get_circuit(circuit).await.unwrap();
}

#[tokio::test]
#[ignore = "depends on remote service and deployed circuit"]
async fn entrypoint_works() {
    let controller = "d840ffde7bc7ad6004b4b0c2a7d66f5f87d5f9d7b649a9e75ab55becf55609c8";
    let args = json!({
        "payload": {
            "cmd": "store",
            "path": "/etc/foo.bin",
        }
    });

    Client::default()
        .entrypoint(controller, &args)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "depends on remote service and deployed ethereum-alpha domain"]
async fn get_latest_domain_block_works() {
    Client::default()
        .get_latest_domain_block("ethereum-alpha")
        .await
        .unwrap();
}

use serde_json::{json, Value};
use tokio::time::{self, Duration};
use uuid::Uuid;
use valence_coprocessor::{Base64, Proof};

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
    pub fn stats(&self) -> anyhow::Result<Value> {
        let uri = self.uri("stats");

        Ok(reqwest::blocking::Client::new().get(uri).send()?.json()?)
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
    pub async fn queue_proof<C: AsRef<str>>(
        &self,
        circuit: C,
        args: &Value,
    ) -> anyhow::Result<String> {
        let uri = format!("registry/controller/{}/prove", circuit.as_ref());
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
        let retries = 15;
        let frequency = 2000;

        self.prove_with_params(circuit, retries, frequency, args)
            .await
    }

    /// Computes a proof for the given circuit, with the provided controller arguments.
    pub async fn prove_with_params<C: AsRef<str>>(
        &self,
        circuit: C,
        retries: u64,
        frequency: u64,
        args: &Value,
    ) -> anyhow::Result<Proof> {
        let circuit = circuit.as_ref();
        let path = self.queue_proof(circuit, args).await?;

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
}

#[test]
#[ignore = "depends on remote service"]
fn remote_stats_works() {
    Client::default().stats().unwrap();
}

#[tokio::test]
#[ignore = "depends on remote service and deployed circuit"]
async fn remote_prove_works() {
    let circuit = "7e0207a1fa0a979282b7246c028a6a87c25bc60f7b6d5230e943003634e897fd";
    let args = json!({"value": 42});

    Client::default().prove(circuit, &args).await.unwrap();
}

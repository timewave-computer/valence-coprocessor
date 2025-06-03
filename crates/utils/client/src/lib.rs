use serde_json::{json, Value};
use uuid::Uuid;

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
    pub const DEFAULT_COPROCESSOR: &str = "prover.timewave.computer:37281";

    pub fn local() -> Self {
        Self {
            coprocessor: "127.0.0.1:37281".into(),
        }
    }

    pub fn with_coprocessor<C: AsRef<str>>(mut self, coprocessor: C) -> Self {
        self.coprocessor = coprocessor.as_ref().into();
        self
    }

    pub fn uri<P: AsRef<str>>(&self, path: P) -> String {
        format!("http://{}/api/{}", self.coprocessor, path.as_ref(),)
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
    pub fn queue_proof<C: AsRef<str>>(&self, circuit: C, args: &Value) -> anyhow::Result<String> {
        let uri = format!("registry/controller/{}/prove", circuit.as_ref());
        let uri = self.uri(uri);

        let output = Uuid::new_v4();
        let output = output.as_u128().to_le_bytes();
        let output = hex::encode(&output);
        let output = format!("/var/share/proofs/{output}.bin");

        reqwest::blocking::Client::new()
            .post(uri)
            .json(&json!({
                "args": args,
                "payload": {
                    "cmd": "store",
                    "path": &output
                }
            }))
            .send()?
            .text()?;

        Ok(output)
    }

    /*
    /// Fetches a proof from the queue, returning if present.
    pub fn get_proof<C: AsRef<str>, P: AsRef<str>>(&self, circuit: C, path: P) {
        let uri = format!("registry/controller/{}/storage/fs", circuit.as_ref());
        let uri = self.uri(uri);

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
    }
    */
}

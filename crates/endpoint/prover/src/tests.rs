use std::{
    array,
    net::{TcpListener, TcpStream},
    ops::Deref,
    process::{Child, Command},
    thread,
    time::Duration,
};

use sp1_sdk::{Prover as _, ProverClient};

use crate::client::Client;

#[test]
#[ignore = "expensive test"]
fn test_get_sp1_mock_proof() {
    let c = TestClient::new();
    let circuit = array::from_fn(|i| i as u8);
    let w = 42u64.to_le_bytes();

    let proof = c
        .get_sp1_mock_proof(circuit, w, |_| Ok(c.elf.clone()))
        .unwrap();

    let vk = c
        .get_sp1_verifying_key(circuit, |_| Ok(c.elf.clone()))
        .unwrap();

    ProverClient::builder()
        .mock()
        .build()
        .verify(&proof, &vk)
        .unwrap();
}

struct TestClient {
    pub client: Client,
    pub child: Child,
    pub elf: Vec<u8>,
}

impl Deref for TestClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl TestClient {
    #[allow(clippy::zombie_processes)]
    pub fn new() -> Self {
        assert!(Command::new("cargo")
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .arg("build")
            .status()
            .unwrap()
            .success());

        let socket = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();

        let child = Command::new("cargo")
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .args(["run", "--", "--bind", socket.to_string().as_str()])
            .spawn()
            .unwrap();

        let elf = include_bytes!("../assets/hello.elf").to_vec();
        let timeout = Duration::from_millis(2000);

        for _ in 0..120 {
            thread::sleep(Duration::from_millis(1000));

            if TcpStream::connect_timeout(&socket, timeout).is_ok() {
                let client = Client::new(socket).unwrap();

                return Self { client, child, elf };
            }
        }

        panic!("failed to connect to service");
    }
}

impl Drop for TestClient {
    fn drop(&mut self) {
        self.child.kill().ok();
    }
}

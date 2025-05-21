use serde_json::{json, Value};
use sp1_sdk::{Prover as _, ProverClient, SP1ProofWithPublicValues, SP1VerifyingKey};
use valence_coprocessor::Base64;
use valence_coprocessor_integrated_tests::Tester;
use valence_coprocessor_integrated_tests_domain::{Domain, ID};

fn main() -> anyhow::Result<()> {
    let tester = Tester::default();
    println!("service loaded...");

    let domain = tester.build_wasm("valence-coprocessor-integrated-tests-domain-wasm")?;
    println!("domain built...");

    let program = tester.build_wasm("valence-coprocessor-integrated-tests-program-wasm")?;
    println!("program built...");

    let circuit = tester.build_circuit("program-circuit")?;
    println!("circuit built...");

    let name = ID;
    let domain = tester.post(
        "/registry/domain",
        json!({
            "name": name,
            "lib": Base64::encode(domain),
        }),
    )?["domain"]
        .as_str()
        .unwrap()
        .to_string();
    println!("domain registered `{domain}`...");

    let program = tester.post(
        "/registry/program",
        json!({
            "lib": Base64::encode(program),
            "circuit": Base64::encode(circuit),
        }),
    )?["program"]
        .as_str()
        .unwrap()
        .to_string();
    println!("program registered `{program}`...");

    let number = 2;
    let value = 15;
    tester.post(
        format!("/registry/domain/{name}"),
        serde_json::to_value(Domain::new_block(number, value))?,
    )?;
    println!("block `{number}->{value}` added...");

    let number = 3;
    let value = 13;
    tester.post(
        format!("/registry/domain/{name}"),
        serde_json::to_value(Domain::new_block(number, value))?,
    )?;
    println!("block `{number}->{value}` added...");

    let number = 1;
    let value = 17;
    tester.post(
        format!("/registry/domain/{name}"),
        serde_json::to_value(Domain::new_block(number, value))?,
    )?;
    println!("block `{number}->{value}` added...");

    let latest = tester.get(format!("/registry/domain/{name}/latest"))?;
    println!("latest block: `{}`...", serde_json::to_string(&latest)?);

    let stats = tester.get(format!("/stats"))?;
    println!("stats: `{}`...", serde_json::to_string(&stats)?);

    let path = "/var/share/proof.bin";
    let state = 13;
    let value = 8;
    tester.post(
        format!("/registry/program/{program}/prove"),
        json!({
            "args": {
                "state": {
                    "value": state
                },
                "value": value,
            },
            "payload": {
                "cmd": "store",
                "path": path
            }
        }),
    )?;
    println!("proof submitted...");

    let p = program.as_str();
    let t = &tester;
    let data = Tester::retry_until::<Value, anyhow::Error, _>(2000, 10, move || {
        let storage = t.post(
            format!("/registry/program/{p}/storage/fs"),
            json!({
                "path": path
            }),
        )?;

        let data = storage
            .get("data")
            .ok_or_else(|| anyhow::anyhow!("no data received"))?;

        let data = data
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("invalid data received"))?;

        let data = Base64::decode(data)?;
        let data = serde_json::from_slice(&data)?;

        Ok(data)
    })
    .unwrap();
    println!("storage fetched `{}`...", serde_json::to_string(&data)?);

    let data = data["proof"].as_str().unwrap();
    let data = Base64::decode(data)?;
    let mut proof: SP1ProofWithPublicValues = bincode::deserialize(&data)?;
    println!("proof decoded...");

    let vk = tester.get(format!("/registry/program/{program}/vk"))?;
    let vk = vk["base64"].as_str().unwrap();
    let vk = Base64::decode(vk)?;
    let vk: SP1VerifyingKey = bincode::deserialize(&vk)?;
    println!("vk decoded...");

    ProverClient::builder().mock().build().verify(&proof, &vk)?;
    println!("proof verified...");

    let out: u64 = proof.public_values.read();
    let expected = state + value;
    anyhow::ensure!(out == expected);
    println!("outputs verified!");

    Ok(())
}

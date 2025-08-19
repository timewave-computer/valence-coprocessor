use cargo_valence::{App, Cli, CmdDeploy, Commands};
use clap::Parser as _;

fn main() -> anyhow::Result<()> {
    let Cli {
        cmd,
        socket,
        tag,
        docker_host,
    } = Cli::parse();

    eprintln!("cargo-valence is deprecated! Use valence-domain-clients instead.");
    eprintln!("https://github.com/timewave-computer/valence-domain-clients?tab=readme-ov-file#cli");

    let app = App::default()
        .with_tag(tag)
        .with_socket(socket)
        .with_docker_host(docker_host);

    let response = match cmd {
        Commands::Deploy(d) => match d {
            CmdDeploy::Domain { name, controller } => app.deploy_domain(controller, name)?,

            CmdDeploy::Circuit {
                controller,
                circuit,
            } => app.deploy_circuit(controller, circuit)?,
        },

        Commands::Prove {
            circuit,
            json,
            path,
        } => app.prove(circuit, path, json)?,

        Commands::Storage { circuit, path } => app.storage(circuit, path)?,

        Commands::Vk { circuit } => app.vk(circuit)?,

        Commands::ProofInputs { circuit, path } => app.proof_inputs(circuit, path)?,
    };

    println!("{}", serde_json::to_string(&response)?);

    Ok(())
}

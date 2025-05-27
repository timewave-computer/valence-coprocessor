use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::App;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Socket address of the co-processor.
    #[arg(short, long, value_name = "SOCKET", default_value = App::DEFAULT_SOCKET)]
    pub socket: String,

    /// A port to be shared from the host to the docker container.
    #[arg(short, long, value_name = "PORT", default_value_t = App::DEFAULT_PORT)]
    pub port: u16,

    /// Co-processor version tag.
    #[arg(short, long, value_name = "TAG", default_value = App::DEFAULT_TAG)]
    pub tag: String,

    #[command(subcommand)]
    pub cmd: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Deploys definitions to the co-processor
    #[command(subcommand)]
    Deploy(CmdDeploy),

    /// Submits a proof request to the co-processor queue.
    Prove {
        /// ID of the deployed circuit
        #[arg(value_name = "CIRCUIT")]
        circuit: String,

        /// Optional JSON argument to be passed to the controller.
        #[arg(short, long, value_name = "JSON")]
        json: Option<String>,

        /// Path to store the proof on the virtual filesystem
        #[arg(
            short,
            long,
            value_name = "PATH",
            default_value = "/var/share/proof.bin"
        )]
        path: PathBuf,
    },

    /// Reads a file from the storage, returning its base64 data
    Storage {
        /// ID of the deployed circuit
        #[arg(value_name = "CIRCUIT")]
        circuit: String,

        /// Path to the file on the virtual filesystem
        #[arg(
            short,
            long,
            value_name = "PATH",
            default_value = "/var/share/proof.bin"
        )]
        path: PathBuf,
    },

    /// Returns the VK of a circuit
    Vk {
        /// ID of the deployed circuit
        #[arg(value_name = "CIRCUIT")]
        circuit: String,
    },

    /// Returns the public inputs of the proof stored on the provided path of the virtual
    /// filesystem.
    ProofInputs {
        /// ID of the deployed circuit
        #[arg(value_name = "CIRCUIT")]
        circuit: String,

        /// Path to the file on the virtual filesystem
        #[arg(
            short,
            long,
            value_name = "PATH",
            default_value = "/var/share/proof.bin"
        )]
        path: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum CmdDeploy {
    /// Deploys the domain definition to the co-processor
    Domain {
        /// Name of the domain to be deployed
        #[arg(short, long, value_name = "NAME")]
        name: String,

        /// Path of the controller (defaults to current dir).
        #[arg(short, long, value_name = "PATH")]
        controller: Option<PathBuf>,
    },

    /// Deploys a circuit to the co-processor.
    Circuit {
        /// Path of the controller. Must share a workspace with the circuit.
        #[arg(long, value_name = "CONTROLLER")]
        controller: Option<PathBuf>,

        /// Workspace member name of the circuit.
        #[arg(short, long, value_name = "CIRCUIT")]
        circuit: String,
    },
}

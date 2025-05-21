use std::net::TcpStream;

use msgpacker::{MsgPacker, Unpackable};
use serde::{Deserialize, Serialize};
use tungstenite::WebSocket;
use valence_coprocessor::Hash;

/// A circuit definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, MsgPacker)]
pub enum Circuit {
    /// A cached circuit identifier.
    Identifier(Hash),

    /// An ELF circuit definition.
    Elf {
        /// Custom identifier of the circuit.
        identifier: Hash,

        /// ELF bytes.
        bytes: String,
    },
}

impl From<Hash> for Circuit {
    fn from(id: Hash) -> Self {
        Self::Identifier(id)
    }
}

/// Jobs that can be accepted by a worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, MsgPacker)]
pub enum Request {
    /// SP1 groth16 proof
    Sp1Proof {
        /// Proving circuit
        circuit: Circuit,
        /// Circuit witnesses (base64)
        witnesses: String,
    },

    /// Get the SP1 verifying key.
    Sp1GetVerifyingKey {
        /// Proving circuit
        circuit: Circuit,
    },

    /// Close the connection
    Close,
}

/// Possible states resulting of a proof request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, MsgPacker)]
pub enum Response {
    /// Successfully executed a command without output.
    Ack,

    /// The provided circuit proving key was not found in the cache.
    ///
    /// The service should provide the full proving key.
    ProvingKeyNotCached,

    /// The proof result (base64)
    Proof(String),

    /// The verifying key (base64)
    VerifyingKey(String),

    /// An error has occurred.
    Err(String),
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Task {
    /// Connection request
    Conn(WebSocket<TcpStream>),

    /// Quit the worker thread
    Quit,
}

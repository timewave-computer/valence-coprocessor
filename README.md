# Valence co-processor

The Valence co-processor functions over zkVMs (Zero-Knowledge Virtual Machines), enabling users to effortlessly develop ZK applications that unify multiple chains through a single proof verification process.

Its main objective is to be user-friendly, ensuring a seamless experience by abstracting the complexities of the zkVMs while maintaining transparency about circuits and witnesses concepts. This allows users to access cross-chain data via light client-verified state proofs.

The service is available publicly at http://prover.timewave.computer:37281/

#### CLI helper

The Valence co-processor includes a CLI helper to facilitate the use of standard operations like deploying a domain, circuit, proving statements, and retrieving state information.

To install:

```sh
cargo install \
  --git https://github.com/timewave-computer/valence-coprocessor.git \
  --tag v0.3.3 \
  --locked cargo-valence
```

Check [valence-coprocessor-app](https://github.com/timewave-computer/valence-coprocessor-app) for an app template.

To deploy an application, you can call `cargo-valence deploy`:

```sh
cargo-valence --docker-host \
  deploy circuit \
  --controller ./crates/controller \
  --circuit valence-coprocessor-app-vault-circuit
```

This process aims to deploy the circuit into a locally running co-processor instance. It leverages a Docker host network, ensuring seamless operation without additional setup. On Linux systems, utilize the `--docker-host` option. However, for Windows and OSX users, opt for the `--socket` option instead, and ensure you've identified your exposed IP address accessible to Docker containers, as this is where the circuit will be deployed.

To deploy a circuit to the public co-processor:

```sh
cargo-valence --socket prover.timewave.computer:37281 \
  deploy circuit \
  --controller ./crates/controller \
  --circuit valence-coprocessor-app-vault-circuit
```

#### Local execution

The service can be started locally via:

```sh
VALENCE_PROVER_SECRET=secret \
  ALCHEMY_API_KEY=key \
  cargo run -p valence-coprocessor-service
```

It will create an in-memory database and will consume the remote prover service. Note: the environment variable `VALENCE_PROVER_SECRET` must be set in order to be able to consume the dedicated prover. You can request a secret directly with the team. Alternatively, you can use the public service.

You can also customize the log messages via [RUST_LOG](https://rust-lang-nursery.github.io/rust-cookbook/development_tools/debugging/config_log.html):

```sh
VALENCE_PROVER_SECRET=secret \
  ALCHEMY_API_KEY=key \
  RUST_LOG=info,valence_coprocessor=debug,valence_coprocessor_wasm=debug \
  cargo run -p valence-coprocessor-service
```

#### Nix

Alternatively, use Nix for a reproducible development environment:

```sh
# Enter development shell
nix develop

# Start service (equivalent to cargo run-service)
VALENCE_PROVER_SECRET=secret start-service

# Or run directly
nix run .#service
```

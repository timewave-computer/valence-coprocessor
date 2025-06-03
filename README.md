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
  --tag v0.2.0 \
  --locked cargo-valence
```

Check [valence-coprocessor-app](https://github.com/timewave-computer/valence-coprocessor-app) for an app template.

#### Local execution

The service can be started locally via:

```sh
VALENCE_PROVER_SECRET=secret cargo run-service
```

It will create an in-memory database and will consume the remote prover service. Note: the environment variable `VALENCE_PROVER_SECRET` must be set in order to be able to consume the dedicated prover. You can request a secret directly with the team. Alternatively, you can use the public service.

You can also customize the log messages via [RUST_LOG](https://rust-lang-nursery.github.io/rust-cookbook/development_tools/debugging/config_log.html):

```sh
VALENCE_PROVER_SECRET=secret \
  RUST_LOG=info,valence_coprocessor=debug,valence_coprocessor_wasm=debug \
  cargo run-service
```

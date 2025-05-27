#!/bin/bash

set -e

if [ -z "$1" ] || [ -z "$2" ] || [ -z "$3" ] || [ -z "$4" ] || [ -z "$5" ]; then
  echo "Error: Missing arguments."
  echo "Arguments: <CMD> <VERSION> <PROJECT> <DIR> <SOCKET> <command specific>"
  exit 1
fi

CMD=$(echo "$1" | tr '[:upper:]' '[:lower:]')
VERSION="$2"
PROJECT="$3"
LIB=$(echo "$3" | tr '-' '_')
DIR=$(echo "$4" | sed 's/\//\\\//g')
SOCKET="$5"

case "$CMD" in
  "domain")
    NAME="$6"

    if [ -z "$NAME" ]; then
      echo "Error: Missing domain name."
      exit 1
    fi

    sed -i "s/!PROJECT!/$PROJECT/g" \
      /rust-wrappers/domain-controller/Cargo.toml
    sed -i "s/!VERSION!/$VERSION/g" \
      /rust-wrappers/domain-controller/Cargo.toml
    sed -i "s/!PATH!/$DIR/g" \
      /rust-wrappers/domain-controller/Cargo.toml
    sed -i "s/!LIB!/$LIB/g" \
      /rust-wrappers/domain-controller/src/lib.rs

    $CARGO_HOME/bin/cargo build \
      --target wasm32-unknown-unknown \
      --release \
      --manifest-path /rust-wrappers/domain-controller/Cargo.toml

    TARGET="/tmp/domain-controller-json"
    CONTROLLER="/rust-wrappers/domain-controller/target/wasm32-unknown-unknown/release/${LIB}_controller.wasm"

    echo -n '{"controller": "' > $TARGET
    base64 -w 0 $CONTROLLER >> $TARGET
    echo -n '", "name": "' >> $TARGET
    echo -n $NAME >> $TARGET
    echo -n '"}' >> $TARGET

    cat $TARGET | curl -X POST \
      -H "Content-Type: application/json" \
      --data-binary @- "$SOCKET/api/registry/domain" | jq
    ;;
  "controller")
    CIRCUIT_PROJECT="$6"
    CIRCUIT_DIR=$(echo "$7" | sed 's/\//\\\//g')
    CIRCUIT_LIB=$(echo "$6" | tr '-' '_')

    if [ -z "$CIRCUIT_PROJECT" ]; then
      echo "Error: Missing circuit project."
      exit 1
    fi

    if [ -z "$CIRCUIT_DIR" ]; then
      echo "Error: Missing circuit path."
      exit 1
    fi

    sed -i "s/!PROJECT!/$PROJECT/g" \
      /rust-wrappers/controller/Cargo.toml
    sed -i "s/!VERSION!/$VERSION/g" \
      /rust-wrappers/controller/Cargo.toml
    sed -i "s/!PATH!/$DIR/g" \
      /rust-wrappers/controller/Cargo.toml
    sed -i "s/!LIB!/$LIB/g" \
      /rust-wrappers/controller/src/lib.rs

    sed -i "s/!PROJECT!/$CIRCUIT_PROJECT/g" \
      /rust-wrappers/circuit/Cargo.toml
    sed -i "s/!VERSION!/$VERSION/g" \
      /rust-wrappers/circuit/Cargo.toml
    sed -i "s/!PATH!/$CIRCUIT_DIR/g" \
      /rust-wrappers/circuit/Cargo.toml
    sed -i "s/!LIB!/$CIRCUIT_LIB/g" \
      /rust-wrappers/circuit/src/main.rs

    $CARGO_HOME/bin/cargo build \
      --target wasm32-unknown-unknown \
      --release \
      --manifest-path /rust-wrappers/controller/Cargo.toml

    cd /rust-wrappers/circuit && \
      PATH="$PATH:/root/.sp1/bin" $CARGO_HOME/bin/cargo prove build >&2

    TARGET="/tmp/controller-json"
    CONTROLLER="/rust-wrappers/controller/target/wasm32-unknown-unknown/release/${LIB}_controller.wasm"
    CIRCUIT="/rust-wrappers/circuit/target/elf-compilation/riscv32im-succinct-zkvm-elf/release/program-circuit"

    echo -n '{"controller": "' > $TARGET
    base64 -w 0 $CONTROLLER >> $TARGET
    echo -n '", "circuit": "' >> $TARGET
    base64 -w 0 $CIRCUIT >> $TARGET
    echo -n '"}' >> $TARGET

    cat $TARGET | curl -X POST \
      -H "Content-Type: application/json" \
      --data-binary @- "$SOCKET/api/registry/controller" | jq
    ;;
  *)
    echo "Error: '$1' is not a recognized word."
    echo "Possible words: domain, controller"
    exit 1
    ;;
esac

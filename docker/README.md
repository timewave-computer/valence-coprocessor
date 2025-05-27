# Valence Co-Processor docker

This is the docker definition for the Valence circuit & controller builder.

#### Example

```sh
docker run --rm -i \
  -v $HOME/dev/timewave/valence-coprocessor-app:/mnt \
  valence-coprocessor-utils:0.1.0 \
  domain \
  v0.1.12 \
  valence-coprocessor-app-domain \
  /mnt/crates/domain \
  104.171.203.127:37281 \
  foo
```

```json
{
  "domain": "0e7764b61da6abb6dbd6c7e233e8781308538e73e583a1cbf54f41b01e419f6f"
}
```

```sh
docker run --rm -i \
  -v $HOME/dev/timewave/valence-coprocessor-app:/mnt \
  valence-coprocessor-utils:0.1.0 \
  controller \
  v0.1.12 \
  valence-coprocessor-app-controller \
  /mnt/crates/controller \
  104.171.203.127:37281 \
  valence-coprocessor-app-circuit \
  /mnt/crates/circuit
```

```json
{
  "controller": "7e21982aa4b80822e36c65e00f1190cd7b6e729c9c98a1e83197bd49a2b892e1"
}
```

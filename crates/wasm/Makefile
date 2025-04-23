PROJECT := $(shell cargo pkgid | cut -d '#' -f2 | cut -d ':' -f1 | cut -d '@' -f1)
VERSION := $(shell cargo pkgid | cut -d '#' -f2 | cut -d ':' -f1 | cut -d '@' -f2)

all: wasm ## Default target.

wasm: ## Build the binaries.
	cargo build \
		--target wasm32-unknown-unknown \
		--release \
		--manifest-path contrib/hello/Cargo.toml
	cp contrib/hello/target/wasm32-unknown-unknown/release/valence_coprocessor_wasm_hello.wasm \
		contrib/hello/hello.wasm

help: ## Display this help screen
	@echo -e "\033[1;37m$(PROJECT) ($(VERSION))\033[0m"
	@grep -h -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

.PHONY: help all wasm

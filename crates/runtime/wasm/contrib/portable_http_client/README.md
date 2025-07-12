# HTTP Client - Portable WASM Module

A portable HTTP client WASM module that works in both browser and Valence coprocessor environments. This demonstrates how to create WASM binaries that can make HTTP calls across different runtime environments.

This example demonstrates the interaction with the coprocessor portability logic located at `crates/runtime/wasm/src/portable.rs`.

## Features

- **Portable**: Works in both browser and Valence coprocessor environments
- **Generic HTTP Interface**: Can interact with any HTTP-based API
- **JSON-RPC Support**: Built-in support for JSON-RPC protocols (commonly used by blockchain nodes)
- **REST API Support**: Flexible REST API calling capabilities

## Usage

### Generic HTTP Commands

#### GET Request
```json
{
  "command": "http_get",
  "url": "https://api.example.com/data",
  "headers": {
    "Authorization": "Bearer token",
    "Accept": "application/json"
  }
}
```

#### POST Request
```json
{
  "command": "http_post",
  "url": "https://api.example.com/submit",
  "headers": {
    "Content-Type": "application/json"
  },
  "json": {
    "data": "value"
  }
}
```

#### REST API Call
```json
{
  "command": "rest_api",
  "url": "https://api.example.com/endpoint",
  "method": "GET"
}
```

## Building

```bash
cargo build --target wasm32-unknown-unknown --release
```

The compiled WASM binary will be portable across both browser and Valence environments.

## Architecture

The module provides a common HTTP abstraction layer that:

1. **In Valence Environment**: Uses the built-in `abi::http` host function
2. **In Browser Environment**: Can be configured to use `fetch` API (via wasm-bindgen)

The portable interface provides a common API surface that works across both environments, allowing the same WASM binary to run in different contexts.

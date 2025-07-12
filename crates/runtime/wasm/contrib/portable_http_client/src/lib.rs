//! HTTP Client - A portable WASM module for making HTTP requests
//!
//! This module demonstrates how to create WASM binaries that can make HTTP calls
//! to any HTTP-based API in both browser and Valence coprocessor environments.
//! While the examples focus on blockchain/Ethereum JSON-RPC, the HTTP interface
//! is completely generic and can be used with any REST API, GraphQL endpoint,
//! or other HTTP-based service.

#![no_std]

extern crate alloc;

use alloc::{string::String, format};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use valence_coprocessor_wasm::{abi, portable::*};

/// Generic JSON-RPC request
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: u64,
}

/// Generic JSON-RPC response
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<JsonRpcError>,
    id: u64,
}

/// JSON-RPC error
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    data: Option<Value>,
}

/// Block information structure (Ethereum example)
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct BlockInfo {
    number: String,
    hash: String,
    #[serde(rename = "parentHash")]
    parent_hash: String,
    timestamp: String,
    #[serde(rename = "gasLimit")]
    gas_limit: String,
    #[serde(rename = "gasUsed")]
    gas_used: String,
}

/// Account balance information (Ethereum example)
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct AccountInfo {
    balance: String,
    nonce: u64,
}

/// Main entrypoint for the HTTP client
#[no_mangle]
pub extern "C" fn entrypoint() {
    let args = match abi::args() {
        Ok(args) => args,
        Err(e) => {
            let response = json!({
                "success": false,
                "error": format!("Failed to parse arguments: {}", e),
            });
            let _ = abi::ret(&response);
            return;
        }
    };
    
    let command = args["command"].as_str().unwrap_or("help");
    
    let result = match command {
        // Ethereum/Blockchain specific commands (examples)
        "get_block" => get_block_info(&args),
        "get_balance" => get_account_balance(&args),
        "get_transaction" => get_transaction(&args),
        "eth_call" => make_eth_call(&args),
        "test_connection" => test_node_connection(&args),
        
        // Generic HTTP commands
        "http_get" => make_http_get(&args),
        "http_post" => make_http_post(&args),
        "rest_api" => call_rest_api(&args),
        
        "help" => show_help(),
        _ => Err(format!("Unknown command: {}", command)),
    };

    let response = match result {
        Ok(data) => json!({
            "success": true,
            "data": data
        }),
        Err(error) => json!({
            "success": false,
            "error": error
        })
    };

    // Return the response, handling potential serialization errors
    if let Err(e) = abi::ret(&response) {
        // If we can't return the response, try to return a minimal error
        let fallback_response = json!({
            "success": false,
            "error": format!("Failed to serialize response: {}", e)
        });
        let _ = abi::ret(&fallback_response);
    }
}

/// Make a generic HTTP GET request
fn make_http_get(args: &Value) -> Result<Value, String> {
    let url = args["url"].as_str()
        .ok_or("Missing 'url' parameter")?;
    
    let mut request = HttpRequest::get(url);
    
    // Add headers if provided
    if let Some(headers) = args["headers"].as_object() {
        for (key, value) in headers {
            if let Some(value_str) = value.as_str() {
                request = request.header(key, value_str);
            } else {
                return Err(format!("Header value for key '{}' must be a string, got: {:?}", key, value));
            }
        }
    }
    
    let response = HttpClient::execute(request)
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    
    Ok(json!({
        "status": response.status,
        "headers": response.headers,
        "body": response.text().map_err(|_| String::from("Response body contains invalid UTF-8"))?
    }))
}

/// Make a generic HTTP POST request
fn make_http_post(args: &Value) -> Result<Value, String> {
    let url = args["url"].as_str()
        .ok_or("Missing 'url' parameter")?;
    
    let mut request = HttpRequest::post(url);
    
    // Add headers if provided
    if let Some(headers) = args["headers"].as_object() {
        for (key, value) in headers {
            if let Some(value_str) = value.as_str() {
                request = request.header(key, value_str);
            } else {
                return Err(format!("Header value for key '{}' must be a string, got: {:?}", key, value));
            }
        }
    }
    
    // Add body if provided
    if let Some(body) = args["body"].as_str() {
        request = request.body(body.as_bytes());
    } else if let Some(json_body) = args.get("json") {
        request = request.json_value(json_body)
            .map_err(|e| format!("Failed to serialize JSON body: {}", e))?;
    }
    
    let response = HttpClient::execute(request)
        .map_err(|e| format!("HTTP request failed: {}", e))?;
    
    Ok(json!({
        "status": response.status,
        "headers": response.headers,
        "body": response.text().map_err(|_| String::from("Response body contains invalid UTF-8"))?
    }))
}

/// Call a REST API with flexible parameters
fn call_rest_api(args: &Value) -> Result<Value, String> {
    let url = args["url"].as_str()
        .ok_or("Missing 'url' parameter")?;
    
    let method = args["method"].as_str().unwrap_or("GET");
    
    let mut request = match method.to_uppercase().as_str() {
        "GET" => HttpRequest::get(url),
        "POST" => HttpRequest::post(url),
        "PUT" => HttpRequest::put(url),
        "DELETE" => HttpRequest::delete(url),
        "PATCH" => HttpRequest::patch(url),
        "HEAD" => HttpRequest::head(url),
        _ => return Err(format!("Unsupported HTTP method: {}. Supported methods: GET, POST, PUT, DELETE, PATCH, HEAD", method)),
    };
    
    // Add headers if provided
    if let Some(headers) = args["headers"].as_object() {
        for (key, value) in headers {
            if let Some(value_str) = value.as_str() {
                request = request.header(key, value_str);
            } else {
                return Err(format!("Header value for key '{}' must be a string, got: {:?}", key, value));
            }
        }
    }
    
    // Add body if provided (for methods that support it)
    match method.to_uppercase().as_str() {
        "POST" | "PUT" | "PATCH" => {
            if let Some(body) = args["body"].as_str() {
                request = request.body(body.as_bytes());
            } else if let Some(json_body) = args.get("json") {
                request = request.json_value(json_body)
                    .map_err(|e| format!("Failed to serialize JSON body: {}", e))?;
            }
        },
        _ => {
            // For GET, DELETE, HEAD methods, warn if body is provided
            if args.get("body").is_some() || args.get("json").is_some() {
                return Err(format!("{} method does not support request body", method.to_uppercase()));
            }
        }
    }
    
    let response = HttpClient::execute(request)
        .map_err(|e| format!("REST API call failed: {}", e))?;
    
    if response.is_success() {
        // Try to parse as JSON if possible
        match response.json_value() {
            Ok(json_data) => Ok(json_data),
            Err(_) => Ok(json!({
                "text": response.text().map_err(|_| String::from("Response body contains invalid UTF-8"))?
            }))
        }
    } else {
        Err(format!("API call failed with status: {}", response.status))
    }
}

// === Blockchain/Ethereum specific examples below ===
// These demonstrate how the generic HTTP interface can be used for specific APIs

/// Get block information by number or hash (Ethereum example)
fn get_block_info(args: &Value) -> Result<Value, String> {
    let node_url = args["node_url"].as_str()
        .ok_or("Missing 'node_url' parameter")?;
    
    let block_number = args["block_number"].as_str().unwrap_or("latest");
    
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "eth_getBlockByNumber".into(),
        params: json!([block_number, false]),
        id: 1,
    };

    let response = make_rpc_call(node_url, &request)?;
    
    if let Some(error) = response.error {
        return Err(format!("RPC Error: {} - {}", error.code, error.message));
    }

    Ok(response.result.unwrap_or_default())
}

/// Get account balance (Ethereum example)
fn get_account_balance(args: &Value) -> Result<Value, String> {
    let node_url = args["node_url"].as_str()
        .ok_or("Missing 'node_url' parameter")?;
    
    let address = args["address"].as_str()
        .ok_or("Missing 'address' parameter")?;
    
    let block_number = args["block_number"].as_str().unwrap_or("latest");

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "eth_getBalance".into(),
        params: json!([address, block_number]),
        id: 1,
    };

    let response = make_rpc_call(node_url, &request)?;
    
    if let Some(error) = response.error {
        return Err(format!("RPC Error: {} - {}", error.code, error.message));
    }

    Ok(json!({
        "address": address,
        "balance": response.result.unwrap_or_default(),
        "block": block_number
    }))
}

/// Get transaction information (Ethereum example)
fn get_transaction(args: &Value) -> Result<Value, String> {
    let node_url = args["node_url"].as_str()
        .ok_or("Missing 'node_url' parameter")?;
    
    let tx_hash = args["tx_hash"].as_str()
        .ok_or("Missing 'tx_hash' parameter")?;

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "eth_getTransactionByHash".into(),
        params: json!([tx_hash]),
        id: 1,
    };

    let response = make_rpc_call(node_url, &request)?;
    
    if let Some(error) = response.error {
        return Err(format!("RPC Error: {} - {}", error.code, error.message));
    }

    Ok(response.result.unwrap_or_default())
}

/// Make an eth_call to interact with smart contracts (Ethereum example)
fn make_eth_call(args: &Value) -> Result<Value, String> {
    let node_url = args["node_url"].as_str()
        .ok_or("Missing 'node_url' parameter")?;
    
    let to = args["to"].as_str()
        .ok_or("Missing 'to' parameter")?;
    
    let data = args["data"].as_str()
        .ok_or("Missing 'data' parameter")?;
    
    let block_number = args["block_number"].as_str().unwrap_or("latest");

    let call_object = json!({
        "to": to,
        "data": data
    });

    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "eth_call".into(),
        params: json!([call_object, block_number]),
        id: 1,
    };

    let response = make_rpc_call(node_url, &request)?;
    
    if let Some(error) = response.error {
        return Err(format!("RPC Error: {} - {}", error.code, error.message));
    }

    Ok(json!({
        "to": to,
        "data": data,
        "result": response.result.unwrap_or_default(),
        "block": block_number
    }))
}

/// Test connection to blockchain node (Ethereum example)
fn test_node_connection(args: &Value) -> Result<Value, String> {
    let node_url = args["node_url"].as_str()
        .ok_or("Missing 'node_url' parameter")?;

    // Test with a simple eth_chainId call
    let request = JsonRpcRequest {
        jsonrpc: "2.0".into(),
        method: "eth_chainId".into(),
        params: json!([]),
        id: 1,
    };

    let response = make_rpc_call(node_url, &request)?;
    
    if let Some(error) = response.error {
        return Err(format!("RPC Error: {} - {}", error.code, error.message));
    }

    Ok(json!({
        "node_url": node_url,
        "chain_id": response.result.unwrap_or_default(),
        "status": "connected"
    }))
}

/// Show help information
fn show_help() -> Result<Value, String> {
    Ok(json!({
        "description": "Portable HTTP Client for WASM - works in both browser and Valence environments",
        "commands": {
            "http_get": {
                "description": "Make a generic HTTP GET request",
                "parameters": {
                    "url": "Target URL",
                    "headers": "Optional headers object"
                }
            },
            "http_post": {
                "description": "Make a generic HTTP POST request",
                "parameters": {
                    "url": "Target URL",
                    "headers": "Optional headers object",
                    "body": "Request body (string)",
                    "json": "Request body (JSON object)"
                }
            },
            "rest_api": {
                "description": "Call a REST API with flexible parameters",
                "parameters": {
                    "url": "API endpoint URL",
                    "method": "HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD) - defaults to GET",
                    "headers": "Optional headers object",
                    "body": "Request body (string) - only for POST, PUT, PATCH",
                    "json": "Request body (JSON object) - only for POST, PUT, PATCH"
                }
            },
            "blockchain_examples": {
                "get_block": {
                    "description": "Get blockchain block information (Ethereum)",
                    "parameters": {
                        "node_url": "Blockchain node URL",
                        "block_number": "Block number or 'latest' (optional)"
                    }
                },
                "get_balance": {
                    "description": "Get account balance (Ethereum)",
                    "parameters": {
                        "node_url": "Blockchain node URL",
                        "address": "Ethereum address",
                        "block_number": "Block number or 'latest' (optional)"
                    }
                },
                "get_transaction": {
                    "description": "Get transaction information (Ethereum)",
                    "parameters": {
                        "node_url": "Blockchain node URL",
                        "tx_hash": "Transaction hash"
                    }
                },
                "eth_call": {
                    "description": "Call smart contract function (Ethereum)",
                    "parameters": {
                        "node_url": "Blockchain node URL",
                        "to": "Contract address",
                        "data": "Encoded function call data",
                        "block_number": "Block number or 'latest' (optional)"
                    }
                },
                "test_connection": {
                    "description": "Test connection to blockchain node (Ethereum)",
                    "parameters": {
                        "node_url": "Blockchain node URL"
                    }
                }
            }
        }
    }))
}

/// Make a JSON-RPC call using portable HTTP interface
fn make_rpc_call(node_url: &str, request: &JsonRpcRequest) -> Result<JsonRpcResponse, String> {
    // Convert to JSON Value for the portable interface
    let request_value = serde_json::to_value(request)
        .map_err(|e| format!("Failed to serialize request: {}", e))?;
    
    // Use the portable HTTP interface
    let http_response = post_json_value(node_url, &request_value)
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !http_response.is_success() {
        return Err(format!("HTTP error: {}", http_response.status));
    }

    let response_value = http_response.json_value()
        .map_err(|e| format!("Failed to parse JSON response: {}", e))?;
    
    let rpc_response: JsonRpcResponse = serde_json::from_value(response_value)
        .map_err(|e| format!("Failed to deserialize response: {}", e))?;

    Ok(rpc_response)
} 
//! Portable HTTP interface for cross-environment WASM compatibility
//! 
//! This module provides a common HTTP interface that can be compiled to work
//! in both browser environments (using fetch) and Valence coprocessor environments
//! (using the custom host function).

extern crate alloc;

use alloc::{string::String, vec::Vec, format};
use core::fmt;
use serde_json::Value;

/// Portable HTTP request configuration
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub url: String,
    pub method: HttpMethod,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub timeout_secs: Option<u64>,
}

/// HTTP methods
#[derive(Debug, Clone)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Delete => write!(f, "DELETE"),
            HttpMethod::Patch => write!(f, "PATCH"),
            HttpMethod::Head => write!(f, "HEAD"),
        }
    }
}

/// Portable HTTP response
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpRequest {
    /// Create a new GET request
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: HttpMethod::Get,
            headers: Vec::new(),
            body: None,
            timeout_secs: Some(30),
        }
    }

    /// Create a new POST request
    pub fn post(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: HttpMethod::Post,
            headers: Vec::new(),
            body: None,
            timeout_secs: Some(30),
        }
    }

    /// Add a header
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    /// Set the request body
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Set JSON body (accepts any Value that can be serialized to JSON)
    pub fn json_value(mut self, data: &Value) -> Result<Self, serde_json::Error> {
        let json_str = serde_json::to_string(data)?;
        self.body = Some(json_str.into_bytes());
        self.headers.push((String::from("Content-Type"), String::from("application/json")));
        Ok(self)
    }

    /// Set timeout in seconds
    pub fn timeout(mut self, seconds: u64) -> Self {
        self.timeout_secs = Some(seconds);
        self
    }
}

impl HttpResponse {
    /// Check if the response was successful (2xx status)
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// Get response body as string
    pub fn text(&self) -> Result<String, core::str::Utf8Error> {
        core::str::from_utf8(&self.body).map(|s| String::from(s))
    }

    /// Parse response body as JSON Value
    pub fn json_value(&self) -> Result<Value, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }

    /// Get a header value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// Portable HTTP client that works in both browser and Valence environments
pub struct HttpClient;

impl HttpClient {
    /// Execute an HTTP request
    pub fn execute(request: HttpRequest) -> Result<HttpResponse, HttpError> {
        // Always use Valence host for now
        execute_valence_host(request)
    }
}

/// HTTP error types
#[derive(Debug)]
pub enum HttpError {
    /// Network error
    Network(String),
    /// Timeout error
    Timeout,
    /// Serialization error
    Serialization(String),
    /// Unsupported environment
    UnsupportedEnvironment,
    /// Invalid URL
    InvalidUrl,
    /// Invalid response
    InvalidResponse,
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpError::Network(msg) => write!(f, "Network error: {}", msg),
            HttpError::Timeout => write!(f, "Request timeout"),
            HttpError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            HttpError::UnsupportedEnvironment => write!(f, "Unsupported environment"),
            HttpError::InvalidUrl => write!(f, "Invalid URL"),
            HttpError::InvalidResponse => write!(f, "Invalid response"),
        }
    }
}

fn execute_valence_host(request: HttpRequest) -> Result<HttpResponse, HttpError> {
    use crate::abi;
    
    // Convert headers to JSON object
    let headers: serde_json::Map<String, Value> = request
        .headers
        .iter()
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();

    // Build request JSON for Valence host function
    let mut req_json = serde_json::json!({
        "url": request.url,
        "method": format!("{}", request.method).to_lowercase(),
        "headers": headers,
    });

    // Add body if present
    if let Some(body) = request.body {
        req_json["body"] = Value::Array(body.iter().map(|&b| Value::Number(b.into())).collect());
    }

    // Make the HTTP request through Valence host function
    let response = abi::http(&req_json)
        .map_err(|e| HttpError::Network(format!("{}", e)))?;

    // Parse response
    let status = response["status"].as_u64().ok_or(HttpError::InvalidResponse)? as u16;
    
    let headers: Vec<(String, String)> = response["headers"]
        .as_object()
        .unwrap_or(&serde_json::Map::new())
        .iter()
        .map(|(k, v)| (k.clone(), String::from(v.as_str().unwrap_or(""))))
        .collect();

    let body = match &response["body"] {
        Value::Array(arr) => arr
            .iter()
            .map(|v| v.as_u64().unwrap_or(0) as u8)
            .collect(),
        Value::String(s) => s.as_bytes().to_vec(),
        _ => Vec::new(),
    };

    Ok(HttpResponse {
        status,
        headers,
        body,
    })
}

/// Convenience function for making GET requests
pub fn get(url: impl Into<String>) -> Result<HttpResponse, HttpError> {
    HttpClient::execute(HttpRequest::get(url))
}

/// Convenience function for making POST requests
pub fn post(url: impl Into<String>) -> Result<HttpResponse, HttpError> {
    HttpClient::execute(HttpRequest::post(url))
}

/// Convenience function for making JSON POST requests with serde_json::Value
pub fn post_json_value(url: impl Into<String>, data: &Value) -> Result<HttpResponse, HttpError> {
    let request = HttpRequest::post(url).json_value(data)
        .map_err(|e| HttpError::Serialization(format!("{}", e)))?;
    HttpClient::execute(request)
} 
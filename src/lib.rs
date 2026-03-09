// src/lib.rs — mcpjs:MCP server framework

#![deny(clippy::all)]
#![allow(clippy::needless_pass_by_value)]

use napi::threadsafe_function::ThreadsafeFunction;
use napi_derive::napi;
use serde::{Deserialize, Serialize};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// ─── MCP Protocol Types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct McpContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
    pub data: Option<String>,
    pub mime_type: Option<String>,
    pub uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    pub is_error: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct McpResourceContent {
    pub uri: String,
    pub mime_type: Option<String>,
    pub text: Option<String>,
    pub blob: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct McpResourceResult {
    pub contents: Vec<McpResourceContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct McpPromptMessage {
    pub role: String,
    pub content: McpContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct McpPromptResult {
    pub description: Option<String>,
    pub messages: Vec<McpPromptMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct ToolSchema {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: String, // JSON string of JSON Schema
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct ResourceSchema {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct PromptSchema {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Option<String>, // JSON string of argument definitions
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct ServerOptions {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub log_level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct ListenOptions {
    pub transport: String,       // "stdio" | "sse" | "http"
    pub port: Option<u32>,
    pub host: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<String>,
    pub method: String,
    pub params: Option<String>, // JSON string
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<String>,
    pub result: Option<String>, // JSON string
    pub error: Option<String>,  // JSON string
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct PluginOptions {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[napi(object)]
pub struct HookContext {
    pub request_id: String,
    pub method: String,
    pub timestamp: f64,
}

// ─── Internal Registry ────────────────────────────────────────────────────────

static TOOL_REGISTRY: Lazy<DashMap<String, ToolSchema>> = Lazy::new(DashMap::new);
static RESOURCE_REGISTRY: Lazy<DashMap<String, ResourceSchema>> = Lazy::new(DashMap::new);
static PROMPT_REGISTRY: Lazy<DashMap<String, PromptSchema>> = Lazy::new(DashMap::new);

// ─── McpServer NAPI Class ─────────────────────────────────────────────────────

#[napi]
pub struct McpServer {
    name: String,
    version: String,
    description: Option<String>,
    tool_handlers: Arc<DashMap<String, ThreadsafeFunction<String>>>,
    resource_handlers: Arc<DashMap<String, ThreadsafeFunction<String>>>,
    prompt_handlers: Arc<DashMap<String, ThreadsafeFunction<String>>>,
    pre_hooks: Arc<RwLock<Vec<ThreadsafeFunction<String>>>>,
    post_hooks: Arc<RwLock<Vec<ThreadsafeFunction<String>>>>,
    error_hooks: Arc<RwLock<Vec<ThreadsafeFunction<String>>>>,
    plugins: Arc<RwLock<Vec<String>>>,
    decorators: Arc<DashMap<String, String>>,
}

#[napi]
impl McpServer {
    /// Create a new MCP server:
    #[napi(constructor)]
    pub fn new(options: ServerOptions) -> Self {
        let log_level = options.log_level.unwrap_or_else(|| "info".to_string());

        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::new(format!("mcpjs={}", log_level))
            )
            .try_init()
            .ok();

        tracing::info!(
            "🚀 mcpjs v{} — {} initializing",
            env!("CARGO_PKG_VERSION"),
            options.name
        );

        McpServer {
            name: options.name,
            version: options.version,
            description: options.description,
            tool_handlers: Arc::new(DashMap::new()),
            resource_handlers: Arc::new(DashMap::new()),
            prompt_handlers: Arc::new(DashMap::new()),
            pre_hooks: Arc::new(RwLock::new(Vec::new())),
            post_hooks: Arc::new(RwLock::new(Vec::new())),
            error_hooks: Arc::new(RwLock::new(Vec::new())),
            plugins: Arc::new(RwLock::new(Vec::new())),
            decorators: Arc::new(DashMap::new()),
        }
    }

    /// Register a tool:
    #[napi]
    pub fn tool(
        &self,
        name: String,
        schema: ToolSchema,
        handler: ThreadsafeFunction<String>,
    ) -> &Self {
        tracing::info!("🔧 Registering tool: {}", name);

        TOOL_REGISTRY.insert(name.clone(), schema);
        self.tool_handlers.insert(name, handler);
        self
    }

    /// Register a resource:
    #[napi]
    pub fn resource(
        &self,
        name: String,
        uri_pattern: String,
        schema: ResourceSchema,
        handler: ThreadsafeFunction<String>,
    ) -> &Self {
        tracing::info!("📦 Registering resource: {} ({})", name, uri_pattern);

        RESOURCE_REGISTRY.insert(uri_pattern.clone(), schema);
        self.resource_handlers.insert(uri_pattern, handler);
        self
    }

    /// Register a prompt template
    #[napi]
    pub fn prompt(
        &self,
        name: String,
        schema: PromptSchema,
        handler: ThreadsafeFunction<String>,
    ) -> &Self {
        tracing::info!("💬 Registering prompt: {}", name);

        PROMPT_REGISTRY.insert(name.clone(), schema);
        self.prompt_handlers.insert(name, handler);
        self
    }

    /// Register a plugin :
    #[napi]
    pub fn register(&self, plugin_name: String, options: Option<PluginOptions>) -> &Self {
        let opts_json = match options {
            Some(o) => serde_json::to_string(&o).unwrap_or_default(),
            None => "{}".to_string(),
        };

        tracing::info!("🔌 Registering plugin: {}", plugin_name);

        let plugins = Arc::clone(&self.plugins);
        tokio::spawn(async move {
            plugins.write().await.push(format!("{}:{}", plugin_name, opts_json));
        });

        self
    }

    /// Add a pre-request lifecycle hook :
    #[napi]
    pub fn add_hook_pre(&self, hook: ThreadsafeFunction<String>) -> &Self {
        let hooks = Arc::clone(&self.pre_hooks);
        tokio::spawn(async move {
            hooks.write().await.push(hook);
        });
        self
    }

    /// Add a post-response lifecycle hook
    #[napi]
    pub fn add_hook_post(&self, hook: ThreadsafeFunction<String>) -> &Self {
        let hooks = Arc::clone(&self.post_hooks);
        tokio::spawn(async move {
            hooks.write().await.push(hook);
        });
        self
    }

    /// Add an error lifecycle hook :
    #[napi]
    pub fn add_hook_error(&self, hook: ThreadsafeFunction<String>) -> &Self {
        let hooks = Arc::clone(&self.error_hooks);
        tokio::spawn(async move {
            hooks.write().await.push(hook);
        });
        self
    }

    /// Decorate the server with custom values:
    #[napi]
    pub fn decorate(&self, key: String, value: String) -> &Self {
        tracing::debug!("🎨 Adding decorator: {}", key);
        self.decorators.insert(key, value);
        self
    }

    /// Get a decorator value
    #[napi]
    pub fn get_decorator(&self, key: String) -> Option<String> {
        self.decorators.get(&key).map(|v| v.clone())
    }

    /// Handle an incoming JSON-RPC request (the core dispatch loop)
    #[napi]
    pub async fn handle_request(&self, raw_request: String) -> napi::Result<String> {
        let request: serde_json::Value = serde_json::from_str(&raw_request)
            .map_err(|e| napi::Error::from_reason(format!("Invalid JSON-RPC: {}", e)))?;

        let request_id = request["id"].as_str().map(|s| s.to_string())
            .or_else(|| request["id"].as_u64().map(|n| n.to_string()));
        let method = request["method"].as_str().unwrap_or("").to_string();
        let params = request.get("params").cloned().unwrap_or(serde_json::Value::Null);

        tracing::debug!("📨 Handling: {} (id: {:?})", method, request_id);

        let result = match method.as_str() {
            "initialize" => self.handle_initialize(&params),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tool_call(&params).await,
            "resources/list" => self.handle_resources_list(),
            "resources/read" => self.handle_resource_read(&params).await,
            "prompts/list" => self.handle_prompts_list(),
            "prompts/get" => self.handle_prompt_get(&params).await,
            "ping" => Ok(serde_json::json!({ "status": "pong" })),
            _ => Err(format!("Method not found: {}", method)),
        };

        let response = match result {
            Ok(value) => serde_json::json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": value,
            }),
            Err(err) => serde_json::json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {
                    "code": -32601,
                    "message": err
                }
            }),
        };

        Ok(serde_json::to_string(&response).unwrap_or_default())
    }

    fn handle_initialize(&self, _params: &serde_json::Value) -> std::result::Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {
                "name": self.name,
                "version": self.version,
                "description": self.description,
            },
            "capabilities": {
                "tools": { "listChanged": true },
                "resources": { "listChanged": true, "subscribe": false },
                "prompts": { "listChanged": true },
                "logging": {}
            }
        }))
    }

    fn handle_tools_list(&self) -> std::result::Result<serde_json::Value, String> {
        let tools: Vec<serde_json::Value> = TOOL_REGISTRY.iter().map(|entry| {
            let schema = entry.value();
            let input_schema: serde_json::Value = serde_json::from_str(&schema.input_schema)
                .unwrap_or_else(|_| serde_json::json!({ "type": "object" }));
            serde_json::json!({
                "name": schema.name,
                "description": schema.description,
                "inputSchema": input_schema,
            })
        }).collect();

        Ok(serde_json::json!({ "tools": tools }))
    }

    async fn handle_tool_call(&self, params: &serde_json::Value) -> std::result::Result<serde_json::Value, String> {
        let tool_name = params["name"].as_str()
            .ok_or("Missing tool name")?;
        let args = params.get("arguments").cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let handler = self.tool_handlers.get(tool_name)
            .ok_or_else(|| format!("Tool not found: {}", tool_name))?;

        let args_str = serde_json::to_string(&args).map_err(|e| e.to_string())?;

        let result_str: String = handler
            .call_async::<String>(Ok(args_str))
            .await
            .map_err(|e| e.to_string())?;

        let result: serde_json::Value = serde_json::from_str(&result_str)
            .unwrap_or_else(|_| serde_json::json!({
                "content": [{ "type": "text", "text": result_str }]
            }));

        Ok(result)
    }

    fn handle_resources_list(&self) -> std::result::Result<serde_json::Value, String> {
        let resources: Vec<serde_json::Value> = RESOURCE_REGISTRY.iter().map(|entry| {
            let schema = entry.value();
            serde_json::json!({
                "uri": schema.uri,
                "name": schema.name,
                "description": schema.description,
                "mimeType": schema.mime_type,
            })
        }).collect();

        Ok(serde_json::json!({ "resources": resources }))
    }

    async fn handle_resource_read(&self, params: &serde_json::Value) -> std::result::Result<serde_json::Value, String> {
        let uri = params["uri"].as_str().ok_or("Missing URI")?;

        // Find matching handler by URI pattern
        let handler_entry = self.resource_handlers.iter()
            .find(|entry| {
                let pattern = entry.key();
                uri_matches(pattern, uri)
            });

        let handler = handler_entry
            .ok_or_else(|| format!("No resource handler for URI: {}", uri))?;

        let uri_string = uri.to_string();
        let result_str: String = handler
            .value()
            .call_async::<String>(Ok(uri_string))
            .await
            .map_err(|e| e.to_string())?;

        let result: serde_json::Value = serde_json::from_str(&result_str)
            .unwrap_or_else(|_| serde_json::json!({
                "contents": [{ "uri": uri, "text": result_str }]
            }));

        Ok(result)
    }

    fn handle_prompts_list(&self) -> std::result::Result<serde_json::Value, String> {
        let prompts: Vec<serde_json::Value> = PROMPT_REGISTRY.iter().map(|entry| {
            let schema = entry.value();
            let args: serde_json::Value = schema.arguments.as_ref()
                .and_then(|a| serde_json::from_str(a).ok())
                .unwrap_or(serde_json::Value::Array(vec![]));
            serde_json::json!({
                "name": schema.name,
                "description": schema.description,
                "arguments": args,
            })
        }).collect();

        Ok(serde_json::json!({ "prompts": prompts }))
    }

    async fn handle_prompt_get(&self, params: &serde_json::Value) -> std::result::Result<serde_json::Value, String> {
        let name = params["name"].as_str().ok_or("Missing prompt name")?;
        let args = params.get("arguments").cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let handler = self.prompt_handlers.get(name)
            .ok_or_else(|| format!("Prompt not found: {}", name))?;

        let args_str = serde_json::to_string(&args).map_err(|e| e.to_string())?;

        let result_str: String = handler
            .value()
            .call_async::<String>(Ok(args_str))
            .await
            .map_err(|e| e.to_string())?;

        let result: serde_json::Value = serde_json::from_str(&result_str)
            .map_err(|e| e.to_string())?;

        Ok(result)
    }

    /// List all registered tools
    #[napi]
    pub fn list_tools(&self) -> Vec<String> {
        TOOL_REGISTRY.iter().map(|e| e.key().clone()).collect()
    }

    /// List all registered resources
    #[napi]
    pub fn list_resources(&self) -> Vec<String> {
        RESOURCE_REGISTRY.iter().map(|e| e.key().clone()).collect()
    }

    /// List all registered prompts
    #[napi]
    pub fn list_prompts(&self) -> Vec<String> {
        PROMPT_REGISTRY.iter().map(|e| e.key().clone()).collect()
    }

    /// Get server info
    #[napi]
    pub fn server_info(&self) -> String {
        serde_json::to_string(&serde_json::json!({
            "name": self.name,
            "version": self.version,
            "description": self.description,
            "mcpjsVersion": env!("CARGO_PKG_VERSION"),
            "tools": self.list_tools(),
            "resources": self.list_resources(),
            "prompts": self.list_prompts(),
        })).unwrap_or_default()
    }

    /// Generate a unique request ID (for internal use)
    #[napi]
    pub fn generate_id(&self) -> String {
        Uuid::new_v4().to_string()
    }
}

// ─── Utilities ────────────────────────────────────────────────────────────────

/// Match URI against a pattern (supports glob-like wildcards)
fn uri_matches(pattern: &str, uri: &str) -> bool {
    if pattern == uri {
        return true;
    }
    if pattern.ends_with("/**") {
        let prefix = &pattern[..pattern.len() - 3];
        return uri.starts_with(prefix);
    }
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 2];
        let rest = &uri[prefix.len()..];
        return uri.starts_with(prefix) && !rest.contains('/');
    }
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        let mut pos = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() { continue; }
            if i == 0 {
                if !uri.starts_with(part) { return false; }
                pos = part.len();
            } else {
                if let Some(found) = uri[pos..].find(part) {
                    pos += found + part.len();
                } else {
                    return false;
                }
            }
        }
        return true;
    }
    false
}

// ─── Standalone helpers exported to JS ───────────────────────────────────────

/// Create a text content block
#[napi]
pub fn text_content(text: String) -> McpContent {
    McpContent {
        content_type: "text".to_string(),
        text: Some(text),
        data: None,
        mime_type: None,
        uri: None,
    }
}

/// Create an image content block
#[napi]
pub fn image_content(data: String, mime_type: String) -> McpContent {
    McpContent {
        content_type: "image".to_string(),
        text: None,
        data: Some(data),
        mime_type: Some(mime_type),
        uri: None,
    }
}

/// Create a resource reference content block
#[napi]
pub fn resource_content(uri: String, mime_type: Option<String>) -> McpContent {
    McpContent {
        content_type: "resource".to_string(),
        text: None,
        data: None,
        mime_type,
        uri: Some(uri),
    }
}

/// Validate JSON Schema (returns error message or null)
#[napi]
pub fn validate_schema(schema_json: String, data_json: String) -> Option<String> {
    let schema: serde_json::Value = match serde_json::from_str(&schema_json) {
        Ok(v) => v,
        Err(e) => return Some(format!("Invalid schema: {}", e)),
    };
    let data: serde_json::Value = match serde_json::from_str(&data_json) {
        Ok(v) => v,
        Err(e) => return Some(format!("Invalid data: {}", e)),
    };

    let compiled = match jsonschema::JSONSchema::compile(&schema) {
        Ok(compiled) => compiled,
        Err(e) => return Some(format!("Invalid schema: {}", e)),
    };

    let validation_result = match compiled.validate(&data) {
        Ok(()) => None,
        Err(mut errors) => errors.next().map(|e| e.to_string()),
    };

    validation_result
}

/// Parse a JSON-RPC request from raw string
#[napi]
pub fn parse_jsonrpc(raw: String) -> napi::Result<JsonRpcRequest> {
    let v: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| napi::Error::from_reason(e.to_string()))?;

    Ok(JsonRpcRequest {
        jsonrpc: v["jsonrpc"].as_str().unwrap_or("2.0").to_string(),
        id: v["id"].as_str().map(|s| s.to_string())
            .or_else(|| v["id"].as_u64().map(|n| n.to_string())),
        method: v["method"].as_str().unwrap_or("").to_string(),
        params: v.get("params").map(|p| serde_json::to_string(p).unwrap_or_default()),
    })
}

/// Serialize a JSON-RPC success response
#[napi]
pub fn jsonrpc_ok(id: Option<String>, result_json: String) -> String {
    let result: serde_json::Value = serde_json::from_str(&result_json)
        .unwrap_or(serde_json::Value::Null);
    serde_json::to_string(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })).unwrap_or_default()
}

/// Serialize a JSON-RPC error response
#[napi]
pub fn jsonrpc_error(id: Option<String>, code: i32, message: String) -> String {
    serde_json::to_string(&serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })).unwrap_or_default()
}

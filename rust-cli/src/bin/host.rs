//! Native messaging host for Chrome Agent Bridge
//!
//! This binary replaces the Node.js host.js. It:
//! 1. Runs a WebSocket server on ws://127.0.0.1:8766
//! 2. Communicates with Chrome via native messaging (stdin/stdout)
//! 3. Routes messages between WebSocket clients and the browser extension

use std::collections::HashMap;
use std::env;
use std::io::{self, Read, Write as IoWrite};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Session state for a WebSocket client connection.
/// Each connection gets its own session with an isolated active tab.
struct ClientSession {
    id: String,
    active_tab_id: Option<i64>,
    forks: HashMap<String, i64>, // fork_name -> tabId
}

use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, RwLock};
use tokio::time::timeout;
use tokio_tungstenite::accept_async_with_config;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;

/// Environment variable configuration
fn ws_host() -> String {
    env::var("FAB_WS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}

fn ws_port() -> u16 {
    env::var("FAB_WS_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8766)
}

fn request_timeout_ms() -> u64 {
    env::var("FAB_REQUEST_TIMEOUT_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30000)
}

 
/// Log to stderr (native messaging uses stdout for messages)
macro_rules! log {
    ($($arg:tt)*) => {
        eprintln!("[chrome-agent-bridge] {}", format!($($arg)*));
    };
}

/// Request counter for generating unique IDs
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_id() -> String {
    let count = REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("req_{}_{}", now, count)
}

/// Pending request tracking
struct PendingRequest {
    response_tx: mpsc::Sender<Value>,
    started: Instant,
    profile: bool,
}

type PendingMap = Arc<RwLock<HashMap<String, PendingRequest>>>;

/// Channel for sending messages to native messaging (stdout)
type NativeTx = mpsc::Sender<Value>;

/// Get MIME type from file extension
fn get_mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(|s| s.to_lowercase()).as_deref() {
        Some("zip") => "application/zip",
        Some("xpi") => "application/x-xpinstall",
        Some("json") => "application/json",
        Some("js") => "application/javascript",
        Some("html") | Some("htm") => "text/html",
        Some("css") => "text/css",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("pdf") => "application/pdf",
        Some("txt") => "text/plain",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

/// Process uploadFile action - read file and convert to fillForm with base64 data
/// Max base64 data size per native messaging message (~750KB to stay under Chrome's 1MB limit)
const CHUNK_SIZE: usize = 750_000;

fn process_upload_file(message: &mut Value) -> Result<(), String> {
    let params = message.get_mut("params").ok_or("Missing params")?;
    let file_path = params.get("filePath")
        .and_then(|v| v.as_str())
        .ok_or("Missing filePath")?
        .to_string();
    let selector = params.get("selector")
        .and_then(|v| v.as_str())
        .ok_or("Missing selector")?
        .to_string();

    let path = Path::new(&file_path);
    let file_data = std::fs::read(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    let base64_data = base64::engine::general_purpose::STANDARD.encode(&file_data);
    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();
    let mime_type = get_mime_type(path);

    // Convert to fillForm action
    message["action"] = json!("fillForm");
    message["params"] = json!({
        "fields": [{
            "selector": selector,
            "file": {
                "name": file_name,
                "type": mime_type,
                "data": base64_data
            }
        }]
    });

    Ok(())
}

/// Check if a message payload is too large for native messaging and needs chunking
fn needs_chunking(message: &Value) -> bool {
    let serialized = message.to_string();
    serialized.len() > CHUNK_SIZE
}

/// Send a large message in chunks via native messaging, then send the action with a reassembly reference
async fn send_chunked_file(
    message: &mut Value,
    native_tx: &NativeTx,
) -> Result<(), String> {
    let id = message.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let action = message.get("action").and_then(|v| v.as_str()).unwrap_or("").to_string();

    // Extract file data from the message (works for fillForm and dropFile)
    let (base64_data, file_name, mime_type, selector) = if action == "fillForm" {
        let fields = message.get("params")
            .and_then(|p| p.get("fields"))
            .and_then(|f| f.as_array())
            .ok_or("Missing fields")?;
        let field = fields.first().ok_or("Empty fields")?;
        let file = field.get("file").ok_or("No file in field")?;
        let data = file.get("data").and_then(|v| v.as_str()).ok_or("No data")?.to_string();
        let name = file.get("name").and_then(|v| v.as_str()).unwrap_or("file").to_string();
        let mime = file.get("type").and_then(|v| v.as_str()).unwrap_or("application/octet-stream").to_string();
        let sel = field.get("selector").and_then(|v| v.as_str()).unwrap_or("").to_string();
        (data, name, mime, sel)
    } else if action == "dropFile" {
        let params = message.get("params").ok_or("Missing params")?;
        let data = params.get("data").and_then(|v| v.as_str()).ok_or("No data")?.to_string();
        let name = params.get("fileName").and_then(|v| v.as_str()).unwrap_or("file").to_string();
        let mime = params.get("mimeType").and_then(|v| v.as_str()).unwrap_or("application/octet-stream").to_string();
        let sel = params.get("selector").and_then(|v| v.as_str()).unwrap_or("").to_string();
        (data, name, mime, sel)
    } else {
        return Err("Unsupported chunked action".to_string());
    };

    let transfer_id = format!("chunk_{}", id);
    let total_chunks = (base64_data.len() + CHUNK_SIZE - 1) / CHUNK_SIZE;

    log!("Chunking file {} ({} bytes base64) into {} chunks", file_name, base64_data.len(), total_chunks);

    // Send chunk_start
    let start_msg = json!({
        "type": "chunk_start",
        "transferId": transfer_id,
        "fileName": file_name,
        "mimeType": mime_type,
        "totalSize": base64_data.len(),
        "totalChunks": total_chunks
    });
    native_tx.send(start_msg).await.map_err(|e| format!("Failed to send chunk_start: {}", e))?;

    // Send chunks
    for i in 0..total_chunks {
        let start = i * CHUNK_SIZE;
        let end = std::cmp::min(start + CHUNK_SIZE, base64_data.len());
        let chunk_data = &base64_data[start..end];

        let chunk_msg = json!({
            "type": "chunk_data",
            "transferId": transfer_id,
            "chunkIndex": i,
            "data": chunk_data
        });
        native_tx.send(chunk_msg).await.map_err(|e| format!("Failed to send chunk {}: {}", i, e))?;
    }

    // Send the actual action with a reference to the chunked transfer
    if action == "fillForm" {
        message["params"] = json!({
            "fields": [{
                "selector": selector,
                "file": {
                    "name": file_name,
                    "type": mime_type,
                    "chunkedTransfer": transfer_id
                }
            }]
        });
    } else if action == "dropFile" {
        let params = message.get_mut("params").ok_or("Missing params")?;
        if let Some(obj) = params.as_object_mut() {
            obj.remove("data");
            obj.insert("chunkedTransfer".to_string(), json!(transfer_id));
        }
    }

    Ok(())
}

/// Process fillForm with filePath in fields - read files before sending
fn process_fill_form_files(message: &mut Value) -> Result<(), String> {
    let params = match message.get_mut("params") {
        Some(p) => p,
        None => return Ok(()),
    };

    let fields = match params.get_mut("fields") {
        Some(Value::Array(f)) => f,
        _ => return Ok(()),
    };

    for field in fields.iter_mut() {
        if let Some(file_path) = field.get("filePath").and_then(|v| v.as_str()).map(|s| s.to_string()) {
            let path = Path::new(&file_path);
            let file_data = std::fs::read(path)
                .map_err(|e| format!("Failed to read file: {}", e))?;
            let base64_data = base64::engine::general_purpose::STANDARD.encode(&file_data);
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file")
                .to_string();
            let mime_type = get_mime_type(path);

            field["file"] = json!({
                "name": file_name,
                "type": mime_type,
                "data": base64_data
            });

            // Remove filePath
            if let Value::Object(ref mut obj) = field {
                obj.remove("filePath");
            }
        }
    }

    Ok(())
}

/// Actions that need an active tab to operate on
fn action_needs_tab(action: &str) -> bool {
    matches!(
        action,
        "navigate" | "click" | "type" | "fillForm" | "getContent"
            | "getInteractables" | "preexplore" | "screenshot" | "scroll"
            | "evaluate" | "waitFor" | "tryUntil" | "uploadFile" | "dropFile"
            | "getAuthContext" | "requestAuth" | "secureAutoFill" | "listFrames"
    )
}

/// Handle a WebSocket client connection
async fn handle_ws_client(
    stream: tokio::net::TcpStream,
    native_tx: NativeTx,
    pending: PendingMap,

) {
    let session_id = next_id().replace("req_", "sess_");
    let mut session = ClientSession {
        id: session_id.clone(),
        active_tab_id: None,
        forks: HashMap::new(),
    };
    log!("Session {} connected", session.id);

    let ws_config = WebSocketConfig {
        max_message_size: Some(128 * 1024 * 1024),
        max_frame_size: Some(64 * 1024 * 1024),
        ..Default::default()
    };
    let ws_stream = match accept_async_with_config(stream, Some(ws_config)).await {
        Ok(ws) => ws,
        Err(e) => {
            log!("WebSocket handshake error: {}", e);
            return;
        }
    };

    let (mut write, mut read) = ws_stream.split();

    // Send ready message with session ID
    let ready_msg = json!({
        "type": "ready",
        "host": ws_host(),
        "port": ws_port(),
        "sessionId": session.id
    });
    if let Err(e) = write.send(Message::Text(ready_msg.to_string())).await {
        log!("Failed to send ready message: {}", e);
        return;
    }

    while let Some(msg) = read.next().await {
        let msg = match msg {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) => break,
            Ok(_) => continue,
            Err(e) => {
                log!("WebSocket read error: {}", e);
                break;
            }
        };

        let mut message: Value = match serde_json::from_str(&msg) {
            Ok(m) => m,
            Err(_) => {
                let error_msg = json!({"ok": false, "error": "Invalid JSON"});
                let _ = write.send(Message::Text(error_msg.to_string())).await;
                continue;
            }
        };

        // Handle session control messages
        if let Some(msg_type) = message.get("type").and_then(|v| v.as_str()) {
            if msg_type == "session_info" {
                let info = json!({
                    "type": "session_info",
                    "sessionId": session.id,
                    "activeTabId": session.active_tab_id,
                    "forks": session.forks.keys().collect::<Vec<_>>()
                });
                let _ = write.send(Message::Text(info.to_string())).await;
                continue;
            }
        }

        if message.get("action").is_none() {
            let error_msg = json!({"ok": false, "error": "Missing action"});
            let _ = write.send(Message::Text(error_msg.to_string())).await;
            continue;
        }

        let id = match message.get("id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => {
                let id = next_id();
                message["id"] = json!(id);
                id
            }
        };

        let action = message.get("action").and_then(|v| v.as_str()).unwrap_or("").to_string();

        // Session-scoped tab injection: if the message doesn't have an explicit
        // tabId and the action needs a tab, inject the session's active tab.
        if action_needs_tab(&action) {
            let has_explicit_tab = message.get("params")
                .and_then(|p| p.get("tabId"))
                .and_then(|v| v.as_i64())
                .is_some();
            if !has_explicit_tab {
                if let Some(tab_id) = session.active_tab_id {
                    if let Some(params) = message.get_mut("params") {
                        params["tabId"] = json!(tab_id);
                    } else {
                        message["params"] = json!({"tabId": tab_id});
                    }
                }
            }
        }

        // Session-scoped fork resolution: resolve fork names within this session
        if let Some(fork_name) = message.get("params").and_then(|p| p.get("fork")).and_then(|v| v.as_str()).map(|s| s.to_string()) {
            if let Some(&tab_id) = session.forks.get(&fork_name) {
                if let Some(params) = message.get_mut("params") {
                    params["tabId"] = json!(tab_id);
                }
            }
        }

        let profile = message.get("profile").and_then(|v| v.as_bool()).unwrap_or(false)
            || message.get("params")
                .and_then(|p| p.get("profile"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

        let started = Instant::now();

        // Hard policy gate: configureAuth is not allowed via agent-facing API.
        if message.get("action").and_then(|v| v.as_str()) == Some("configureAuth") {
            let error_msg = json!({
                "id": id,
                "ok": false,
                "error": "configureAuth is not supported by this host build."
            });
            let _ = write.send(Message::Text(error_msg.to_string())).await;
            continue;
        }

        // Handle uploadFile action
        if message.get("action").and_then(|v| v.as_str()) == Some("uploadFile") {
            if let Err(e) = process_upload_file(&mut message) {
                let error_msg = json!({"id": id, "ok": false, "error": e});
                let _ = write.send(Message::Text(error_msg.to_string())).await;
                continue;
            }
        }

        // Handle fillForm with filePath in fields
        if message.get("action").and_then(|v| v.as_str()) == Some("fillForm") {
            if let Err(e) = process_fill_form_files(&mut message) {
                let error_msg = json!({"id": id, "ok": false, "error": e});
                let _ = write.send(Message::Text(error_msg.to_string())).await;
                continue;
            }
        }

        // Create response channel for this request
        let (response_tx, mut response_rx) = mpsc::channel::<Value>(1);

        // Register pending request
        {
            let mut pending_guard = pending.write().await;
            pending_guard.insert(id.clone(), PendingRequest {
                response_tx,
                started,
                profile,
            });
        }

        // Send to native messaging (with chunking for large payloads)
        if needs_chunking(&message) {
            if let Err(e) = send_chunked_file(&mut message, &native_tx).await {
                log!("Failed to chunk file: {}", e);
                pending.write().await.remove(&id);
                let error_msg = json!({"id": id, "ok": false, "error": format!("Failed to chunk file: {}", e)});
                let _ = write.send(Message::Text(error_msg.to_string())).await;
                continue;
            }
        }
        if let Err(e) = native_tx.send(message).await {
            log!("Failed to send to native: {}", e);
            pending.write().await.remove(&id);
            let error_msg = json!({"id": id, "ok": false, "error": "Failed to send to browser"});
            let _ = write.send(Message::Text(error_msg.to_string())).await;
            continue;
        }

        // Wait for response with timeout
        let timeout_ms = request_timeout_ms();
        let response = match timeout(Duration::from_millis(timeout_ms), response_rx.recv()).await {
            Ok(Some(resp)) => resp,
            Ok(None) => {
                pending.write().await.remove(&id);
                let mut error_msg = json!({"id": id, "ok": false, "error": "Request channel closed"});
                if profile {
                    let host_ms = started.elapsed().as_secs_f64() * 1000.0;
                    error_msg["timing"] = json!({"hostMs": (host_ms * 100.0).round() / 100.0});
                }
                let _ = write.send(Message::Text(error_msg.to_string())).await;
                continue;
            }
            Err(_) => {
                pending.write().await.remove(&id);
                let mut error_msg = json!({"id": id, "ok": false, "error": "Request timed out"});
                if profile {
                    let host_ms = started.elapsed().as_secs_f64() * 1000.0;
                    error_msg["timing"] = json!({"hostMs": (host_ms * 100.0).round() / 100.0});
                }
                let _ = write.send(Message::Text(error_msg.to_string())).await;
                continue;
            }
        };

        // Update session state from responses
        let resp_ok = response.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if resp_ok {
            if let Some(result) = response.get("result") {
                // Track tabId from navigate, newSession, setActiveTab responses
                if matches!(action.as_str(), "navigate" | "newSession" | "setActiveTab") {
                    if let Some(tab_id) = result.get("tabId").and_then(|v| v.as_i64()) {
                        session.active_tab_id = Some(tab_id);
                    }
                }
                // Track forks created by this session
                if action == "fork" {
                    if let Some(forks) = result.get("forks").and_then(|v| v.as_array()) {
                        for fork in forks {
                            if let (Some(name), Some(tab_id)) = (
                                fork.get("name").and_then(|v| v.as_str()),
                                fork.get("tabId").and_then(|v| v.as_i64()),
                            ) {
                                session.forks.insert(name.to_string(), tab_id);
                            }
                        }
                    }
                }
                // Track fork deletion
                if action == "killFork" {
                    if let Some(killed) = result.get("fork").and_then(|v| v.as_str()) {
                        session.forks.remove(killed);
                    }
                }
            }
        }

        // Send response back to client
        if let Err(e) = write.send(Message::Text(response.to_string())).await {
            log!("Failed to send response: {}", e);
            break;
        }
    }

    log!("Session {} disconnected (tab: {:?})", session.id, session.active_tab_id);
}

/// Read native messaging input from stdin (blocking, runs in separate thread)
fn read_native_stdin(tx: mpsc::Sender<Value>) {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let mut len_buf = [0u8; 4];

    loop {
        // Read 4-byte length prefix (little-endian)
        if stdin.read_exact(&mut len_buf).is_err() {
            log!("Native messaging stream ended");
            break;
        }

        let len = u32::from_le_bytes(len_buf) as usize;
        if len == 0 || len > 100 * 1024 * 1024 {
            log!("Invalid message length: {}", len);
            continue;
        }

        // Read message body
        let mut msg_buf = vec![0u8; len];
        if stdin.read_exact(&mut msg_buf).is_err() {
            log!("Failed to read message body");
            continue;
        }

        // Parse JSON
        let message: Value = match serde_json::from_slice(&msg_buf) {
            Ok(m) => m,
            Err(e) => {
                log!("Failed to parse native message: {}", e);
                continue;
            }
        };

        // Send to async handler
        if tx.blocking_send(message).is_err() {
            log!("Failed to send native message to handler");
            break;
        }
    }
}

/// Write native messaging output to stdout
fn write_native_stdout(message: &Value) {
    let payload = message.to_string();
    let payload_bytes = payload.as_bytes();
    let len = payload_bytes.len() as u32;
    let len_bytes = len.to_le_bytes();

    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    if stdout.write_all(&len_bytes).is_err() {
        log!("Failed to write message length");
        return;
    }
    if stdout.write_all(payload_bytes).is_err() {
        log!("Failed to write message body");
        return;
    }
    if stdout.flush().is_err() {
        log!("Failed to flush stdout");
    }
}

/// Process messages from the browser extension
async fn handle_native_messages(
    mut native_rx: mpsc::Receiver<Value>,
    pending: PendingMap,
) {
    while let Some(mut message) = native_rx.recv().await {
        // Check if this is a response to a pending request
        if let Some(id) = message.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()) {
            let entry = pending.write().await.remove(&id);
            if let Some(req) = entry {
                // Add timing if profiling
                if req.profile {
                    let host_ms = req.started.elapsed().as_secs_f64() * 1000.0;
                    let timing = message.get("timing")
                        .and_then(|t| t.as_object())
                        .cloned()
                        .unwrap_or_default();
                    let mut timing_obj = timing;
                    timing_obj.insert("hostMs".to_string(), json!((host_ms * 100.0).round() / 100.0));
                    message["timing"] = json!(timing_obj);
                }

                // Send response back to the WebSocket client
                let _ = req.response_tx.send(message).await;
                continue;
            }
        }

        // Not a response - this is an event, broadcast to all clients
        // (For now we just log it, as we don't track all connected clients for broadcasting)
        log!("Received event from browser: {}", message);
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    let host = ws_host();
    let port = ws_port();
    let addr = format!("{}:{}", host, port);

    // Create pending request map
    let pending: PendingMap = Arc::new(RwLock::new(HashMap::new()));

    // Create channel for outgoing native messages
    let (native_out_tx, mut native_out_rx) = mpsc::channel::<Value>(100);

    // Create channel for incoming native messages
    let (native_in_tx, native_in_rx) = mpsc::channel::<Value>(100);

    // Spawn thread for reading native stdin (blocking I/O)
    let stdin_tx = native_in_tx.clone();
    std::thread::spawn(move || {
        read_native_stdin(stdin_tx);
        std::process::exit(0);
    });

    // Spawn task for writing to native stdout
    tokio::spawn(async move {
        while let Some(message) = native_out_rx.recv().await {
            write_native_stdout(&message);
        }
    });

    // Spawn task for handling incoming native messages
    let pending_clone = pending.clone();
    tokio::spawn(async move {
        handle_native_messages(native_in_rx, pending_clone).await;
    });

    // Start WebSocket server
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            log!("Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    log!("WebSocket server listening on ws://{}", addr);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let native_tx = native_out_tx.clone();
                let pending_clone = pending.clone();
                tokio::spawn(async move {
                    handle_ws_client(stream, native_tx, pending_clone).await;
                });
            }
            Err(e) => {
                log!("Failed to accept connection: {}", e);
            }
        }
    }
}

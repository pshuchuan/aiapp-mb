//! HostApi: platform-specific host capabilities exposed to the renderer.
//!
//! This is the "JS Bridge" abstraction — the set of native capabilities that the platform shell
//! provides to the rendering layer. It mirrors the `Host` trait in `aiapp-engine` but is
//! designed for the client-side rendering context (WebView / native renderer), not the server-side
//! Wasmtime execution context.
//!
//! Each platform shell implements this trait and injects it into the renderer.

use std::sync::Arc;

/// Host capability result.
pub type HostResult<T> = Result<T, HostError>;

/// Host capability error.
#[derive(Debug, Clone)]
pub enum HostError {
    /// Permission not granted.
    PermissionDenied(String),
    /// Capability not supported on this platform.
    Unsupported(String),
    /// Runtime error.
    Runtime(String),
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostError::PermissionDenied(p) => write!(f, "permission denied: {p}"),
            HostError::Unsupported(c) => write!(f, "capability not supported: {c}"),
            HostError::Runtime(e) => write!(f, "runtime error: {e}"),
        }
    }
}

/// Host capabilities provided by the platform shell to the renderer.
///
/// This is the "JS Bridge" API surface. Each method maps to a native capability
/// that the platform can provide. The renderer (WebView or native) calls these
/// methods to interact with the device.
pub trait HostApi: Send + Sync {
    /// ---- Storage ----
    /// Save a value by key.
    fn storage_set(&self, key: &str, value: &[u8]) -> HostResult<()>;
    /// Load a value by key.
    fn storage_get(&self, key: &str) -> HostResult<Option<Vec<u8>>>;
    /// Delete a value by key.
    fn storage_delete(&self, key: &str) -> HostResult<()>;

    /// ---- Notifications ----
    /// Show a system notification.
    fn show_notification(&self, title: &str, body: &str) -> HostResult<()>;

    /// ---- Logging ----
    /// Write to the platform log.
    fn log(&self, level: &str, message: &str);

    /// ---- Network (optional) ----
    /// Make an HTTP request.
    fn http_request(
        &self,
        url: &str,
        method: &str,
        headers: &[(String, String)],
        body: Option<&[u8]>,
    ) -> HostResult<(u16, Vec<u8>)>;

    /// ---- Location (optional) ----
    /// Get the current location.
    fn get_location(&self) -> HostResult<(f64, f64)>;

    /// ---- Platform info ----
    /// Returns the platform identifier (e.g., "web", "ios", "android", "desktop", "tv", "car").
    fn platform(&self) -> &str;

    /// Returns the renderer mode ("webview" or "native").
    fn renderer_mode(&self) -> &str;
}

/// A default in-memory HostApi implementation for testing and preview.
pub struct MemoryHostApi {
    store: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
    platform: String,
    renderer_mode: String,
}

impl MemoryHostApi {
    pub fn new(platform: &str, renderer_mode: &str) -> Self {
        MemoryHostApi {
            store: std::sync::Mutex::new(std::collections::HashMap::new()),
            platform: platform.to_string(),
            renderer_mode: renderer_mode.to_string(),
        }
    }
}

impl Default for MemoryHostApi {
    fn default() -> Self {
        Self::new("web", "webview")
    }
}

impl HostApi for MemoryHostApi {
    fn storage_set(&self, key: &str, value: &[u8]) -> HostResult<()> {
        self.store
            .lock()
            .map_err(|e| HostError::Runtime(e.to_string()))?
            .insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn storage_get(&self, key: &str) -> HostResult<Option<Vec<u8>>> {
        Ok(self
            .store
            .lock()
            .map_err(|e| HostError::Runtime(e.to_string()))?
            .get(key)
            .cloned())
    }

    fn storage_delete(&self, key: &str) -> HostResult<()> {
        self.store
            .lock()
            .map_err(|e| HostError::Runtime(e.to_string()))?
            .remove(key);
        Ok(())
    }

    fn show_notification(&self, title: &str, body: &str) -> HostResult<()> {
        println!("[notification] {title}: {body}");
        Ok(())
    }

    fn log(&self, level: &str, message: &str) {
        println!("[{level}] {message}");
    }

    fn http_request(
        &self,
        _url: &str,
        _method: &str,
        _headers: &[(String, String)],
        _body: Option<&[u8]>,
    ) -> HostResult<(u16, Vec<u8>)> {
        Err(HostError::Unsupported("http_request".into()))
    }

    fn get_location(&self) -> HostResult<(f64, f64)> {
        Err(HostError::Unsupported("get_location".into()))
    }

    fn platform(&self) -> &str {
        &self.platform
    }

    fn renderer_mode(&self) -> &str {
        &self.renderer_mode
    }
}
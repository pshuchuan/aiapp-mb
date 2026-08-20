//! Host capability abstraction: different sides implement the same `Host` trait, so apps never change code.
//!
//! Corresponds to the WIT contract `aiapp:app-host/host` interface. Per-renderer adapters:
//! - Web version: browser Web APIs (Notification / localStorage / IndexedDB / console);
//! - Host app: native capabilities (SQLite / push);
//! - Desktop: system capabilities (system notifications / file system).

use async_trait::async_trait;

/// Host capabilities. Methods are idempotent/re-entrant; host implementations must ensure thread safety themselves.
#[async_trait]
pub trait Host: Send + Sync {
    /// Show a system notification (requires the `notifications` permission).
    async fn show_notification(&self, title: &str, body: &str);

    /// Save data (requires the `storage` permission); returns whether it succeeded.
    async fn save_data(&self, key: &str, value: &[u8]) -> bool;

    /// Load data (requires the `storage` permission).
    async fn load_data(&self, key: &str) -> Option<Vec<u8>>;

    /// In-app logging (no permission needed).
    async fn log(&self, level: &str, message: &str);

    // ---- Native capabilities (progressive enhancement; default to "not supported") ----

    /// Network request (requires the `network` permission).
    async fn http_request(
        &self,
        _url: &str,
        _method: &str,
        _headers: &[String],
        _body: Option<&[u8]>,
    ) -> Result<(u16, Vec<u8>), String> {
        Err("The current host does not support network requests (network)".to_string())
    }

    /// Location (requires the `location` permission).
    async fn get_location(&self) -> Result<(f64, f64), String> {
        Err("The current host does not support location (location)".to_string())
    }

    /// Take a photo / pick from the gallery (requires the `camera` permission).
    async fn take_photo(&self) -> Result<Vec<u8>, String> {
        Err("The current host does not support the camera (camera)".to_string())
    }

    /// Native push token (requires the `push` permission).
    async fn get_push_token(&self) -> Option<String> {
        None
    }
}

/// In-memory host: convenient for tests and preview; data lives only for the current process.
pub struct MemoryHost {
    store: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
    logs: std::sync::Mutex<Vec<(String, String)>>,
}

impl Default for MemoryHost {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryHost {
    pub fn new() -> Self {
        MemoryHost {
            store: std::sync::Mutex::new(std::collections::HashMap::new()),
            logs: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Read the logs written so far.
    pub fn logs(&self) -> Vec<(String, String)> {
        self.logs.lock().unwrap().clone()
    }
}

#[async_trait]
impl Host for MemoryHost {
    async fn show_notification(&self, title: &str, body: &str) {
        println!("[aiapp-host:notification] {title}: {body}");
    }

    async fn save_data(&self, key: &str, value: &[u8]) -> bool {
        self.store
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_vec());
        true
    }

    async fn load_data(&self, key: &str) -> Option<Vec<u8>> {
        self.store.lock().unwrap().get(key).cloned()
    }

    async fn log(&self, level: &str, message: &str) {
        self.logs.lock().unwrap().push((level.to_string(), message.to_string()));
    }
}

//! Unified app package manifest (`aiapp.json`) format.
//!
//! Every `.aiapp` app carries a manifest declaring its metadata, required permissions, host SDK version,
//! etc. It is the "face" of the platform's unified app package format. Fields and evolution follow
//! `docs/ARCHITECTURE.md`.

use serde::{Deserialize, Serialize};

/// Current manifest file version.
pub const MANIFEST_VERSION: u32 = 1;

/// Unified app package manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppManifest {
    /// Manifest format version (currently 1).
    #[serde(default = "default_manifest_version")]
    pub manifest_version: u32,
    /// Unique app id, e.g. `app_12345678`.
    pub app_id: String,
    /// App name.
    pub name: String,
    /// Semantic version number, e.g. `0.1.0`.
    pub version: String,
    /// App description (the user's natural-language description).
    pub description: String,
    /// App category (tool / office / entertainment / game / family / life / industry / other).
    #[serde(default)]
    pub category: String,
    /// Template type (minimal / todo / image-filter / pomodoro / memory-game / tv-movies).
    #[serde(default)]
    pub template: String,
    /// Minimum required host SDK version (WIT contract version, see `wit::WIT_VERSION`).
    pub host_sdk_version: String,
    /// Runtime permission declarations (`storage` / `notifications` / `network`, etc.),
    /// granted on demand by the host, similar to the Android permission model.
    #[serde(default)]
    pub permissions: Vec<String>,
    /// Target platform list (web / android / ios / desktop / tv / car ...).
    #[serde(default)]
    pub platforms: Vec<String>,
    /// WASM entry file name (default `main.wasm`).
    #[serde(default = "default_wasm_entry")]
    pub entry: String,
    /// Creation time (RFC 3339).
    pub created_at: String,
}

fn default_manifest_version() -> u32 {
    MANIFEST_VERSION
}
fn default_wasm_entry() -> String {
    "main.wasm".to_string()
}

/// Declared permission constants (each corresponds to a capability in the WIT interface).
pub mod permissions {
    /// Local data read/write (`save-data` / `load-data`).
    pub const STORAGE: &str = "storage";
    /// System notifications (`show-notification`).
    pub const NOTIFICATIONS: &str = "notifications";
    /// Network access (`http-request`).
    pub const NETWORK: &str = "network";
    /// Location (`get-location`).
    pub const LOCATION: &str = "location";
    /// Camera / gallery (`take-photo`).
    pub const CAMERA: &str = "camera";
    /// Native push (`get-push-token`).
    pub const PUSH: &str = "push";
}

impl AppManifest {
    /// Build a manifest from a description / template / slug.
    pub fn new(desc: &str, template: &str, slug: &str) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let hash = (now as u64).wrapping_mul(2654435761).to_string();
        let app_id = format!("app_{}", &hash[..8.min(hash.len())]);
        Self {
            manifest_version: MANIFEST_VERSION,
            app_id,
            name: slug.replace('_', " "),
            version: "0.1.0".into(),
            description: desc.to_string(),
            category: crate::manifest::default_category(template),
            template: template.to_string(),
            host_sdk_version: format!(">={}", crate::wit::WIT_VERSION),
            permissions: default_permissions(template),
            platforms: vec!["web".into()],
            entry: "main.wasm".into(),
            created_at: now_to_rfc3339(now),
        }
    }

    /// Serialize to a formatted JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("Failed to serialize AppManifest")
    }

    /// Parse from a JSON string.
    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| format!("Manifest parse failed: {e}"))
    }

    /// Whether the given permission is declared.
    pub fn has_permission(&self, perm: &str) -> bool {
        self.permissions.iter().any(|p| p == perm)
    }
}

/// Derive the default category from a template (tool is the general fallback).
pub fn default_category(template: &str) -> String {
    match template {
        "todo" => "tool".to_string(),
        "image-filter" => "tool".to_string(),
        "pomodoro" => "tool".to_string(),
        "memory-game" => "family".to_string(),
        "tv-movies" => "entertainment".to_string(),
        _ => "tool".to_string(),
    }
}

/// Return the default permission declarations for a template.
pub fn default_permissions(template: &str) -> Vec<String> {
    match template {
        // Apps that need to persist data request the storage permission by default
        "todo" | "image-filter" | "pomodoro" | "memory-game" | "tv-movies" => {
            vec![permissions::STORAGE.into()]
        }
        _ => vec![],
    }
}

fn now_to_rfc3339(secs: u64) -> String {
    let days = secs / 86400;
    let remaining = secs % 86400;
    let h = remaining / 3600;
    let m = (remaining % 3600) / 60;
    let s = remaining % 60;
    let (year, month, day) = days_to_date(days as u32);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, h, m, s)
}

/// Convert UNIX epoch days to a calendar date.
fn days_to_date(mut days: u32) -> (u32, u32, u32) {
    let mut y = 1970u32;
    let mut m = 1u32;
    loop {
        let dim = days_in_month(y, m);
        if days < dim {
            break;
        }
        days -= dim;
        m += 1;
        if m > 12 {
            m = 1;
            y += 1;
        }
    }
    (y, m, days + 1)
}

fn days_in_month(y: u32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_json() {
        let m = AppManifest::new("My Todo", "todo", "my_todo");
        let json = m.to_json();
        let parsed = AppManifest::from_json(&json).unwrap();
        assert_eq!(parsed.app_id, m.app_id);
        assert_eq!(parsed.category, "tool");
        assert!(parsed.has_permission(permissions::STORAGE));
    }

    #[test]
    fn default_categories() {
        assert_eq!(default_category("memory-game"), "family");
        assert_eq!(default_category("tv-movies"), "entertainment");
        assert_eq!(default_category("unknown"), "tool");
    }
}

//! `.aiapp` unified app package manifest format definition.
//!
//! Every AI-generated app includes an `aiapp.json` manifest file describing the app's metadata,
//! permission declarations and version info. This is the platform's unified app package format.

use serde::{Deserialize, Serialize};

/// Unified app package manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManifest {
    /// Unique app id, format `app_<slug>`.
    pub app_id: String,
    /// App name (derived from the description).
    pub name: String,
    /// Semantic version number.
    pub version: String,
    /// App description (the user's natural-language description).
    pub description: String,
    /// Minimum required host SDK version.
    pub host_sdk_version: String,
    /// Runtime permission declarations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
    /// App template type.
    pub template: String,
    /// Creation time (RFC 3339).
    pub created_at: String,
}

impl AppManifest {
    /// Build a manifest from the app description and template type.
    pub fn new(desc: &str, template: &str, slug: &str) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // Generate a short app_id from a timestamp hash
        let hash = (now as u64).wrapping_mul(2654435761).to_string();
        let app_id = format!("app_{}", &hash[..8.min(hash.len())]);
        Self {
            app_id,
            name: slug.replace('_', " "),
            version: "0.1.0".into(),
            description: desc.to_string(),
            host_sdk_version: ">=0.1.0".into(),
            permissions: default_permissions(template),
            template: template.to_string(),
            created_at: now_to_rfc3339(now),
        }
    }

    /// Serialize to a formatted JSON string.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("Failed to serialize AppManifest")
    }
}

/// Return the default permission declarations for a template type.
fn default_permissions(template: &str) -> Vec<String> {
    match template {
        "todo" => vec!["storage".into()],
        "image-filter" => vec!["storage".into()],
        _ => vec![],
    }
}

fn now_to_rfc3339(secs: u64) -> String {
    // Compute the date from the UNIX epoch
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
    // Simplified: accumulate month by month starting from 1970-01-01
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
    (y, m, days + 1) // day is 1-based
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

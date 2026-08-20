//! `.aiapp` package validation: checks package integrity and permission legality before deployment / going live.

use crate::manifest::{permissions, AppManifest};
use crate::wit::HOST_CAPABILITIES;
use crate::{MANIFEST_FILE, WASM_FILE};

/// Validation report: `ok` is whether everything passed, `errors` is the error list, `warnings` the warnings.
#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    pub ok: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    fn push_err(&mut self, msg: String) {
        self.errors.push(msg);
        self.ok = false;
    }
}

/// The known set of legal permissions (declaring an unregistered permission is flagged as a risk warning).
fn known_permissions() -> Vec<&'static str> {
    HOST_CAPABILITIES
        .iter()
        .map(|(name, _)| *name)
        .chain([permissions::NETWORK])
        .collect()
}

/// Validate the manifest itself.
pub fn validate_manifest(manifest: &AppManifest) -> ValidationReport {
    let mut report = ValidationReport { ok: true, ..Default::default() };

    if manifest.app_id.is_empty() {
        report.push_err("app_id must not be empty".into());
    }
    if manifest.name.is_empty() {
        report.push_err("name must not be empty".into());
    }
    if manifest.version.is_empty() {
        report.push_err("version must not be empty".into());
    }
    if manifest.entry.is_empty() {
        report.push_err("entry must not be empty".into());
    }
    let known = known_permissions();
    for p in &manifest.permissions {
        if !known.contains(&p.as_str()) {
            report.warnings.push(format!("Permission \"{p}\" is not registered in the standard capability set; the host may refuse to grant it"));
        }
    }
    // Declares permissions but is missing storage (a common omission for data-storing apps)
    if !manifest.permissions.is_empty() && !manifest.has_permission(permissions::STORAGE) {
        report
            .warnings
            .push("The app declares permissions but does not include storage; persisted data may be unavailable".into());
    }
    report
}

/// Validate an entire package directory (manifest + entry WASM present + manifest validation).
pub fn validate_package(dir: &std::path::Path) -> ValidationReport {
    let mut report = ValidationReport { ok: true, ..Default::default() };

    let manifest_path = dir.join(MANIFEST_FILE);
    if !manifest_path.is_file() {
        report.push_err(format!("Missing {MANIFEST_FILE}"));
        return report;
    }
    let manifest = match std::fs::read_to_string(&manifest_path) {
        Ok(s) => match AppManifest::from_json(&s) {
            Ok(m) => m,
            Err(e) => {
                report.push_err(format!("Manifest parse failed: {e}"));
                return report;
            }
        },
        Err(e) => {
            report.push_err(format!("Failed to read manifest: {e}"));
            return report;
        }
    };

    let wasm_path = dir.join(&manifest.entry);
    if !wasm_path.is_file() {
        report.push_err(format!("Missing entry file {}", manifest.entry));
    }

    let manifest_report = validate_manifest(&manifest);
    report.ok = report.ok && manifest_report.ok;
    report.errors.extend(manifest_report.errors);
    report.warnings.extend(manifest_report.warnings);

    // The entry must be WASM bytecode (starts with the magic `\0asm`)
    if let Ok(bytes) = std::fs::read(&wasm_path) {
        let magic_ok = bytes.get(0..4) == Some(b"\0asm");
        if !magic_ok {
            report.push_err(format!("{} is not valid WASM bytecode", manifest.entry));
        }
    }

    report
}

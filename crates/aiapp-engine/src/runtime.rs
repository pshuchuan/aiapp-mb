//! Runtime: loads a `.aiapp` package and executes it.
//!
//! - Default (no `wasmtime` feature): provides "metadata execution" — validates the package, checks
//!   permissions, and hands the app entry/resources to the host; suitable for preview and development
//!   before sandboxing.
//! - With the `wasmtime` feature: uses Wasmtime to actually execute the WASM bytecode, linking host
//!   capabilities to the app through WIT interfaces (`host::Host` → WASM imports).

use std::sync::Arc;

use aiapp_format::AppManifest;
use aiapp_format::{validate_package, AiappPackage};

use crate::host::Host;
use crate::permissions::{Permission, PermissionChecker};

/// Runtime error.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Package parse failed: {0}")]
    Package(#[from] aiapp_format::PackageError),
    #[error("Package validation failed: {0:?}")]
    InvalidPackage(Vec<String>),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("WASM execution failed: {0}")]
    Wasm(String),
}

/// Runtime configuration.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Permissions actually granted by the host (default = all declared in the manifest).
    pub granted_permissions: Option<Vec<String>>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        RuntimeConfig {
            granted_permissions: None,
        }
    }
}

/// A running app instance.
pub struct RunningApp {
    /// App manifest.
    pub manifest: AppManifest,
    /// Permission checker (created by the runtime; the host can re-check it).
    pub permissions: PermissionChecker,
}

/// The runtime.
pub struct Runtime {
    /// Host capabilities.
    host: Arc<dyn Host>,
    /// Configuration.
    config: RuntimeConfig,
}

impl Runtime {
    /// Create a runtime.
    pub fn new(host: Arc<dyn Host>) -> Self {
        Runtime {
            host,
            config: RuntimeConfig::default(),
        }
    }

    /// Create a runtime with a configuration.
    pub fn with_config(host: Arc<dyn Host>, config: RuntimeConfig) -> Self {
        Runtime { host, config }
    }

    /// Load and start the app: parse the package → validate → build the permission gate → execute the entry.
    pub async fn run(&self, package: &AiappPackage) -> Result<RunningApp, RuntimeError> {
        // Validate the package structure (manifest / WASM magic number, etc.)
        let report = validate_package(
            package
                .dir
                .as_deref()
                .expect("the runtime requires the package to come from a directory (already parsed)"),
        );
        if !report.ok {
            return Err(RuntimeError::InvalidPackage(report.errors));
        }

        // Permission gate
        let checker = PermissionChecker::from_manifest(&package.manifest)
            .with_granted(
                self.config
                    .granted_permissions
                    .clone()
                    .unwrap_or_else(|| package.manifest.permissions.clone()),
            );

        // Execute the entry
        self.execute(package, &checker).await?;

        Ok(RunningApp {
            manifest: package.manifest.clone(),
            permissions: checker,
        })
    }

    /// Execute the app entry (WASM execution, see the feature branches below).
    async fn execute(
        &self,
        package: &AiappPackage,
        checker: &PermissionChecker,
    ) -> Result<(), RuntimeError> {
        // If the app declares the storage permission but the host denies it, warn at startup (read-only logic still runs)
        if !package.manifest.permissions.is_empty()
            && !package
                .manifest
                .permissions
                .iter()
                .all(|p| checker.check(p) == Permission::Granted)
        {
            for p in &package.manifest.permissions {
                if checker.check(p) != Permission::Granted {
                    let _ = self
                        .host
                        .log("warn", &format!("Permission \"{p}\" not granted; related capabilities unavailable"));
                }
            }
        }
        self.run_entry(package).await
    }

    /// Execute the entry. With the `wasmtime` feature, the WASM is actually executed;
    /// otherwise a placeholder implementation is used (calls the host log, for preview/debugging).
    #[cfg(feature = "wasmtime")]
    async fn run_entry(&self, package: &AiappPackage) -> Result<(), RuntimeError> {
        let _ = self
            .host
            .log("info", "aiapp-engine: wasmtime executor enabled (real WASM run)");
        // WasmExec builds its own tokio runtime internally to drive the async Host (block_on);
        // here we move to a blocking thread via spawn_blocking to avoid a "runtime within runtime" panic.
        let host = Arc::clone(&self.host);
        let wasm = package.wasm.clone();
        let app_id = package.manifest.app_id.clone();
        let exec = crate::WasmExec::new(host);
        let summary = tokio::task::spawn_blocking(move || exec.run(&wasm, &app_id))
            .await
            .map_err(|e| RuntimeError::Wasm(format!("WASM execution task failed: {e}")))?
            .map_err(RuntimeError::Wasm)?;
        let _ = self.host.log("info", &summary);
        Ok(())
    }

    /// Placeholder execution (without the wasmtime feature): only lifecycle hooks, for preview/development.
    #[cfg(not(feature = "wasmtime"))]
    async fn run_entry(&self, _package: &AiappPackage) -> Result<(), RuntimeError> {
        let _ = self.host.log("info", "aiapp-engine: app loaded (lightweight run mode)");
        Ok(())
    }
}

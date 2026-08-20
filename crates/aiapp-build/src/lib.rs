//! aiapp-build: invokes the MoonBit toolchain to compile a project into WASM bytecode and package it into the `.aiapp` format.
//!
//! The second stage of the pipeline. Compilation is done via `moon build --target <target>`;
//! artifact location rule: `<project>/_build/<target>/<mode>/build/cmd/main/main.wasm`.
//! After compiling, it is automatically packaged into the `.aiapp` unified app package format.

use std::path::{Path, PathBuf};

/// Unified builder error type.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("moon command not found; please install the MoonBit toolchain first: https://www.moonbitlang.com/download")]
    MoonNotFound,
    #[error("moon build failed: {0}")]
    BuildFailed(String),
    #[error("Build succeeded but the WASM artifact (main.wasm) was not located")]
    WasmNotFound,
    #[error("Failed to package .aiapp: {0}")]
    PackageFailed(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Build configuration.
#[derive(Debug, Clone)]
pub struct BuildConfig {
    /// Compilation target, default `wasm-gc`.
    pub target: String,
    /// Whether to do a release build.
    pub release: bool,
    /// Whether to package into the `.aiapp` format (default true).
    pub package: bool,
    /// `.aiapp` output directory (defaults to `<name>.aiapp` in the project's parent).
    pub output: Option<PathBuf>,
    /// The `moon` executable path. Defaults to `"moon"` (looked up via PATH);
    /// after auto-install, an absolute path can be passed (e.g. `~/.moon/bin/moon`).
    pub moon_bin: PathBuf,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            target: "wasm-gc".into(),
            release: false,
            package: true,
            output: None,
            moon_bin: PathBuf::from("moon"),
        }
    }
}

/// Build output.
#[derive(Debug)]
pub struct BuildOutput {
    /// WASM bytecode file path.
    pub wasm: PathBuf,
    /// Optional source map file.
    pub map: Option<PathBuf>,
    /// `.aiapp` package directory path (if packaging is enabled).
    pub aiapp: Option<PathBuf>,
}

/// Compile a MoonBit project into WASM bytecode.
pub fn build(project_dir: &Path, config: &BuildConfig) -> Result<BuildOutput, BuildError> {
    // 1. Verify the toolchain exists.
    if std::process::Command::new(&config.moon_bin)
        .arg("--version")
        .output()
        .is_err()
    {
        return Err(BuildError::MoonNotFound);
    }

    // 2. Run the build.
    let mut cmd = std::process::Command::new(&config.moon_bin);
    cmd.arg("build")
        .arg("--target")
        .arg(&config.target)
        .current_dir(project_dir);
    if config.release {
        cmd.arg("--release");
    }
    let out = cmd
        .output()
        .map_err(|e| BuildError::Io(std::io::Error::other(format!("Failed to launch moon: {e}"))))?;
    if !out.status.success() {
        return Err(BuildError::BuildFailed(
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ));
    }

    // 3. Locate the artifact.
    let mode = if config.release { "release" } else { "debug" };
    let candidate = project_dir
        .join("_build")
        .join(&config.target)
        .join(mode)
        .join("build/cmd/main/main.wasm");
    if !candidate.is_file() {
        return Err(BuildError::WasmNotFound);
    }

    let map = {
        let m = candidate.with_extension("wasm.map");
        if m.is_file() {
            Some(m)
        } else {
            None
        }
    };

    // 4. Package into the `.aiapp` format (unified format: manifest + WASM + WIT + resources, see aiapp-format).
    let aiapp_dir = if config.package {
        Some(package_as_aiapp(project_dir, &candidate)?)
    } else {
        None
    };

    Ok(BuildOutput {
        wasm: candidate,
        map,
        aiapp: aiapp_dir,
    })
}

/// Package the compiled artifact into the `.aiapp` unified app package format.
///
/// Delegates to `aiapp-format` so all platforms use the same format definition.
/// `.aiapp` package structure:
/// ```text
/// <name>.aiapp/
///   aiapp.json       # app manifest
///   main.wasm        # compiled WASM bytecode
///   wit/app-host.wit # WIT interface contract (bundled copy, self-contained)
///   resources/       # optional resources
/// ```
fn package_as_aiapp(project_dir: &Path, wasm: &Path) -> Result<PathBuf, BuildError> {
    // moon build puts the artifact in `_build/<target>/<mode>/.../main.wasm`, while the format
    // convention expects the entry to be `main.wasm` at the project root: copy it there first so the
    // package is self-contained.
    let entry = project_dir.join(aiapp_format::WASM_FILE);
    std::fs::copy(wasm, &entry)?;
    aiapp_format::AiappPackage::package(project_dir)
        .map_err(|e| BuildError::PackageFailed(e.to_string()))
}

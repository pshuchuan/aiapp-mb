//! `.aiapp` package packaging / parsing.
//!
//! A `.aiapp` package is a directory (or later compressed into a single file); its structure is described in `lib.rs`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::manifest::AppManifest;
use crate::{MANIFEST_FILE, RESOURCES_DIR, WASM_FILE, WIT_DIR};

/// Packaging / parsing error.
#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Manifest missing or invalid: {0}")]
    BadManifest(String),
    #[error("WASM entry missing: {0}")]
    MissingWasm(String),
    #[error("Package directory does not exist: {0}")]
    MissingDir(String),
}

/// A parsed `.aiapp` package (in-memory state).
#[derive(Debug, Clone)]
pub struct AiappPackage {
    /// Manifest.
    pub manifest: AppManifest,
    /// WASM bytecode.
    pub wasm: Vec<u8>,
    /// The directory the package is in (if any).
    pub dir: Option<PathBuf>,
    /// Resource files (relative path → contents).
    pub resources: Vec<(String, Vec<u8>)>,
}

impl AiappPackage {
    /// Package a project directory into a `.aiapp` directory (consistent with the `aiapp-build` artifact convention).
    ///
    /// The input project directory should contain `aiapp.json` and the compiled `main.wasm`;
    /// the output is `<parent>/<name>.aiapp/`.
    pub fn package(project_dir: &Path) -> Result<PathBuf, PackageError> {
        let manifest_path = project_dir.join(MANIFEST_FILE);
        if !manifest_path.is_file() {
            return Err(PackageError::BadManifest(
                "aiapp.json is missing from the project directory".into(),
            ));
        }
        let project_name = project_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "app".into());
        let aiapp_dir = project_dir
            .parent()
            .map(|p| p.join(format!("{project_name}.aiapp")))
            .unwrap_or_else(|| PathBuf::from(format!("{project_name}.aiapp")));
        if aiapp_dir.exists() {
            fs::remove_dir_all(&aiapp_dir)?;
        }
        fs::create_dir_all(&aiapp_dir)?;
        fs::copy(&manifest_path, aiapp_dir.join(MANIFEST_FILE))?;

        let wasm_src = project_dir.join(WASM_FILE);
        if wasm_src.is_file() {
            fs::copy(&wasm_src, aiapp_dir.join(WASM_FILE))?;
        }

        // Package the WIT interface definition (a bundled copy, so the package is self-contained).
        let wit_dir = aiapp_dir.join(WIT_DIR);
        fs::create_dir_all(&wit_dir)?;
        fs::write(wit_dir.join("app-host.wit"), crate::wit::APP_HOST_WIT)?;

        // Package the resources directory (if present in the project).
        let src_res = project_dir.join(RESOURCES_DIR);
        if src_res.is_dir() {
            let dst_res = aiapp_dir.join(RESOURCES_DIR);
            copy_dir(&src_res, &dst_res)?;
        }

        Ok(aiapp_dir)
    }

    /// Parse a `.aiapp` package from its directory.
    pub fn parse(package_dir: &Path) -> Result<Self, PackageError> {
        if !package_dir.is_dir() {
            return Err(PackageError::MissingDir(package_dir.display().to_string()));
        }
        let manifest_json = fs::read_to_string(package_dir.join(MANIFEST_FILE))
            .map_err(|_| PackageError::BadManifest("aiapp.json is missing".into()))?;
        let manifest = AppManifest::from_json(&manifest_json)
            .map_err(PackageError::BadManifest)?;

        let wasm_path = package_dir.join(&manifest.entry);
        let wasm = fs::read(&wasm_path).map_err(|_| {
            PackageError::MissingWasm(format!("Missing entry file {}", manifest.entry))
        })?;

        let mut resources = Vec::new();
        let res_dir = package_dir.join(RESOURCES_DIR);
        if res_dir.is_dir() {
            collect_dir(&res_dir, &res_dir, &mut resources)?;
        }

        Ok(AiappPackage {
            manifest,
            wasm,
            dir: Some(package_dir.to_path_buf()),
            resources,
        })
    }
}

/// Recursively copy a directory.
fn copy_dir(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

/// Recursively collect resource files (relative path → contents).
fn collect_dir(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_dir(root, &path, out)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            out.push((rel, fs::read(&path)?));
        }
    }
    Ok(())
}

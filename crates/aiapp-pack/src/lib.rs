//! aiapp-pack: turn a `.aiapp` unified app package into a **buildable standalone app project** in one step (Phase 3).
//!
//! Supports two targets:
//! - [`PackTarget::Tauri`]: desktop (Windows .msi/.exe, macOS .dmg/.app, Linux .deb/.AppImage);
//! - [`PackTarget::Capacitor`]: mobile (Android .apk/.aab, iOS .ipa).
//!
//! Every pack applies **brand customization** (`Brand`): display name / unique identifier / version /
//! description / author / homepage (user custom domain), and generates a branded placeholder icon. The
//! shell project executes the WASM inside `.aiapp` with a real Wasmtime / browser WASI runtime, so
//! "generate once, run everywhere" lands in a standalone installer.
//!
//! Relationship to the rest of the pipeline:
//! `description → aiapp-gen generates → aiapp-build compiles .aiapp → aiapp-pack generates a standalone app project → user builds the installer`.

pub mod brand;
pub mod capacitor;
pub mod tauri;

use std::path::{Path, PathBuf};

use aiapp_format::AiappPackage;

use brand::{Brand, BrandOverrides};

/// Pack target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackTarget {
    /// Tauri: standalone desktop app.
    Tauri,
    /// Capacitor: standalone mobile app.
    Capacitor,
}

impl PackTarget {
    /// Parse from a string (`tauri` / `capacitor`).
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "tauri" => Ok(PackTarget::Tauri),
            "capacitor" | "cap" => Ok(PackTarget::Capacitor),
            other => Err(format!(
                "Unknown pack target `{other}`, available: tauri / capacitor"
            )),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            PackTarget::Tauri => "tauri",
            PackTarget::Capacitor => "capacitor",
        }
    }
}

/// Pack configuration.
#[derive(Debug, Clone)]
pub struct PackConfig {
    /// Pack target.
    pub target: PackTarget,
    /// Output directory (default `./dist/<name>-<target>`).
    pub out_dir: Option<PathBuf>,
    /// Brand customization overrides (`None` = derive default from the manifest).
    pub brand: BrandOverrides,
}

impl Default for PackConfig {
    fn default() -> Self {
        Self {
            target: PackTarget::Tauri,
            out_dir: None,
            brand: BrandOverrides::default(),
        }
    }
}

/// Pack error.
#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to parse app package: {0}")]
    Package(String),
    #[error("Invalid brand info: {0}")]
    Brand(String),
}

/// Pack in one step: parse the `.aiapp` package → build the brand → generate the target shell project.
///
/// Returns the generated project root directory.
pub fn pack(package_dir: &Path, config: &PackConfig) -> Result<PathBuf, PackError> {
    let package = AiappPackage::parse(package_dir).map_err(|e| PackError::Package(e.to_string()))?;
    let brand =
        Brand::from_manifest(&package.manifest, &config.brand).map_err(PackError::Brand)?;

    let out_dir = config
        .out_dir
        .clone()
        .unwrap_or_else(|| default_out_dir(&package, config.target, &brand));

    // The output directory must be empty (or absent), to avoid overwriting an existing project
    if out_dir.exists() && std::fs::read_dir(&out_dir)?.next().is_some() {
        return Err(PackError::Io(std::io::Error::other(format!(
            "Output directory exists and is not empty: {}",
            out_dir.display()
        ))));
    }
    std::fs::create_dir_all(&out_dir)?;

    match config.target {
        PackTarget::Tauri => tauri::pack(&package, &brand, &out_dir),
        PackTarget::Capacitor => capacitor::pack(&package, &brand, &out_dir),
    }
}

/// Default output directory: `<package_dir parent>/<name>-<target>`.
fn default_out_dir(package: &AiappPackage, target: PackTarget, brand: &Brand) -> PathBuf {
    let base = package
        .dir
        .as_deref()
        .and_then(|d| d.parent())
        .unwrap_or_else(|| Path::new("."));
    let slug: String = brand
        .name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let slug = slug.trim_matches('-');
    let name = if slug.is_empty() { "app" } else { slug };
    base.join(format!("{name}-{}", target.as_str()))
}

/// Simple `{{KEY}}` template rendering.
pub(crate) fn render(tpl: &str, vars: &[(&str, &str)]) -> String {
    let mut out = tpl.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("{{{{{k}}}}}"), v);
    }
    out
}

/// Recursively copy a directory.
pub(crate) fn copy_dir(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_package(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("aiapp-pack-test-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("cmd/main")).unwrap();
        // Minimal .aiapp: manifest + fake wasm
        let m = aiapp_format::AppManifest::new("Demo App", "todo", "demo_app");
        std::fs::write(dir.join("aiapp.json"), m.to_json()).unwrap();
        std::fs::write(dir.join("main.wasm"), b"\0asm\x01\0\0\0").unwrap();
        dir
    }

    #[test]
    fn render_replaces_placeholders() {
        assert_eq!(render("a{{X}}b", &[("X", "1")]), "a1b");
    }

    #[test]
    fn target_parse() {
        assert_eq!(PackTarget::parse("tauri").unwrap(), PackTarget::Tauri);
        assert_eq!(PackTarget::parse("capacitor").unwrap(), PackTarget::Capacitor);
        assert!(PackTarget::parse("windows").is_err());
    }

    #[test]
    fn pack_tauri_scaffold() {
        let pkg = temp_package("tauri");
        let out = std::env::temp_dir().join(format!("aiapp-pack-out-tauri-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        let cfg = PackConfig {
            target: PackTarget::Tauri,
            out_dir: Some(out.clone()),
            brand: BrandOverrides {
                name: Some("My Todo".into()),
                identifier: Some("com.example.todo".into()),
                author: Some("me".into()),
                homepage: Some("https://example.com".into()),
                ..Default::default()
            },
        };
        let root = pack(&pkg, &cfg).unwrap();
        assert_eq!(root, out);
        for f in [
            "src-tauri/Cargo.toml",
            "src-tauri/build.rs",
            "src-tauri/tauri.conf.json",
            "src-tauri/src/main.rs",
            "src-tauri/src/aiapp_runner.rs",
            "ui/index.html",
            "README.md",
        ] {
            assert!(out.join(f).is_file(), "missing {f}");
        }
        // Brand has been written
        let conf = std::fs::read_to_string(out.join("src-tauri/tauri.conf.json")).unwrap();
        assert!(conf.contains("com.example.todo"));
        assert!(conf.contains("My Todo"));
        assert!(conf.contains("https://example.com"));
        // WASM has been embedded into the resources directory
        let res = std::fs::read_dir(out.join("src-tauri/resources")).unwrap().next().unwrap().unwrap();
        assert!(res.path().join("main.wasm").is_file());
        // The icon has been generated
        assert!(out.join("src-tauri/icons/app-icon.png").is_file());
        let _ = std::fs::remove_dir_all(&pkg);
        let _ = std::fs::remove_dir_all(&out);
    }

    #[test]
    fn pack_capacitor_scaffold() {
        let pkg = temp_package("cap");
        let out = std::env::temp_dir().join(format!("aiapp-pack-out-cap-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        let cfg = PackConfig {
            target: PackTarget::Capacitor,
            out_dir: Some(out.clone()),
            brand: BrandOverrides {
                name: Some("My Todo".into()),
                identifier: Some("com.example.todo".into()),
                ..Default::default()
            },
        };
        pack(&pkg, &cfg).unwrap();
        for f in [
            "package.json",
            "capacitor.config.ts",
            "www/index.html",
            "www/app.js",
            "www/main.wasm",
            "www/aiapp.json",
            "README.md",
        ] {
            assert!(out.join(f).is_file(), "missing {f}");
        }
        let conf = std::fs::read_to_string(out.join("capacitor.config.ts")).unwrap();
        assert!(conf.contains("com.example.todo"));
        let pj = std::fs::read_to_string(out.join("package.json")).unwrap();
        assert!(pj.contains("capacitor"));
        let _ = std::fs::remove_dir_all(&pkg);
        let _ = std::fs::remove_dir_all(&out);
    }

    #[test]
    fn non_empty_outdir_rejected() {
        let pkg = temp_package("occ");
        let out = std::env::temp_dir().join(format!("aiapp-pack-out-occ-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(out.join("stale.txt"), "x").unwrap();
        let cfg = PackConfig {
            target: PackTarget::Tauri,
            out_dir: Some(out.clone()),
            ..Default::default()
        };
        assert!(pack(&pkg, &cfg).is_err());
        let _ = std::fs::remove_dir_all(&pkg);
        let _ = std::fs::remove_dir_all(&out);
    }
}

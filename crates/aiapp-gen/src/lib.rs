//! aiapp-gen: generate MoonBit project source from a natural-language description.
//!
//! The first stage of the pipeline (community edition). Backends:
//! - `mock`: local samples, no external service, ideal for offline demos and tests (default);
//!
//! Real OpenAI-powered generation belongs to the closed-source **Pro** component
//! (`pro/crates/aiapp-gen-pro`), which is not distributed with this open-source repo;
//! the community edition returns a guidance message for the `openai` backend.

pub mod config;
pub mod gen_client;
pub mod manifest;
pub mod mock;
pub mod templates;

use std::fs;
use std::path::{Path, PathBuf};

pub use config::{Backend, GenConfig};
pub use gen_client::{
    default_system_prompt, generate, generate_with_prompt, generate_with_repair,
    validate_source, SYSTEM_PROMPT, SYSTEM_PROMPT_VERSION,
};
pub use manifest::AppManifest;
pub use templates::TEMPLATES;

/// Unified generator error type.
#[derive(Debug, thiserror::Error)]
pub enum GenError {
    #[error("AI backend ({0}) generation failed: {1}")]
    Backend(&'static str, String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Destination directory exists and is not empty: {0}")]
    DestNotEmpty(PathBuf),
    #[error("Unknown template: {0}, available templates: {1}")]
    UnknownTemplate(String, String),
}

/// Call the currently configured AI backend based on the description and return the generated MoonBit source.
/// When the `mock` backend is used with a template specified, the template source is returned.
pub fn generate_source(desc: &str, config: &GenConfig, template: &str) -> Result<String, GenError> {
    // If a template is specified and the mock backend is used, prefer the template source
    if config.backend == Backend::Mock && !template.is_empty() {
        if let Some(source) = templates::get_template_source(template, desc) {
            return Ok(source);
        }
        let available = TEMPLATES
            .iter()
            .map(|(n, _)| *n)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(GenError::UnknownTemplate(template.into(), available));
    }
    match config.backend {
        Backend::Mock => mock::generate(desc),
        Backend::OpenAi => gen_client::generate(desc, config),
        Backend::Pro => gen_client::generate(desc, config),
    }
}

/// Generate source with an overridable system prompt (supports continuous prompt iteration in the admin).
/// `override_prompt` has the same semantics as `generate_source`; only the OpenAI backend uses the prompt.
pub fn generate_source_with_prompt(
    desc: &str,
    config: &GenConfig,
    template: &str,
    override_prompt: &str,
) -> Result<String, GenError> {
    if config.backend == Backend::Mock && !template.is_empty() {
        if let Some(source) = templates::get_template_source(template, desc) {
            return Ok(source);
        }
        let available = TEMPLATES
            .iter()
            .map(|(n, _)| *n)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(GenError::UnknownTemplate(template.into(), available));
    }
    match config.backend {
        Backend::Mock => mock::generate(desc),
        Backend::OpenAi => gen_client::generate_with_prompt(desc, config, override_prompt),
        Backend::Pro => gen_client::generate_with_prompt(desc, config, override_prompt),
    }
}

/// Derive a valid MoonBit package name from the description (keep alphanumerics only, underscore-separated, lowercase).
pub fn slugify(desc: &str) -> String {
    let cleaned: String = desc
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect();
    let mut name = cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
        .to_lowercase();
    if name.is_empty() {
        name = "app".into();
    }
    name
}

/// Generate a complete MoonBit project (including the `aiapp.json` manifest).
///
/// Structure:
/// ```text
/// dest/
///   aiapp.json        # unified app package manifest
///   moon.mod          # package declaration
///   cmd/main/
///     moon.pkg        # marked as an executable package
///     main.mbt        # generated entry source
/// ```
pub fn write_project(dest: &Path, desc: &str, source: &str, template: &str) -> Result<(), GenError> {
    if dest.exists() && fs::read_dir(dest)?.next().is_some() {
        return Err(GenError::DestNotEmpty(dest.to_path_buf()));
    }
    let pkg = slugify(desc);
    let cmd_main = dest.join("cmd/main");
    fs::create_dir_all(&cmd_main)?;

    // Write the MoonBit package declaration
    fs::write(
        dest.join("moon.mod"),
        format!("name = \"aiapp/{pkg}\"\nversion = \"0.1.0\"\n"),
    )?;
    fs::write(cmd_main.join("moon.pkg"), "pkgtype(kind: \"executable\")\n")?;
    fs::write(cmd_main.join("main.mbt"), source)?;

    // Write the unified aiapp.json manifest
    let manifest = AppManifest::new(desc, template, &pkg);
    fs::write(dest.join("aiapp.json"), manifest.to_json())?;

    Ok(())
}

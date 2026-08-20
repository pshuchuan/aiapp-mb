//! aiapp-format: the authoritative definition of the unified app package format (`.aiapp`).
//!
//! A `.aiapp` package = manifest (`aiapp.json`) + WASM bytecode (`main.wasm`)
//! + WIT interface definition (`wit/`, the communication contract between the app and the host)
//! + resource files (`resources/`).
//!
//! ```text
//! <name>.aiapp/
//!   aiapp.json      # manifest: metadata / permission declarations / entry / WIT version
//!   main.wasm       # business logic (MoonBit → WASM bytecode)
//!   wit/
//!     app-host.wit  # app–host communication contract (WIT interface)
//!   resources/      # optional resources (images, styles, etc.)
//! ```
//!
//! This crate only handles the format (description / packaging / parsing / validation), not execution;
//! execution is handled by `aiapp-engine` (lightweight runtime).

pub mod manifest;
pub mod package;
pub mod validate;
pub mod wit;

pub use manifest::AppManifest;
pub use package::{AiappPackage, PackageError};
pub use validate::{ValidationReport, validate_package};
pub use wit::{APP_HOST_WIT, HOST_CAPABILITIES, WIT_VERSION};

/// Manifest file name of a `.aiapp` package.
pub const MANIFEST_FILE: &str = "aiapp.json";
/// WASM entry file name of a `.aiapp` package.
pub const WASM_FILE: &str = "main.wasm";
/// WIT interface directory of a `.aiapp` package.
pub const WIT_DIR: &str = "wit";
/// Resources directory of a `.aiapp` package.
pub const RESOURCES_DIR: &str = "resources";

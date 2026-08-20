//! aiapp-engine: lightweight runtime (open source).
//!
//! Responsible for "playing" a `.aiapp` package: loading the manifest and WASM, invoking host
//! capabilities through the permission gate, and running the app entry. It is the "engine" of the
//! whole platform, paired with `aiapp-format` (format).
//!
//! Layering (aligned with the WIT contract `aiapp:app-host`):
//! - [`host::Host`]: host capability abstraction (notifications / storage / logging), implemented per side;
//! - [`permissions`]: permission gate, authorizing per the manifest declarations;
//! - [`runtime::Runtime`]: loads a `.aiapp` package and executes it (Wasmtime optional).
//!
//! This crate ships no LLM / commercial logic; it is a pure open-source "player".

pub mod host;
pub mod permissions;
pub mod runtime;
#[cfg(feature = "wasmtime")]
pub mod wasmtime;

pub use host::Host;
pub use runtime::{Runtime, RuntimeConfig, RuntimeError};
#[cfg(feature = "wasmtime")]
pub use wasmtime::WasmExec;

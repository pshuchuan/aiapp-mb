//! aiapp-render: rendering layer abstraction.
//!
//! Defines the `AppRenderer` trait — the common interface for rendering a `.aiapp` package
//! on any platform. The platform-agnostic default is the WebView renderer (browser-hosted WASM +
//! HTML/CSS/JS). For platforms without a WebView, a native renderer can be plugged in by
//! implementing `AppRenderer` + `NativeRenderer`.
//!
//! Architecture:
//!
//! ```text
//! RenderLayer (抽象)
//!   ├── WebViewRenderer (默认，全平台 90%)
//!   │   └── 浏览器内置 WASM + HTML/CSS/JS
//!   └── NativeRenderer (扩展，无 WebView 平台)
//!       └── ui.json + 原生组件渲染 + Wasmtime
//! ```
//!
//! This crate lives in `aiapp-lib` so that both the client container (`aiapp-client`) and
//! server-side tooling (`aiapp-engine`) can depend on the same renderer interface.

pub mod host_api;
pub mod renderer;

pub use host_api::HostApi;
pub use renderer::AppRenderer;
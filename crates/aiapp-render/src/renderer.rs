//! AppRenderer: the rendering abstraction layer.
//!
//! Defines the `AppRenderer` trait that all renderers (WebView, native, etc.) must implement.
//! The container app selects a renderer at runtime based on platform capabilities.

use std::sync::Arc;

use aiapp_format::AiappPackage;

use crate::host_api::HostApi;

/// Renderer error.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// Package cannot be loaded.
    #[error("package error: {0}")]
    Package(String),
    /// WASM execution failed.
    #[error("WASM error: {0}")]
    Wasm(String),
    /// Host API error.
    #[error("host error: {0}")]
    Host(String),
    /// Renderer not supported on this platform.
    #[error("renderer not supported: {0}")]
    NotSupported(String),
    /// Other runtime error.
    #[error("{0}")]
    Other(String),
}

/// Renderer capability flags.
#[derive(Debug, Clone, Default)]
pub struct RendererCapabilities {
    /// Whether this renderer supports HTML/CSS/JS rendering.
    pub html: bool,
    /// Whether this renderer supports JSON UI description rendering.
    pub json_ui: bool,
    /// Whether this renderer supports the WASM runtime directly.
    pub wasm_runtime: bool,
    /// Whether this renderer supports native UI components.
    pub native_components: bool,
}

/// AppRenderer: the interface for rendering a `.aiapp` package.
///
/// The default implementation is `WebViewRenderer`, which uses the platform's WebView
/// to render HTML/CSS/JS and runs WASM in the browser's built-in WebAssembly runtime.
/// For platforms without a WebView, a `NativeRenderer` implementation can be provided.
pub trait AppRenderer: Send + Sync {
    /// Human-readable renderer name (e.g., "webview", "native", "cef").
    fn name(&self) -> &str;

    /// Returns the capabilities of this renderer.
    fn capabilities(&self) -> RendererCapabilities;

    /// Whether this renderer is supported on the current platform.
    /// Returns `true` if the renderer can be used on this platform.
    fn supported(&self) -> bool;

    /// Load and render a `.aiapp` package.
    ///
    /// The renderer is responsible for:
    /// - Setting up the rendering surface (WebView, native view, etc.)
    /// - Loading the app's UI (HTML or JSON UI description)
    /// - Setting up the WASM execution context
    /// - Injecting the HostApi as the JS Bridge / native bridge
    /// - Running the app lifecycle
    fn render(
        &self,
        package: &AiappPackage,
        host: Arc<dyn HostApi>,
    ) -> Result<(), RenderError>;
}

/// WebView renderer: default implementation.
///
/// Uses the platform's WebView to render HTML/CSS/JS and the browser's built-in
/// WebAssembly runtime. This is the default renderer for all platforms that have a WebView.
///
/// Architecture:
/// ```text
/// WebView
/// ├── HTML/CSS/JS (UI 渲染)
/// ├── WebAssembly.instantiate() (WASM 执行，零 IPC 开销)
/// └── JS Bridge (通过平台 native bridge 调用 HostApi)
/// ```
pub struct WebViewRenderer;

impl WebViewRenderer {
    pub fn new() -> Self {
        WebViewRenderer
    }
}

impl Default for WebViewRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl AppRenderer for WebViewRenderer {
    fn name(&self) -> &str {
        "webview"
    }

    fn capabilities(&self) -> RendererCapabilities {
        RendererCapabilities {
            html: true,
            json_ui: false,
            wasm_runtime: true, // 浏览器内置 WebAssembly
            native_components: false,
        }
    }

    fn supported(&self) -> bool {
        // Default: assumed supported. The platform shell should check at runtime.
        true
    }

    fn render(
        &self,
        _package: &AiappPackage,
        _host: Arc<dyn HostApi>,
    ) -> Result<(), RenderError> {
        // Platform-specific implementation:
        // 1. Create a WebView (or use existing one)
        // 2. Load the app's index.html from the package
        // 3. Inject the JS Bridge with HostApi capabilities
        // 4. Load main.wasm via WebAssembly.instantiate()
        // 5. Call the app's entry point
        //
        // The actual WebView creation and JS Bridge injection is platform-specific
        // and should be implemented in the platform shell (aiapp-client / native app).
        Err(RenderError::NotSupported(
            "WebViewRenderer requires platform-specific implementation (use WebViewRenderer::new() with a platform shell)".into(),
        ))
    }
}

/// Native renderer: extension for platforms without WebView.
///
/// Parses the app's `ui.json` (structured UI description) and renders it using
/// native UI components. WASM execution uses Wasmtime via FFI.
///
/// Architecture:
/// ```text
/// Native Renderer
/// ├── ui.json → 原生组件渲染 (Compose/SwiftUI/ArkUI)
/// ├── Wasmtime (通过 FFI 执行 WASM)
/// └── Native Bridge (直接调用 HostApi)
/// ```
pub trait NativeRenderer: AppRenderer {
    /// Render a UI description tree (ui.json) to native components.
    ///
    /// The `ui_json` is a structured JSON tree describing the UI layout.
    /// Each node has a `type` (column, row, text, button, list, image, etc.)
    /// and a `children` array for container nodes.
    fn render_ui(
        &self,
        ui_json: &serde_json::Value,
        data_context: &serde_json::Value,
        host: Arc<dyn HostApi>,
    ) -> Result<(), RenderError>;
}

/// Renderer registry: manages available renderers and selects the best one.
pub struct RendererRegistry {
    renderers: Vec<Box<dyn AppRenderer>>,
}

impl RendererRegistry {
    /// Create a new registry with the default renderers.
    pub fn new() -> Self {
        RendererRegistry {
            renderers: vec![Box::new(WebViewRenderer::new())],
        }
    }

    /// Register an additional renderer.
    pub fn register(&mut self, renderer: Box<dyn AppRenderer>) {
        self.renderers.push(renderer);
    }

    /// Select the first supported renderer.
    pub fn select(&self) -> Option<&dyn AppRenderer> {
        self.renderers.iter().find(|r| r.supported()).map(|r| r.as_ref())
    }

    /// List all registered renderers.
    pub fn list(&self) -> &[Box<dyn AppRenderer>] {
        &self.renderers
    }
}

impl Default for RendererRegistry {
    fn default() -> Self {
        Self::new()
    }
}
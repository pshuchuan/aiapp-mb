//! WIT interface definition: the communication contract between apps and hosts (renderers).
//!
//! This is the platform's "ecosystem rule": different clients implement the same WIT interface,
//! so apps run cross-platform without any code changes.
//!
//! - Web version: browser Web APIs (localStorage / Notification / Geolocation)
//! - Host app: native capabilities (SQLite / push / camera / location)
//! - Desktop: system capabilities (file system / system notifications)
//!
//! Versioning: bump [`WIT_VERSION`] when changing the interface and record it in `docs/ARCHITECTURE.md`.
//! Capabilities are **progressive**: an app may only have basic capabilities on the web version but can
//! get native camera / location / push capabilities in the host app — the app package stays unchanged;
//! it just requests the corresponding permissions and the host grants them per its capability catalog.

/// Current WIT contract version (corresponds to `host_sdk_version >=X.Y.Z`).
pub const WIT_VERSION: &str = "0.2.0";

/// `app-host` WIT interface definition text (embedded; each package carries its own copy).
pub const APP_HOST_WIT: &str = r#"// app-host.wit — the unified contract between the app and the host (v0.2.0)
package aiapp:app-host;

// Capabilities the app imports from the host: different platforms implement the same interface in different renderers.
interface host {
    // Show a system notification. The web version uses the browser Notification API,
    // the host app uses native push, and desktop uses system notifications.
    show-notification: func(title: string, body: string);

    // Store data. The web version uses localStorage/IndexedDB, the host app uses SQLite.
    save-data: func(key: string, value: list<u8>) -> bool;

    // Load data.
    load-data: func(key: string) -> option<list<u8>>;

    // In-app logging (for debugging).
    log: func(level: string, message: string);

    // ---- Native capabilities (progressive enhancement; only the host app / desktop may implement) ----

    // Network request. The web version uses fetch, the host app uses a native HTTP client.
    http-request: func(url: string, method: string, headers: list<string>, body: option<list<u8>>) -> result<record { status: u16, body: list<u8> }, string>;

    // Location. The web version uses the Geolocation API, the host app uses a native location service.
    get-location: func() -> result<record { lat: f64, lon: f64 }, string>;

    // Camera/gallery: take a photo or pick an image, returning the image bytes.
    take-photo: func() -> result<list<u8>, string>;

    // Native push token (the host app uses it to receive system pushes).
    get-push-token: func() -> option<string>;
}

// Lifecycle entry points exported by the app (invoked by the host).
interface app {
    // App entry: start and render the main UI.
    run: func();
    // App stop (clean up resources).
    stop: func();
}

world app-world {
    import host;
    export app;
}
"#;

/// Capability catalog in the WIT interface (corresponds to `manifest::permissions`).
pub const HOST_CAPABILITIES: &[(&str, &str)] = &[
    ("storage", "save-data / load-data: local data read/write"),
    ("notifications", "show-notification: system notifications"),
    ("log", "log: in-app logging"),
    ("network", "http-request: network requests (fetch / native HTTP)"),
    ("location", "get-location: location (Geolocation / native location)"),
    ("camera", "take-photo: take a photo / pick from the gallery"),
    ("push", "get-push-token: native push token (host app)"),
];

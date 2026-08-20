//! aiapp-host-bridge: mobile host bridge layer (C-ABI).
//!
//! Goal: iOS (Swift) and Android (Kotlin + JNI) host Apps call the Rust runtime through this library,
//! to play `.aiapp` unified application packages. The host's native capabilities (storage / notifications /
//! network / location / camera / push) are injected as **callback function pointers**, implementing the
//! WIT contract `aiapp:app-host`.
//!
//! Usage (iOS Swift example; Android JNI is analogous):
//! ```text
//! 1. Build the cdylib (on macOS, run `cargo build -p aiapp-host-bridge` first to get libaiapp_host_bridge.dylib)
//! 2. The host App calls via FFI:
//!    - aiapp_bridge_create(callbacks, ctx)            -> create host session
//!    - aiapp_bridge_load(bridge, pkg_path)            -> parse the .aiapp package
//!    - aiapp_bridge_run(bridge, "meta"|"wasmtime")    -> run the app (callbacks inject native capabilities)
//!    - aiapp_bridge_free(bridge)                      -> release
//! ```
//!
//! Threading model: `aiapp_bridge_run` uses an independent tokio runtime to drive the engine internally;
//! callbacks are invoked synchronously on the engine thread, so the native side should dispatch to the
//! main thread itself if it needs to update the UI.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::Arc;

use aiapp_engine::host::Host;
use aiapp_engine::{Runtime, RuntimeConfig};
use aiapp_format::AiappPackage;

/// Host callback set shared with the C side (function pointers + context).
#[repr(C)]
pub struct AiappHostCallbacks {
    pub show_notification: Option<
        unsafe extern "C" fn(title: *const c_char, body: *const c_char, ctx: *mut c_void),
    >,
    /// Returns 0 on success, non-zero on failure.
    pub save_data: Option<
        unsafe extern "C" fn(
            key: *const c_char,
            value: *const u8,
            len: usize,
            ctx: *mut c_void,
        ) -> c_int,
    >,
    /// On success returns 0 and fills out/out_len (memory owned by the host, released via `free_bytes`); returns non-zero on failure.
    pub load_data: Option<
        unsafe extern "C" fn(
            key: *const c_char,
            out: *mut *mut u8,
            out_len: *mut usize,
            ctx: *mut c_void,
        ) -> c_int,
    >,
    /// Releases memory returned by `load_data` / `http_request` / `take_photo`.
    pub free_bytes: Option<unsafe extern "C" fn(p: *mut u8, ctx: *mut c_void)>,
    pub log: Option<
        unsafe extern "C" fn(level: *const c_char, message: *const c_char, ctx: *mut c_void),
    >,
    /// Network request. On success returns 0 and fills status/out/out_len; returns non-zero on failure.
    pub http_request: Option<
        unsafe extern "C" fn(
            url: *const c_char,
            method: *const c_char,
            headers: *mut *const c_char,
            headers_len: usize,
            body: *const u8,
            body_len: usize,
            status: *mut u16,
            out: *mut *mut u8,
            out_len: *mut usize,
            ctx: *mut c_void,
        ) -> c_int,
    >,
    /// Location. On success returns 0 and fills lat/lon; returns non-zero on failure.
    pub get_location: Option<unsafe extern "C" fn(lat: *mut f64, lon: *mut f64, ctx: *mut c_void) -> c_int>,
    /// Take photo. On success returns 0 and fills out/out_len; returns non-zero on failure.
    pub take_photo: Option<
        unsafe extern "C" fn(out: *mut *mut u8, out_len: *mut usize, ctx: *mut c_void) -> c_int,
    >,
    /// Push token. On success returns 0 and fills out (out is left empty when no token); returns non-zero on failure.
    pub get_push_token: Option<unsafe extern "C" fn(out: *mut *mut c_char, ctx: *mut c_void) -> c_int>,
    /// Releases the C string returned by `get_push_token`.
    pub free_string: Option<unsafe extern "C" fn(s: *mut c_char, ctx: *mut c_void)>,
    /// User context, passed as-is to all callbacks.
    pub ctx: *mut c_void,
}

// The callback struct must be cross-thread (the engine invokes callbacks on a separate tokio runtime thread).
// ctx's thread safety is guaranteed by the host (hosts typically only access native capabilities on the
// main thread / serial queue); we declare Send+Sync here, leaving the responsibility to the host, consistent
// with the C-ABI contract.
unsafe impl Send for AiappHostCallbacks {}
unsafe impl Sync for AiappHostCallbacks {}

/// Host session handle.
pub struct AiappBridge {
    callbacks: Arc<AiappHostCallbacks>,
    /// The most recently loaded application package (used when running).
    package: Option<AiappPackage>,
    /// The most recent error message (read by get_last_error).
    last_error: std::sync::Mutex<String>,
}

/// Wraps the C callbacks as the engine's `Host` implementation.
struct CallbackHost {
    cb: Arc<AiappHostCallbacks>,
}

fn cstr_from(raw: *const c_char) -> String {
    if raw.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(raw) }.to_string_lossy().into_owned()
}

#[async_trait::async_trait]
impl Host for CallbackHost {
    async fn show_notification(&self, title: &str, body: &str) {
        if let Some(f) = self.cb.show_notification {
            let t = CString::new(title).unwrap_or_default();
            let b = CString::new(body).unwrap_or_default();
            unsafe { f(t.as_ptr(), b.as_ptr(), self.cb.ctx) };
        }
    }

    async fn save_data(&self, key: &str, value: &[u8]) -> bool {
        match self.cb.save_data {
            Some(f) => {
                let k = CString::new(key).unwrap_or_default();
                unsafe { f(k.as_ptr(), value.as_ptr(), value.len(), self.cb.ctx) == 0 }
            }
            None => false,
        }
    }

    async fn load_data(&self, key: &str) -> Option<Vec<u8>> {
        let f = self.cb.load_data?;
        let k = CString::new(key).unwrap_or_default();
        let mut out: *mut u8 = std::ptr::null_mut();
        let mut out_len: usize = 0;
        let rc = unsafe { f(k.as_ptr(), &mut out, &mut out_len, self.cb.ctx) };
        if rc != 0 || out.is_null() {
            return None;
        }
        let buf = unsafe { std::slice::from_raw_parts(out, out_len) }.to_vec();
        if let Some(free) = self.cb.free_bytes {
            unsafe { free(out, self.cb.ctx) };
        }
        Some(buf)
    }

    async fn log(&self, level: &str, message: &str) {
        if let Some(f) = self.cb.log {
            let l = CString::new(level).unwrap_or_default();
            let m = CString::new(message).unwrap_or_default();
            unsafe { f(l.as_ptr(), m.as_ptr(), self.cb.ctx) };
        }
    }

    async fn http_request(
        &self,
        url: &str,
        method: &str,
        headers: &[String],
        body: Option<&[u8]>,
    ) -> Result<(u16, Vec<u8>), String> {
        match self.cb.http_request {
            Some(f) => {
                let u = CString::new(url).unwrap_or_default();
                let m = CString::new(method).unwrap_or_default();
                let hdrs: Vec<CString> = headers
                    .iter()
                    .map(|h| CString::new(h.as_str()).unwrap_or_default())
                    .collect();
                let hdrs_ptrs: Vec<*const c_char> = hdrs.iter().map(|h| h.as_ptr()).collect();
                let (body_ptr, body_len) = match body {
                    Some(b) => (b.as_ptr(), b.len()),
                    None => (std::ptr::null(), 0),
                };
                let mut status: u16 = 0;
                let mut out: *mut u8 = std::ptr::null_mut();
                let mut out_len: usize = 0;
                let rc = unsafe {
                    f(
                        u.as_ptr(),
                        m.as_ptr(),
                        hdrs_ptrs.as_ptr() as *mut *const c_char,
                        hdrs_ptrs.len(),
                        body_ptr,
                        body_len,
                        &mut status,
                        &mut out,
                        &mut out_len,
                        self.cb.ctx,
                    )
                };
                if rc != 0 {
                    return Err(format!("host http_request failed, code={rc}"));
                }
                let buf = if out.is_null() {
                    Vec::new()
                } else {
                    unsafe { std::slice::from_raw_parts(out, out_len) }.to_vec()
                };
                if let Some(free) = self.cb.free_bytes {
                    unsafe { free(out, self.cb.ctx) };
                }
                Ok((status, buf))
            }
            None => Err("the current host does not support network requests (network)".to_string()),
        }
    }

    async fn get_location(&self) -> Result<(f64, f64), String> {
        match self.cb.get_location {
            Some(f) => {
                let mut lat = 0.0f64;
                let mut lon = 0.0f64;
                let rc = unsafe { f(&mut lat, &mut lon, self.cb.ctx) };
                if rc == 0 {
                    Ok((lat, lon))
                } else {
                    Err(format!("host location failed, code={rc}"))
                }
            }
            None => Err("the current host does not support location (location)".to_string()),
        }
    }

    async fn take_photo(&self) -> Result<Vec<u8>, String> {
        match self.cb.take_photo {
            Some(f) => {
                let mut out: *mut u8 = std::ptr::null_mut();
                let mut out_len: usize = 0;
                let rc = unsafe { f(&mut out, &mut out_len, self.cb.ctx) };
                if rc != 0 {
                    return Err(format!("host take_photo failed, code={rc}"));
                }
                let buf = if out.is_null() {
                    Vec::new()
                } else {
                    unsafe { std::slice::from_raw_parts(out, out_len) }.to_vec()
                };
                if let Some(free) = self.cb.free_bytes {
                    unsafe { free(out, self.cb.ctx) };
                }
                Ok(buf)
            }
            None => Err("the current host does not support camera (camera)".to_string()),
        }
    }

    async fn get_push_token(&self) -> Option<String> {
        match self.cb.get_push_token {
            Some(f) => {
                let mut out: *mut c_char = std::ptr::null_mut();
                let rc = unsafe { f(&mut out, self.cb.ctx) };
                if rc != 0 || out.is_null() {
                    return None;
                }
                let s = unsafe { CStr::from_ptr(out) }.to_string_lossy().into_owned();
                if let Some(free) = self.cb.free_string {
                    unsafe { free(out, self.cb.ctx) };
                }
                Some(s)
            }
            None => None,
        }
    }
}

fn set_error(bridge: &AiappBridge, msg: impl Into<String>) {
    *bridge.last_error.lock().unwrap() = msg.into();
}

/// Create a host session. `callbacks` is shallow-copied (function pointers + ctx); the host must ensure its
/// lifetime covers the session.
#[no_mangle]
pub extern "C" fn aiapp_bridge_create(callbacks: *const AiappHostCallbacks) -> *mut AiappBridge {
    if callbacks.is_null() {
        return std::ptr::null_mut();
    }
    let cb = unsafe { (*callbacks).clone() };
    Box::into_raw(Box::new(AiappBridge {
        callbacks: Arc::new(cb),
        package: None,
        last_error: std::sync::Mutex::new(String::new()),
    }))
}

/// Parse the `.aiapp` package (pkg_path is the package directory or .aiapp directory). Returns 0 on success, non-zero on failure.
#[no_mangle]
pub extern "C" fn aiapp_bridge_load(
    bridge: *mut AiappBridge,
    pkg_path: *const c_char,
) -> c_int {
    if bridge.is_null() {
        return -1;
    }
    let bridge = unsafe { &mut *bridge };
    if pkg_path.is_null() {
        set_error(bridge, "pkg_path is null");
        return -1;
    }
    let path = cstr_from(pkg_path);
    match AiappPackage::parse(std::path::Path::new(&path)) {
        Ok(pkg) => {
            bridge.package = Some(pkg);
            0
        }
        Err(e) => {
            set_error(bridge, format!("failed to parse application package: {e}"));
            -1
        }
    }
}

/// Run the application. `mode` is `meta` (lightweight, validation + lifecycle only) or `wasmtime` (real WASM, requires the feature).
/// Returns 0 on success, non-zero on failure (see aiapp_bridge_last_error for the error message).
#[no_mangle]
pub extern "C" fn aiapp_bridge_run(
    bridge: *mut AiappBridge,
    mode: *const c_char,
    grant: *const c_char,
) -> c_int {
    if bridge.is_null() {
        return -1;
    }
    let bridge = unsafe { &mut *bridge };
    let mode = cstr_from(mode);
    let grant = cstr_from(grant);
    let Some(package) = bridge.package.take() else {
        set_error(bridge, "application package not loaded yet (call aiapp_bridge_load first)");
        return -1;
    };
    let granted: Vec<String> = if grant.is_empty() {
        vec![]
    } else {
        grant.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    };
    let host: Arc<dyn Host> = Arc::new(CallbackHost {
        cb: Arc::clone(&bridge.callbacks),
    });
    let config = RuntimeConfig {
        granted_permissions: if granted.is_empty() {
            None
        } else {
            Some(granted)
        },
    };
    let runtime = Runtime::with_config(host.clone(), config);
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            set_error(bridge, format!("failed to initialize tokio runtime: {e}"));
            return -1;
        }
    };
    match mode.as_str() {
        "meta" => {
            match rt.block_on(runtime.run(&package)) {
                Ok(_) => 0,
                Err(e) => {
                    set_error(bridge, e.to_string());
                    -1
                }
            }
        }
        // wasmtime mode: real execution via aiapp-engine's WasmExec (requires the wasmtime feature of
        // aiapp-host-bridge to be enabled at compile time, i.e. aiapp-engine/wasmtime).
        "wasmtime" => {
            #[cfg(feature = "wasmtime")]
            {
                match rt.block_on(runtime.run(&package)) {
                    Ok(_) => 0,
                    Err(e) => {
                        set_error(bridge, e.to_string());
                        -1
                    }
                }
            }
            #[cfg(not(feature = "wasmtime"))]
            {
                set_error(
                    bridge,
                    "wasmtime mode requires the feature enabled: cargo build -p aiapp-host-bridge --features wasmtime",
                );
                -1
            }
        }
        other => {
            set_error(bridge, format!("unknown execution mode: {other} (options: meta / wasmtime)"));
            -1
        }
    }
}

/// Get the most recent error message (writes into `buf`, returns the required length including the trailing NUL).
#[no_mangle]
pub extern "C" fn aiapp_bridge_last_error(
    bridge: *const AiappBridge,
    buf: *mut c_char,
    buf_len: usize,
) -> usize {
    if bridge.is_null() {
        return 0;
    }
    let bridge = unsafe { &*bridge };
    let msg = bridge.last_error.lock().unwrap().clone();
    let c = CString::new(msg).unwrap_or_default();
    let bytes = c.as_bytes_with_nul();
    if !buf.is_null() && buf_len > 0 {
        let n = buf_len.min(bytes.len());
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, n);
        }
    }
    bytes.len()
}

/// Release the host session.
#[no_mangle]
pub extern "C" fn aiapp_bridge_free(bridge: *mut AiappBridge) {
    if !bridge.is_null() {
        unsafe {
            drop(Box::from_raw(bridge));
        }
    }
}

// AiappHostCallbacks needs Clone (shallow copy of the function pointer set).
impl Clone for AiappHostCallbacks {
    fn clone(&self) -> Self {
        AiappHostCallbacks {
            show_notification: self.show_notification,
            save_data: self.save_data,
            load_data: self.load_data,
            free_bytes: self.free_bytes,
            log: self.log,
            http_request: self.http_request,
            get_location: self.get_location,
            take_photo: self.take_photo,
            get_push_token: self.get_push_token,
            free_string: self.free_string,
            ctx: self.ctx,
        }
    }
}

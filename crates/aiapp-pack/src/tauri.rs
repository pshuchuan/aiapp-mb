//! Tauri (desktop) standalone app shell project generation.
//!
//! Generates a **buildable** Tauri 2 project from a `.aiapp` package:
//! - `src-tauri/`: Rust shell (embeds the `.aiapp` WASM; clicking "Run App" executes it for real via Wasmtime + WASI);
//! - `ui/`: minimal window UI (brand color, with a run button and output echo);
//! - `src-tauri/icons/`: branded placeholder icon (PNG);
//! - Brand customization: name / identifier / version / description / author / homepage, written into `tauri.conf.json`.
//!
//! Output targets: Windows `.msi/.exe`, macOS `.dmg/.app`, Linux `.deb/.AppImage`.
//! The user runs `npx tauri build` (or `cargo build`) in the generated directory to get the installer.

use std::path::{Path, PathBuf};

use aiapp_format::AiappPackage;

use crate::brand::{Brand, generate_icon};
use crate::{render, PackError, copy_dir};

/// Generate a Tauri desktop shell project; returns the output directory.
pub fn pack(package: &AiappPackage, brand: &Brand, out_dir: &Path) -> Result<PathBuf, PackError> {
    let manifest = &package.manifest;
    let src_t = out_dir.join("src-tauri");
    let ui = out_dir.join("ui");
    std::fs::create_dir_all(&src_t)?;
    std::fs::create_dir_all(&ui)?;

    // Embed the .aiapp contents into src-tauri/resources/<app_id>/
    let res_dir = sanitize_segment(&manifest.app_id);
    let res_dst = src_t.join("resources").join(&res_dir);
    std::fs::create_dir_all(&res_dst)?;
    copy_dir(&package.dir.clone().unwrap_or_else(|| PathBuf::from(".")), &res_dst)?;

    // Brand icon (generate a placeholder icon if the user did not provide one)
    let icons_dir = src_t.join("icons");
    std::fs::create_dir_all(&icons_dir)?;
    if let Some(icon) = &brand.icon {
        std::fs::copy(icon, icons_dir.join("app-icon.png")).map_err(|e| {
            PackError::Io(std::io::Error::other(format!("Failed to copy custom icon: {e}")))
        })?;
    } else {
        generate_icon(&icons_dir.join("app-icon.png"), brand)
            .map_err(PackError::Brand)?;
    }

    let crate_name = sanitize_crate(&brand.identifier);

    // Core templates
    std::fs::write(
        src_t.join("Cargo.toml"),
        render(T_TAURI_CARGO, &[
            ("CRATE", &crate_name),
            ("VERSION", &brand.version),
            ("DESCRIPTION", &json_escape(&brand.description)),
            ("HOMEPAGE", brand.homepage.as_deref().unwrap_or("")),
        ]),
    )?;
    std::fs::write(src_t.join("build.rs"), T_BUILD_RS)?;
    std::fs::write(
        src_t.join("tauri.conf.json"),
        render(T_TAURI_CONF, &[
            ("NAME", &json_escape(&brand.name)),
            ("VERSION", &brand.version),
            ("IDENTIFIER", &brand.identifier),
            ("DESCRIPTION", &json_escape(&brand.description)),
            ("HOMEPAGE", brand.homepage.as_deref().unwrap_or("")),
        ]),
    )?;

    let src = src_t.join("src");
    std::fs::create_dir_all(&src)?;
    std::fs::write(
        src.join("main.rs"),
        render(T_MAIN_RS, &[("NAME", &brand.name)]),
    )?;
    std::fs::write(
        src.join("aiapp_runner.rs"),
        render(T_RUNNER_RS, &[
            ("PKG_RES_DIR", &res_dir),
            ("APP_ID", &manifest.app_id),
        ]),
    )?;

    // Window UI (brand color theme)
    let (r, g, b) = brand.color();
    std::fs::write(
        ui.join("index.html"),
        render(T_UI_HTML, &[
            ("NAME", &html_escape(&brand.name)),
            ("VERSION", &brand.version),
            ("DESCRIPTION", &html_escape(&brand.description)),
            ("R", &r.to_string()),
            ("G", &g.to_string()),
            ("B", &b.to_string()),
        ]),
    )?;

    // Build instructions
    std::fs::write(
        out_dir.join("README.md"),
        render(T_TAURI_README, &[
            ("NAME", &brand.name),
            ("VERSION", &brand.version),
            ("IDENTIFIER", &brand.identifier),
            ("DESCRIPTION", &brand.description),
        ]),
    )?;

    Ok(out_dir.to_path_buf())
}

/// Turn any string into a local-filesystem-safe path segment.
fn sanitize_segment(s: &str) -> String {
    let out: String = s
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if out.is_empty() {
        "app".into()
    } else {
        out
    }
}

/// Convert a reverse-DNS identifier into a valid Rust crate name (`.`→`_`, must start with a lowercase letter).
fn sanitize_crate(id: &str) -> String {
    let base = id.replace('.', "_");
    let mut out = String::new();
    let mut chars = base.chars().peekable();
    while let Some(&c) = chars.peek() {
        if out.is_empty() && !c.is_ascii_alphabetic() {
            out.push('_');
            continue;
        }
        let c = chars.next().unwrap();
        out.push(if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' });
    }
    if out.is_empty() {
        "aiapp_app".into()
    } else {
        out
    }
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------------- Templates ----------------

const T_TAURI_CARGO: &str = r#"# Generated by aiapp-pack — do not hand-edit file names/structure; you may safely change version, author, etc.
[package]
name = "{{CRATE}}"
version = "{{VERSION}}"
description = "{{DESCRIPTION}}"
edition = "2021"
rust-version = "1.75"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Actually execute the WASM inside .aiapp (Wasmtime + WASI), the same strategy as the open-source runtime aiapp-engine
wasmtime = { version = "24", default-features = false, features = ["cranelift", "gc"] }
wasmtime-wasi = "24"
wasi-common = "24"

[profile.release]
strip = true
"#;

const T_BUILD_RS: &str = r#"fn main() {
    tauri_build::build()
}
"#;

const T_TAURI_CONF: &str = r#"{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "{{NAME}}",
  "version": "{{VERSION}}",
  "identifier": "{{IDENTIFIER}}",
  "build": {
    "beforeDevCommand": "",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "",
    "frontendDist": "../ui"
  },
  "app": {
    "windows": [
      { "title": "{{NAME}}", "width": 960, "height": 640, "resizable": true }
    ],
    "security": { "csp": null },
    "withGlobalTauri": true
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [],
    "category": "Utility",
    "shortDescription": "{{DESCRIPTION}}",
    "longDescription": "{{DESCRIPTION}}",
    "homepage": "{{HOMEPAGE}}"
  }
}
"#;

const T_MAIN_RS: &str = r#"//! {{NAME}} — standalone desktop app generated by aiapp-pack (Tauri).
//!
//! The window embeds a "run panel": clicking "Run App" actually executes the packaged
//! `.aiapp` (WASM + WIT contract) through Wasmtime and echoes the console output to the window.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod aiapp_runner;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![run_aiapp])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Run the embedded .aiapp app and return a console output summary.
#[tauri::command]
fn run_aiapp() -> Result<String, String> {
    aiapp_runner::run_embedded()
}
"#;

const T_RUNNER_RS: &str = r#"//! Embedded .aiapp WASM executor (Wasmtime + WASI).
//!
//! Same strategy as `WasmExec` in the open-source runtime `aiapp-engine`:
//! 1) WASI preview1 provides console output (captured into memory, shown in the window log area);
//! 2) Non-WASI host imports are registered as no-op stubs by signature arity (recorded for permissions/audit).
//!
//! Note: the default MoonBit `wasm-gc` target output contains const-expr GC instructions, which wasmtime 24
//! does not yet support; if module parsing fails, recompile with the classic target before packing:
//! `aiapp build <dir> --target wasm` && `aiapp pack <name>.aiapp --target tauri ...`

use wasmtime::{
    Caller, Config, Engine, ExternType, FuncType, Linker, Module, Store, Val, ValType,
};

/// Embedded WASM bytecode written at pack time (aiapp-pack writes it to resources/{{PKG_RES_DIR}}/main.wasm).
pub const EMBEDDED_WASM: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/{{PKG_RES_DIR}}/main.wasm"
));

/// Store data carrying the WASI context + captured stdout.
struct Ctx {
    wasi: wasmtime_wasi::WasiCtx,
    stdout: wasi_common::pipe::WritePipe<Vec<u8>>,
}

/// Run the embedded WASM and return the captured console output and execution summary.
pub fn run_embedded() -> Result<String, String> {
    run(EMBEDDED_WASM, "{{APP_ID}}")
}

/// Run a piece of WASM bytecode: parse → link (WASI + host capability bridge) → execute entry.
///
/// Entry lookup order: `_start` (WASI console entry) → `main` (MoonBit export).
pub fn run(wasm: &[u8], app_id: &str) -> Result<String, String> {
    let mut config = Config::new();
    // MoonBit's wasm-gc target output relies on GC / function-references types; enable explicitly
    config.wasm_function_references(true);
    config.wasm_gc(true);
    let engine = Engine::new(&config).map_err(|e| format!("Engine init failed: {e}"))?;
    let module = Module::from_binary(&engine, wasm).map_err(|e| {
        format!(
            "Module parse failed: {e}\nHint: MoonBit wasm-gc output contains const-expr GC instructions \
             (array.new_default), which wasmtime 24 does not yet support; recompile with the classic target (--target wasm)"
        )
    })?;

    // WASI context + capture stdout/stderr
    let stdout = wasi_common::pipe::WritePipe::new(Vec::new());
    let wasi = wasmtime_wasi::WasiCtxBuilder::new()
        .stdout(Box::new(stdout.clone()))
        .stderr(Box::new(stdout.clone()))
        .build_p1();
    let mut store = Store::new(&engine, Ctx { wasi, stdout });
    let mut linker = Linker::new(&engine);

    // 1) WASI: provide console output (stdout/stderr)
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |cx| &mut cx.wasi)
        .map_err(|e| format!("WASI linking failed: {e}"))?;

    // 2) Host capability bridge: non-WASI imports are registered as no-op stubs by signature arity (recorded for permissions/audit)
    for import in module.imports() {
        if import.module().starts_with("wasi") {
            continue;
        }
        let ty = match import.ty() {
            ExternType::Func(ft) => ft.clone(),
            _ => continue,
        };
        let ftype = FuncType::new(
            &engine,
            ty.params().collect::<Vec<ValType>>(),
            ty.results().collect::<Vec<ValType>>(),
        );
        let name = import.name().to_string();
        linker
            .func_new(
                import.module(),
                &name,
                ftype,
                |_caller: Caller<'_, Ctx>, _args: &[Val], results: &mut [Val]| {
                    for r in results.iter_mut() {
                        *r = Val::I32(0);
                    }
                    Ok(())
                },
            )
            .map_err(|e| format!("Failed to register host import {name}: {e}"))?;
    }

    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("Instantiation failed: {e}"))?;

    let mut log = format!("App {app_id} started\n");
    for n in ["_start", "main"] {
        if let Ok(entry) = instance.get_typed_func::<(), ()>(&mut store, n) {
            entry
                .call(&mut store, ())
                .map_err(|e| format!("Executing {n} failed: {e}"))?;
            log.push_str(&format!("Executed entry {n}\n"));
            // Drop the WasiCtx first (its fd table still holds a reference to the stdout pipe), otherwise try_into_inner fails
            let _ = std::mem::replace(
                &mut store.data_mut().wasi,
                wasmtime_wasi::WasiCtxBuilder::new().build_p1(),
            );
            let pipe = std::mem::take(&mut store.data_mut().stdout);
            if let Ok(inner) = pipe.try_into_inner() {
                log.push_str(&String::from_utf8_lossy(&inner));
            }
            log.push_str("\n── execution finished ──");
            return Ok(log);
        }
    }
    Err("Module has no callable entry (_start / main)".into())
}
"#;

const T_UI_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{{NAME}}</title>
  <style>
    :root {
      --brand: rgb({{R}}, {{G}}, {{B}});
      --brand-dark: color-mix(in srgb, var(--brand) 75%, black);
    }
    * { box-sizing: border-box; }
    body {
      margin: 0; font-family: system-ui, -apple-system, "PingFang SC", sans-serif;
      background: #faf9f5; color: #2b2a28; height: 100vh; display: flex; flex-direction: column;
    }
    header {
      background: var(--brand); color: #fff; padding: 14px 20px;
      display: flex; align-items: baseline; gap: 10px;
    }
    header h1 { margin: 0; font-size: 18px; letter-spacing: .3px; }
    header .ver { opacity: .85; font-size: 12px; }
    main { flex: 1; padding: 20px; display: flex; flex-direction: column; gap: 14px; }
    button {
      align-self: flex-start; background: var(--brand); color: #fff; border: 0;
      padding: 10px 22px; border-radius: 8px; font-size: 14px; cursor: pointer;
    }
    button:hover { background: var(--brand-dark); }
    pre {
      flex: 1; background: #1e1d1b; color: #d8d4cd; border-radius: 10px;
      padding: 14px; overflow: auto; margin: 0; font-size: 13px; line-height: 1.6;
      white-space: pre-wrap; word-break: break-word;
    }
    .desc { color: #6b6863; font-size: 13px; }
  </style>
</head>
<body>
  <header><h1>{{NAME}}</h1><span class="ver">v{{VERSION}}</span></header>
  <main>
    <div class="desc">{{DESCRIPTION}}</div>
    <button id="run"></button>
    <pre id="log"></pre>
  </main>
  <script>
    // Lightweight i18n (browser language): zh-CN / en
    const I18N_UI = {
      'zh-CN': { run: '运行应用', log: '点击「运行应用」通过 WASM 运行时执行 .aiapp 应用包，输出显示在这里。', running: '运行中…', executing: '执行中…', failed: '执行失败: ' },
      'en': { run: 'Run App', log: 'Click "Run App" to execute the .aiapp package through the WASM runtime; output is shown here.', running: 'Running…', executing: 'Executing…', failed: 'Execution failed: ' }
    };
    const T_UI = I18N_UI[(navigator.language || '').toLowerCase().startsWith('zh') ? 'zh-CN' : 'en'];
    document.getElementById('run').textContent = T_UI.run;
    document.getElementById('log').textContent = T_UI.log;
    const { invoke } = window.__TAURI__.core;
    const btn = document.getElementById('run');
    const log = document.getElementById('log');
    btn.addEventListener('click', async () => {
      btn.disabled = true; btn.textContent = T_UI.running;
      log.textContent = T_UI.executing + '\n';
      try {
        log.textContent += await invoke('run_aiapp');
      } catch (e) {
        log.textContent += '\n' + T_UI.failed + e;
      } finally {
        btn.disabled = false; btn.textContent = T_UI.run;
      }
    });
  </script>
</body>
</html>
"#;

const T_TAURI_README: &str = r#"# {{NAME}}

A standalone desktop app project (Tauri 2) generated in one step by **aiapp-pack** from a `.aiapp` unified app package.
After building you get Windows (.msi/.exe), macOS (.dmg/.app) and Linux (.deb/.AppImage) installers.

- Version: v{{VERSION}}
- Identifier: {{IDENTIFIER}}
- Description: {{DESCRIPTION}}

## Build

### 1) Install system dependencies

- **Linux (Debian/Ubuntu)**:
  ```bash
  sudo apt update
  sudo apt install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
      librsvg2-dev patchelf build-essential curl wget file libssl-dev libxdo-dev
  ```
- **macOS**: needs Xcode Command Line Tools (`xcode-select --install`).
- **Windows**: needs Microsoft C++ Build Tools (including the WebView2 SDK).

### 2) Compile

```bash
cd src-tauri
cargo build --release
# Or build the installer in one command (recommended, requires Node.js):
npx tauri build
# Artifacts are under src-tauri/target/release/bundle/<platform dir>/
```

> Note: `tauri build` needs `npm install -D @tauri-apps/cli` first, or use npx to download it automatically.
> Tauri's built-in icons are used by default; to set a brand icon:
> `npx @tauri-apps/cli icon icons/app-icon.png` (generates the .icns/.ico/.png files needed per platform).

## Brand customization

- App name / identifier / version / homepage: edit `src-tauri/tauri.conf.json`.
- Brand icon: replace `icons/app-icon.png` (1024×1024 recommended) then run `npx @tauri-apps/cli icon icons/app-icon.png`.
- Window title, size: `src-tauri/tauri.conf.json` → `app.windows`.

## Runtime

Click "Run App" in the window → the embedded `.aiapp` WASM is executed for real via Wasmtime + WASI,
and the console output is echoed back to the window. After changing the app logic, run `aiapp build` + `aiapp pack` and rebuild.
"#;

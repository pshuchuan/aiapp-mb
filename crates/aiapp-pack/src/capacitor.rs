//! Capacitor (mobile) standalone app shell project generation.
//!
//! Generates a **buildable** Capacitor project from a `.aiapp` package:
//! - `www/`: Web shell (branded UI + an in-browser WASI runtime), embedding the `.aiapp` `main.wasm`;
//! - `capacitor.config.ts` / `package.json`: Capacitor 6 config (appId / appName / webDir);
//! - Brand customization: name / identifier / version / description / author / homepage.
//!
//! The user runs `npx cap add android/ios && npx cap sync && npx cap build` in the generated directory to get
//! Android `.apk/.aab` and iOS `.ipa`. Native mobile capabilities (camera / location / push) are provided by
//! the host app (`aiapp-host-bridge` C-ABI bridge) per the WIT contract; the Web shell first runs the basic WASM logic.

use std::path::{Path, PathBuf};

use aiapp_format::AiappPackage;

use crate::brand::Brand;
use crate::{render, PackError};

/// Generate a Capacitor mobile shell project; returns the output directory.
pub fn pack(package: &AiappPackage, brand: &Brand, out_dir: &Path) -> Result<PathBuf, PackError> {
    let manifest = &package.manifest;
    let www = out_dir.join("www");
    std::fs::create_dir_all(&www)?;

    // Copy the .aiapp WASM and manifest into www (the Web shell executes it in the browser)
    let pkg_dir = package.dir.clone().unwrap_or_else(|| PathBuf::from("."));
    std::fs::copy(pkg_dir.join(&manifest.entry), www.join("main.wasm"))?;
    let manifest_json =
        std::fs::read_to_string(pkg_dir.join(aiapp_format::MANIFEST_FILE)).map_err(|e| {
            PackError::Io(std::io::Error::other(format!("Failed to read manifest: {e}")))
        })?;
    std::fs::write(www.join("aiapp.json"), manifest_json)?;
    // Resources directory (optional)
    let src_res = pkg_dir.join(aiapp_format::RESOURCES_DIR);
    if src_res.is_dir() {
        crate::copy_dir(&src_res, &www.join(aiapp_format::RESOURCES_DIR))?;
    }

    let (r, g, b) = brand.color();
    std::fs::write(
        www.join("index.html"),
        render(T_WWW_HTML, &[
            ("NAME", &html_escape(&brand.name)),
            ("VERSION", &brand.version),
            ("DESCRIPTION", &html_escape(&brand.description)),
            ("R", &r.to_string()),
            ("G", &g.to_string()),
            ("B", &b.to_string()),
        ]),
    )?;
    std::fs::write(
        www.join("app.js"),
        render(T_APP_JS, &[("NAME", &brand.name)]),
    )?;

    std::fs::write(
        out_dir.join("package.json"),
        render(T_PACKAGE_JSON, &[
            ("NAME", &json_escape(&slug_name(&brand.name))),
            ("VERSION", &brand.version),
            ("DESCRIPTION", &json_escape(&brand.description)),
            ("AUTHOR", &json_escape(&brand.author)),
            ("HOMEPAGE", brand.homepage.as_deref().unwrap_or("")),
        ]),
    )?;
    std::fs::write(
        out_dir.join("capacitor.config.ts"),
        render(T_CAP_CONFIG, &[
            ("APP_ID", &brand.identifier),
            ("APP_NAME", &json_escape(&brand.name)),
        ]),
    )?;

    std::fs::write(
        out_dir.join("README.md"),
        render(T_CAP_README, &[
            ("NAME", &brand.name),
            ("VERSION", &brand.version),
            ("IDENTIFIER", &brand.identifier),
            ("DESCRIPTION", &brand.description),
        ]),
    )?;

    Ok(out_dir.to_path_buf())
}

/// Generate an npm package name (lowercase, no spaces, may use hyphens).
fn slug_name(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().to_lowercase().chars() {
        out.push(if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' });
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() { "aiapp-app".into() } else { out }
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

const T_PACKAGE_JSON: &str = r#"{
  "name": "{{NAME}}",
  "version": "{{VERSION}}",
  "description": "{{DESCRIPTION}}",
  "author": "{{AUTHOR}}",
  "homepage": "{{HOMEPAGE}}",
  "private": true,
  "scripts": {
    "add:android": "cap add android",
    "add:ios": "cap add ios",
    "sync": "cap sync",
    "build:android": "cap sync android && cap build android",
    "build:ios": "cap sync ios && cap build ios"
  },
  "dependencies": {
    "@capacitor/core": "^6.0.0",
    "@capacitor/android": "^6.0.0",
    "@capacitor/ios": "^6.0.0"
  },
  "devDependencies": {
    "@capacitor/cli": "^6.0.0",
    "typescript": "^5.0.0"
  }
}
"#;

const T_CAP_CONFIG: &str = r#"import type { CapacitorConfig } from '@capacitor/cli';

const config: CapacitorConfig = {
  appId: '{{APP_ID}}',
  appName: '{{APP_NAME}}',
  webDir: 'www',
  server: {
    androidScheme: 'https'
  }
};

export default config;
"#;

const T_WWW_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
  <title>{{NAME}}</title>
  <style>
    :root { --brand: rgb({{R}}, {{G}}, {{B}}); --brand-dark: color-mix(in srgb, var(--brand) 75%, black); }
    * { box-sizing: border-box; }
    body {
      margin: 0; font-family: system-ui, -apple-system, "PingFang SC", sans-serif;
      background: #faf9f5; color: #2b2a28; height: 100vh; display: flex; flex-direction: column;
      padding: env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left);
    }
    header { background: var(--brand); color: #fff; padding: 14px 18px; display: flex; align-items: baseline; gap: 10px; }
    header h1 { margin: 0; font-size: 18px; }
    header .ver { opacity: .85; font-size: 12px; }
    main { flex: 1; padding: 18px; display: flex; flex-direction: column; gap: 12px; }
    button {
      background: var(--brand); color: #fff; border: 0; padding: 12px 22px;
      border-radius: 10px; font-size: 15px; cursor: pointer;
    }
    button:hover { background: var(--brand-dark); }
    pre {
      flex: 1; background: #1e1d1b; color: #d8d4cd; border-radius: 10px; padding: 14px;
      overflow: auto; margin: 0; font-size: 13px; line-height: 1.6; white-space: pre-wrap; word-break: break-word;
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
      'zh-CN': { run: '运行应用', log: '点击「运行应用」通过浏览器 WASI 运行时执行 .aiapp 的 WASM，输出显示在这里。', running: '运行中…', executing: '执行中…', noEntry: '模块没有可调用的入口（_start / main）', exitCode: '应用退出码: ', execFailed: '执行失败: ' },
      'en': { run: 'Run App', log: 'Click "Run App" to execute the .aiapp WASM through the in-browser WASI runtime; output is shown here.', running: 'Running…', executing: 'Executing…', noEntry: 'Module has no callable entry (_start / main)', exitCode: 'App exit code: ', execFailed: 'Execution failed: ' }
    };
    const T_UI = I18N_UI[(navigator.language || '').toLowerCase().startsWith('zh') ? 'zh-CN' : 'en'];
    document.getElementById('run').textContent = T_UI.run;
    document.getElementById('log').textContent = T_UI.log;
  </script>
  <script src="./app.js"></script>
</body>
</html>
"#;

const T_APP_JS: &str = r#"// {{NAME}} — Web shell: executes the .aiapp WASM in the browser (minimal WASI preview1 polyfill).
// Note: modern browsers (Chrome 119+/Safari 17.4+/Firefox 120+) support the wasm-gc target;
// if instantiation fails, recompile with the classic target: `aiapp build <dir> --target wasm` and repack.

'use strict';

const logEl = document.getElementById('log');
const btn = document.getElementById('run');

/** Exit error (thrown by wasi proc_exit) */
class ExitError extends Error { constructor(code) { super('exit ' + code); this.code = code; } }

function log(text) { logEl.textContent += text + '\n'; }

/**
 * Minimal WASI preview1 polyfill: provides console output (fd_write → log area) and
 * the clock / random / environment / args capabilities commonly used by console apps.
 */
function makeWasi(env) {
  const mem = () => new Uint8Array(env.memory.buffer);
  const u32 = (p) => new DataView(env.memory.buffer).getUint32(p, true);
  const setU32 = (p, v) => new DataView(env.memory.buffer).setUint32(p, v, true);
  const cstr = (p) => {
    const b = mem(); let end = p; while (end < b.length && b[end] !== 0) end++;
    return new TextDecoder().decode(b.subarray(p, end));
  };

  const stdout = { ptr: 0, len: 0 };

  function fdWrite(fd, iovs, iovsLen, nwritten) {
    if (fd !== 1 && fd !== 2) { setU32(nwritten, 0); return 0; }
    let total = 0;
    for (let i = 0; i < iovsLen; i++) {
      const p = u32(iovs + i * 8);
      const l = u32(iovs + i * 8 + 4);
      const chunk = mem().subarray(p, p + l);
      if (stdout.len === 0) stdout.ptr = p;
      stdout.len += chunk.length;
      total += chunk.length;
    }
    const text = new TextDecoder().decode(mem().subarray(stdout.ptr, stdout.ptr + stdout.len));
    // Only echo complete lines, to avoid a huge single line freezing the UI
    let start = 0; let idx;
    while ((idx = text.indexOf('\n', start)) !== -1) {
      log(text.slice(start, idx)); start = idx + 1;
    }
    const tail = text.slice(start);
    if (tail.trim().length > 0) { log('…'); } // incomplete line not echoed yet
    stdout.len = 0;
    setU32(nwritten, total);
    return 0;
  }

  return {
    // args / environment
    args_sizes_get(argc, buf) { setU32(argc, 0); setU32(buf, 0); return 0; },
    args_get(_argv, _buf) { return 0; },
    environ_sizes_get(count, buf) { setU32(count, 0); setU32(buf, 0); return 0; },
    environ_get(_env, _buf) { return 0; },
    // console output
    fd_write: fdWrite,
    fd_close(_fd) { return 0; },
    fd_seek(_fd, _off, _whence, out) { setU32(out, 0); return 0; },
    fd_fdstat_get(_fd, buf) {
      // report the fd type and read/write flags (filetype=2 character device, fdflags=0)
      new DataView(env.memory.buffer).setUint8(buf, 2);
      new DataView(env.memory.buffer).setUint16(buf + 2, 0, true);
      return 0;
    },
    fd_prestat_get(_fd, _buf) { return 8; /* EBADF */ },
    fd_prestat_dir_name() { return 8; },
    fd_read(_fd, _iovs, _iovsLen, out) { setU32(out, 0); return 0; },
    fd_renumber() { return 0; },
    fd_sync() { return 0; },
    // clock
    clock_time_get(_id, _prec, out) {
      new DataView(env.memory.buffer).setBigUint64(out, BigInt(Date.now()) * 1000000n, true);
      return 0;
    },
    clock_res_get(_id, out) {
      new DataView(env.memory.buffer).setBigUint64(out, 1000000n, true);
      return 0;
    },
    // random
    random_get(ptr, len) {
      crypto.getRandomValues(mem().subarray(ptr, ptr + len));
      return 0;
    },
    sched_yield() { return 0; },
    proc_exit(code) { throw new ExitError(code); },
    // other common imports
    path_open() { return 44; /* ENOSYS */ },
    path_filestat_get() { return 44; },
    path_readlink() { return 44; },
    poll_oneoff(_in, _out, n, nevents) { setU32(nevents, n); return 0; }
  };
}

/** Non-WASI host import stubs (reserved per the WIT contract; the Web shell runs the basic logic first) */
function makeHost(env) {
  return {
    'aiapp:app-host/host': {
      'show-notification(title, body)'() {},
      'save-data(key, value)'() { return 0; },
      'load-data(key)'() { return 0; },
      'log(level, message)'() {},
      'http-request(url, method, headers, body)'() { return 0; },
      'get-location()'() { return 0; },
      'take-photo()'() { return 0; },
      'get-push-token()'() { return 0; }
    }
  };
}

async function run() {
  btn.disabled = true; btn.textContent = T_UI.running;
  logEl.textContent = '';
  try {
    const res = await fetch('./main.wasm');
    const bytes = await res.arrayBuffer();
    const env = { memory: null };
    const wasi = makeWasi(env);
    const host = makeHost(env);
    const { instance } = await WebAssembly.instantiate(bytes, {
      wasi_snapshot_preview1: wasi,
      wasi_unstable: wasi,
      env: host
    });
    env.memory = instance.exports.memory;
    const ex = instance.exports;
    if (typeof ex._start === 'function') { ex._start(); }
    else if (typeof ex.main === 'function') { ex.main(); }
    else { log(T_UI.noEntry); }
  } catch (e) {
    if (e instanceof ExitError) log(T_UI.exitCode + e.code);
    else log(T_UI.execFailed + e.message + '\nHint: if this is a module parse error, recompile with the classic target (--target wasm)');
  } finally {
    btn.disabled = false; btn.textContent = T_UI.run;
  }
}

btn.addEventListener('click', run);
"#;

const T_CAP_README: &str = r#"# {{NAME}}

A mobile app project (Capacitor 6) generated in one step by **aiapp-pack** from a `.aiapp` unified app package.
After building you get Android (.apk/.aab) and iOS (.ipa) installers.

- Version: v{{VERSION}}
- Identifier: {{IDENTIFIER}}
- Description: {{DESCRIPTION}}

## Build

Requires [Node.js ≥ 18](https://nodejs.org) and native SDKs (Android Studio / Xcode).

```bash
# 1) Install dependencies
npm install

# 2) Add the target platform (once)
npx cap add android     # or npx cap add ios

# 3) Sync the www resources and build
npx cap sync
npx cap open android    # open Android Studio and click Run; or
npx cap build android   # build from the command line, artifacts: android/app/build/outputs/apk/...

# iOS
npx cap open ios        # open Xcode, pick a device and Run; or Archive to export .ipa
```

## Brand customization

- App name / identifier / version: edit `capacitor.config.ts` (appName / appId).
- App icon and splash screen: `android/app/src/main/res/`, `ios/App/App/Assets.xcassets/` (replace after the native project is generated).
- Homepage / description: `package.json`.

## Runtime

`www/` is the Web shell: the embedded `.aiapp` WASM is executed in the browser through a minimal WASI polyfill,
and console output is echoed back to the UI. Native mobile capabilities (camera / location / push) are provided by
the host app bridge (`aiapp-host-bridge` C-ABI) per the WIT contract.
After changing the app logic, run `aiapp build` + `aiapp pack` and rebuild.
"#;

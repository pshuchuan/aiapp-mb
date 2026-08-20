<p align="right">
  <a href="README.zh-CN.md">中文</a> | <strong>English</strong>
</p>

# aiapp-lib

Core library workspace for the aiapp ecosystem — published to crates.io as individual crates.

## Crates

| Crate | crates.io | Description |
|-------|-----------|-------------|
| `aiapp-gen` | — | AI generation engine: translates natural language descriptions into MoonBit source code |
| `aiapp-build` | — | MoonBit build toolchain: compiles MoonBit sources into WASM bytecode |
| `aiapp-format` | — | `.aiapp` package format definition: manifest, WIT contract, validation |
| `aiapp-pack` | — | App packager: generates Tauri, Capacitor, and standalone app projects |
| `aiapp-engine` | — | Cross-platform WASM runtime: host capability abstraction, permission gating, Wasmtime executor |
| `aiapp-host-bridge` | — | Native bridge layer: C-ABI interface for iOS (Swift) and Android (Kotlin/JNI) |
| `aiapp-cli` | — | CLI tool: one-command generation, build, and packaging pipeline |

## Usage

Add individual crates to your `Cargo.toml`:

```toml
[dependencies]
aiapp-gen = { git = "https://github.com/wy-ent/aiapp-lib" }
aiapp-engine = { git = "https://github.com/wy-ent/aiapp-lib" }
```

After publishing to crates.io:

```toml
[dependencies]
aiapp-gen = "0.1.0"
aiapp-engine = "0.1.0"
```

## Feature Flags

`aiapp-engine` supports WASM runtime execution via the `wasmtime` feature:

```toml
aiapp-engine = { version = "0.1.0", features = ["wasmtime"] }
```

## Platform Support

| Crate | Web | Desktop | Mobile | TV/Car |
|-------|-----|---------|--------|--------|
| aiapp-gen | ✅ | ✅ | — | — |
| aiapp-build | ✅ | ✅ | — | — |
| aiapp-format | ✅ | ✅ | ✅ | ✅ |
| aiapp-pack | ✅ | ✅ | ✅ | — |
| aiapp-engine | — | ✅ | ✅ | ✅ |
| aiapp-host-bridge | — | — | ✅ | — |
| aiapp-cli | — | ✅ | — | — |

## License

Apache-2.0
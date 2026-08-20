<p align="right">
  <strong>中文</strong> | <a href="README.md">English</a>
</p>

# aiapp-lib

aiapp 生态的核心库工作空间 — 以独立 crate 形式发布到 crates.io。

## Crate 清单

| Crate | crates.io | 说明 |
|-------|-----------|------|
| `aiapp-gen` | — | AI 生成引擎：将自然语言描述转换为 MoonBit 源码 |
| `aiapp-build` | — | MoonBit 构建工具链：将 MoonBit 源码编译为 WASM 字节码 |
| `aiapp-format` | — | `.aiapp` 包格式定义：清单文件、WIT 契约、校验 |
| `aiapp-pack` | — | 应用打包器：生成 Tauri、Capacitor 及独立应用项目 |
| `aiapp-engine` | — | 跨平台 WASM 运行时：宿主能力抽象、权限门控、Wasmtime 执行器 |
| `aiapp-host-bridge` | — | 原生桥接层：iOS (Swift) 和 Android (Kotlin/JNI) 的 C-ABI 接口 |
| `aiapp-cli` | — | CLI 工具：一键生成、构建和打包流水线 |

## 使用方式

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
aiapp-gen = { git = "https://github.com/wy-ent/aiapp-lib" }
aiapp-engine = { git = "https://github.com/wy-ent/aiapp-lib" }
```

发布到 crates.io 后：

```toml
[dependencies]
aiapp-gen = "0.1.0"
aiapp-engine = "0.1.0"
```

## 特性开关

`aiapp-engine` 通过 `wasmtime` 特性支持 WASM 运行时执行：

```toml
aiapp-engine = { version = "0.1.0", features = ["wasmtime"] }
```

## 平台支持

| Crate | Web | 桌面端 | 移动端 | TV/车载 |
|-------|-----|--------|--------|---------|
| aiapp-gen | ✅ | ✅ | — | — |
| aiapp-build | ✅ | ✅ | — | — |
| aiapp-format | ✅ | ✅ | ✅ | ✅ |
| aiapp-pack | ✅ | ✅ | ✅ | — |
| aiapp-engine | — | ✅ | ✅ | ✅ |
| aiapp-host-bridge | — | — | ✅ | — |
| aiapp-cli | — | ✅ | — | — |

## 许可证

Apache-2.0
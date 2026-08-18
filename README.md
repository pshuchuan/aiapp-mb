# aiapp-mb

让用户使用自然语言一键生成应用，一次生成，多处运行，支持 web、安卓、iOS、鸿蒙、Windows、Mac、车机、电视盒等平台。

**用户描述 → AI生成 → MoonBit源码 → WASM字节码**，多端统一运行时容器后续 rust 开发。

## 项目目标

本项目旨在构建一个 AI 原生的跨端应用生成平台。核心目标是让用户通过自然语言描述需求，即可一键生成可在 Web、安卓、iOS、鸿蒙、Windows、macOS、车机、电视盒等多端及分布式运行的单机或全栈应用程序。

- 以 MoonBit 为主要开发语言，用于解析中间描述语言
- 生成 MoonBit 前端代码，构建统一标准格式的 app 包（类似 `.wasm` + `manifest.json`），包含：
  - WASM 字节码（业务逻辑）
  - 容器 app 接口逻辑（应用与宿主运行时的接口协议）

## 流水线

```
自然语言描述 → aiapp-gen 生成 MoonBit 源码 → aiapp-build 调 moon 编译 → WASM 字节码
```

| 阶段 | 组件 | 说明 |
|---|---|---|
| 生成 | [aiapp-gen](crates/aiapp-gen) | 由自然语言描述生成可运行的 MoonBit 工程（`mock` / `openai` 后端） |
| 编译 | [aiapp-build](crates/aiapp-build) | 调用 MoonBit 工具链 `moon build --target wasm-gc`，定位并返回 `main.wasm` |
| 编排 | [aiapp-cli](crates/aiapp-cli) | CLI 入口：`create` / `build` / `go` |

## 使用

需要 [MoonBit 工具链](https://www.moonbitlang.com/download)（提供 `moon` 命令）。

```bash
cargo build --release

# 一键端到端：描述 → MoonBit 工程 → WASM 字节码
./target/release/aiapp go "hello world demo" -o generated/demo

# 分步
./target/release/aiapp create "calculator app" -o generated/calc   # 只生成工程
./target/release/aiapp build generated/calc -o out/main.wasm       # 只编译，并可复制产物
```

### AI 后端配置（环境变量）

默认使用 `mock` 后端（本地示例，离线可用）。切换真实 AI：

| 环境变量 | 说明 | 默认 |
|---|---|---|
| `AIAPP_BACKEND` | `mock` 或 `openai` | `mock` |
| `AIAPP_OPENAI_BASE_URL` | OpenAI 兼容 API 基础地址 | `https://api.openai.com/v1` |
| `AIAPP_OPENAI_API_KEY` | API Key | 空 |
| `AIAPP_OPENAI_MODEL` | 模型名 | `gpt-4o-mini` |

```bash
AIAPP_BACKEND=openai \
AIAPP_OPENAI_BASE_URL=https://api.openai.com/v1 \
AIAPP_OPENAI_API_KEY=sk-xxx \
AIAPP_OPENAI_MODEL=gpt-4o-mini \
./target/release/aiapp go "一个计算器应用" -o generated/calc
```

## 工程结构

```
crates/
  aiapp-cli/    # 二进制入口，编排流水线
  aiapp-gen/    # 生成器：mock / openai 后端，产出 MoonBit 工程
  aiapp-build/  # 构建器：moon build → WASM 产物定位
```

生成的 MoonBit 工程结构：

```text
generated/<pkg>/
  moon.mod              # 包声明
  cmd/main/
    moon.pkg            # 可执行包标记
    main.mbt            # 生成的入口源码
```

## 规划路线

1. 当前版本聚焦于 Web 端：用户输入需求后，系统调用 AI 大模型自动生成 MoonBit 代码并编译为 WASM，在浏览器中实时预览应用效果。后续将基于此开发扩展至移动端与桌面端运行时。主要应用场景包括：
   - 个人开发者与小型团队：快速验证想法，生成 MVP（最小可行产品）或内部工具
   - 企业用户：快速生成表单、仪表盘、轻量级业务应用，提升数字化效率
   - 教育领域：作为编程教学和快速原型的辅助工具
   - 政企领域：作为应急事件、工作成果展示等工具
2. 后续规划：开发跨平台运行时容器，实现"一次生成，多端运行"（Android/iOS/鸿蒙/PC）；完善 `.aiapp` 应用包格式，支持离线分发

## 核心功能范围

- AI 代码生成：通过大模型 API（如 Claude/GPT）将用户自然语言描述转换为 MoonBit 源码
- MoonBit → WASM 编译：调用 MoonBit 工具链（`moon build --target wasm`）将源码编译为 WASM 字节码
- Web 预览沙箱：在浏览器中通过 iframe 或 Web 运行时加载 WASM，实时展示生成的应用
- 应用模板库（基础）：提供 3-5 个预设模板（如待办清单、计算器、图片滤镜），降低生成门槛
- 应用包导出：支持将生成的 WASM + 元数据打包下载（`.aiapp` 格式雏形）

## 路线图

- [x] 端到端流水线骨架：描述 → AI 生成 MoonBit 源码 → moon 编译 WASM
- [ ] 多端统一运行时容器（Rust，加载并执行 WASM 字节码）
- [ ] web / 安卓 / iOS / 鸿蒙 / Windows / Mac / 车机 / 电视盒 平台适配
- [ ] 生成结果的校验与修复（AI 代码编译失败自动重试）

> 原创项目，无移植和参考

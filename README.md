# aiapp-mb

让用户使用自然语言一键生成应用，一次生成，多处运行，支持 web、安卓、iOS、鸿蒙、Windows、Mac、车机、电视盒等平台。

**用户描述 → AI生成 → MoonBit源码 → WASM字节码**，多端统一运行时容器后续 rust 开发。

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

## 路线图

- [x] 端到端流水线骨架：描述 → AI 生成 MoonBit 源码 → moon 编译 WASM
- [ ] 多端统一运行时容器（Rust，加载并执行 WASM 字节码）
- [ ] web / 安卓 / iOS / 鸿蒙 / Windows / Mac / 车机 / 电视盒 平台适配
- [ ] 生成结果的校验与修复（AI 代码编译失败自动重试）

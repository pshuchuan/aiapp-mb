# aiapp-mb — AI 应用平台 (Phase 1: Web MVP)

让用户使用自然语言一键生成应用，一次生成，多处运行，支持 web、安卓、iOS、鸿蒙、Windows、Mac、车机、电视盒等平台。

**用户描述 → AI生成 → MoonBit源码 → WASM字节码 → .aiapp 统一应用包**，多端统一运行时容器后续 rust 开发。

## 流水线

```
自然语言描述 → aiapp-gen 生成 MoonBit 工程 → aiapp-build 调 moon 编译 → .aiapp 包
```

| 阶段 | 组件 | 说明 |
|---|---|---|
| 生成 | [aiapp-gen](crates/aiapp-gen) | 由自然语言描述生成可运行的 MoonBit 工程，支持模板选择（`mock` / `openai` 后端） |
| 编译 | [aiapp-build](crates/aiapp-build) | 调用 MoonBit 工具链 `moon build --target wasm-gc`，自动打包为 `.aiapp` 统一格式 |
| 编排 | [aiapp-cli](crates/aiapp-cli) | CLI 入口：`create` / `build` / `go` / `templates` |

## 特性

- **统一应用包格式 (`.aiapp`)**：每个生成的 MoonBit 工程都包含 `aiapp.json` 清单，编译后自动打包为 `*.aiapp` 目录（含 `aiapp.json` + `main.wasm`）
- **预置应用模板**：Phase 1 提供 4 种模板，覆盖不同复杂度场景
- **双后端支持**：`mock` 后端离线可用，`openai` 后端调用真实 AI 生成

## 模板

| 模板 | 说明 |
|---|---|
| `minimal` | 最小模板：Hello World 风格应用 |
| `calculator` | 计算器：支持四则运算的交互式计算器 |
| `todo` | 待办事项：支持增删改查的待办事项管理 |
| `image-filter` | 图片滤镜：图片滤镜处理应用 |

## 使用

需要 [MoonBit 工具链](https://www.moonbitlang.com/download)（提供 `moon` 命令）。

```bash
cargo build --release

# 列出可用模板
./target/release/aiapp templates

# 使用模板生成工程
./target/release/aiapp create "我的计算器" -o generated/calc -t calculator

# 一键端到端：描述 → MoonBit 工程 → WASM 字节码 → .aiapp 包
./target/release/aiapp go "hello world" -o generated/demo -t minimal

# 分步
./target/release/aiapp create "待办事项" -o generated/todo -t todo   # 只生成工程
./target/release/aiapp build generated/todo                            # 编译 + 打包 .aiapp
```

### 生成的工程结构

```text
generated/<app>/
  aiapp.json            # 统一应用包清单（app_id, name, version, permissions 等）
  moon.mod              # MoonBit 包声明
  cmd/main/
    moon.pkg            # 可执行包标记
    main.mbt            # 生成的入口源码
```

### 编译后的 .aiapp 包结构

```text
generated/<app>.aiapp/
  aiapp.json            # 应用清单
  main.wasm             # 编译后的 WASM 字节码
```

### AI 后端配置（环境变量）

默认使用 `mock` 后端（本地模板示例，离线可用）。切换真实 AI：

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
./target/release/aiapp go "一个计算器应用" -o generated/calc -t calculator
```

## 工程结构

```
crates/
  aiapp-cli/    # 二进制入口，编排流水线
  aiapp-gen/    # 生成器：模板 / mock / openai 后端，产出 .aiapp 工程
  aiapp-build/  # 构建器：moon build → WASM → .aiapp 打包
```

## 路线图

根据 [AI 应用平台演进路线](https://qcn5zapfb0ro.feishu.cn/wiki/ZpWuwQy8widUSCkDL0Wc9PM2nBd)：

### 第一阶段：Web MVP ✅（当前）
- [x] 端到端流水线：描述 → AI 生成 MoonBit 源码 → moon 编译 WASM → .aiapp 包
- [x] 4 种应用模板（minimal / calculator / todo / image-filter）
- [x] 统一 `.aiapp` 应用包格式

### 第二阶段：小程序容器
- [ ] 宿主 App（iOS/Android），内嵌 WASM 运行时
- [ ] 应用市场/发现页
- [ ] 调用原生能力（相机、定位、推送）

### 第三阶段：独立 App 生成
- [ ] Tauri 打包后端（桌面端 .exe/.dmg/.deb）
- [ ] Capacitor 打包后端（移动端 .apk/.ipa）
- [ ] 100+ 应用生态

### 第四阶段：AI 原生操作系统
- [ ] 平台核心可独立部署
- [ ] 离线运行 AI 模型
- [ ] 去中心化应用分发协议
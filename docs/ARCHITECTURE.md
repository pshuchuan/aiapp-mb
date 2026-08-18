# 架构规划：社区版(开源) vs Pro(闭源) 商业化分层

> 目标：**前期就把"我可以开源"和"我必须闭源"分开**，为后续企业商业化
> （激励、广告分成、计费）铺好路，并且从第一天就支持拆成多个仓库。

## 1. 两张部署面

| | 企业自托管侧 = 社区版（开源） | 商业中枢 = Pro（闭源） |
|---|---|---|
| 运行位置 | 企业自己的后端 / 员工电脑 | 平台方（我）的云端 |
| 使用者 | 企业管理员管**本企业**用户与应用 | 我管**所有企业**、跨企业统计与分成 |
| 核心数据 | 企业私域数据 | 跨企业聚合、可用于分成/计费 |
| 定位 | 跑 `.aiapp`、建生态、建立信任 | 生成能力 + 商业变现 |

## 2. 开源 / 闭源清单

### 开源（Open，进开源公仓）
- `.aiapp` 应用包格式规范（`aiapp-format`）
- 运行时引擎 / Player（`aiapp-engine`）
- 开发者工具 `aiapp-cli`，MoonBit 示例应用（种子生态）
- 企业自托管服务 + **企业本地后台**（单租户管理本企业用户/应用/审核）
- 生成能力**接口契约**（只开放接口/API，不开放实现）
- 社区离线生成：Mock / 模板（`aiapp-gen` 社区版）

### 闭源（Closed，进 `pro/` 私有仓）
- 生成 app 的真实技能引擎 + 迭代提示词 DNA（`aiapp-gen-pro`）
- 全局运营后台 `aiapp-saas`（跨企业统计、激励/广告分成、计费）
- 官方精品应用市场
- 模型 key、内网地址、Prompt DNA——一律密闭

> 一条铁律：**运行时 + 格式开源做生态与信任，生成技能 + 运营数据闭源变现。**

## 3. 仓库与目录映射

当前单仓为开源仓；闭源统一放进 `pro/`（已 `.gitignore`，你可单独 `cd pro && git init` 推送私有仓）。

```
当前开源仓
├─ crates
│  ├─ aiapp-format/      (规划) 应用包格式    → 开源
│  ├─ aiapp-engine/      (规划) 运行时引擎    → 开源
│  ├─ aiapp-cli/                开发者工具    → 开源
│  ├─ aiapp-web/                市场+企业后台  → 开源(社区版)
│  ├─ aiapp-build/               构建打包     → 开源
│  └─ aiapp-gen/                生成(社区)     → 开源（仅 Mock/模板）
│
├─ pro/                            ← 闭源私有仓（单独推送）
│  ├─ crates/aiapp-gen-pro/        真实生成引擎+Prompt DNA → 闭源
│  └─ crates/aiapp-saas/   (规划) 全球运营后台/统计/分成/计费 → 闭源
```

私有仓 `pro` 通过 path/git 复用开源仓的公共类型；将来拆仓时改为
`aiapp-gen = { git = "<开源仓>" }` 即可。

## 4. 社区版 vs Pro 功能分层

| 能力 | 社区版(开源) | Pro(闭源) |
|---|---|---|
| 生成（AI 自动） | ❌ 仅 Mock/模板（`AIAPP_BACKEND=mock`） | ✅ OpenAI 兼容 + 提示词迭代 |
| 企业本地管理后台 | ✅ 管理本企业用户/应用/审核 | ★=（随运行时部署） |
| 全局运营后台 / 分成 / 激励 / 计费 | ❌ | ✅ `aiapp-saas` |
| 官方市场 | 展示已发布应用 | 上传精品/商业化应用 |

社区版若配置 `AIAPP_BACKEND=openai`，`aiapp-gen` 会返回引导错误：
> OpenAI 自动生成属于闭源 Pro 能力，请部署 Pro 生成服务。

## 5. 数据 / 事件上报契约（为分成与计费预留）

拆仓后运营数据不要散在开源侧，**从第一天按事件流设计**：

- 开源侧（企业部署）只负责**产生事件**并匿名上报：
  `POST {PRO} /v1/telemetry` 事件：`app_launch`、`app_generate`、`app_report`、`user_register`。
- 闭源侧 `aiapp-saas` 接收、聚合，生成统计与分成/计费账单。
- 口径与事件字段用 **versioned schema**（`docs/telemetry.schema.v1.md`）约定，两边各自演进。

这样将来：
- 企业可关掉上报（纯私有模式），只是不享受分成/云端统计。
- 你方只见到「聚合/事件」，不接触企业内部隐私数据。

## 6. 分阶段落地

1. **阶段 1（已完成）**：物理分隔——`aiapp-gen` 只留社区能力；真实引擎 + Prompt DNA 移入 `pro/aiapp-gen-pro`；`pro/` 进 `.gitignore`。
2. **阶段 2**：拆 crate 规范化命名（`aiapp-format` / `aiapp-engine` / `aiapp-host`），并把 `pro/aiapp-saas` 全局后台独立成服务。
3. **阶段 3**：定 Telemetry schema，开源点击 `/api/admin/stats` 的统计改为「事件上报 + Pro 聚合」。
4. **阶段 4**：把每次向上汇报生成的事件接入计费/分成单元。

## 7. 你需要执行的动作

```bash
cd /workspace/pro                # 闭源私有仓
git init
git add crates/aiapp-gen-pro/    # 别 add 开源仓文件
git commit -m "chore: Pro 生成引擎入仓"
git remote add origin <你的私有仓> && git push
```
开源仓正常提交（`pro/` 已被忽略，不会混入）。
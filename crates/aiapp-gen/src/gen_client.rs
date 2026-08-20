//! AI generation client (community edition).
//!
//! Provides:
//! - `mock` / template: local offline generation (default, via `templates` / `mock`);
//! - `openai`: direct OpenAI-compatible Chat Completions API call (no Pro service needed);
//! - `pro`: calls the closed-source Pro generation service (`aiapp-pro-server`,
//!   `AIAPP_PRO_URL`) over an **HTTP contract**.
//!
//! The community edition includes a self-healing loop (validate_source + strip_fences +
//! generate_with_repair) that works with both the OpenAI and Pro backends.

use crate::config::{Backend, GenConfig};
use crate::GenError;
use std::process::Command;
use std::thread;
use std::time::Duration;

/// System prompt version for the generation skill.
pub const SYSTEM_PROMPT_VERSION: &str = "1.1.0";

/// AI generation system prompt: defines the rules for generating MoonBit apps.
/// This is the same prompt used by the Pro service, now included in the community edition.
pub const SYSTEM_PROMPT: &str = r#"你是一名 MoonBit 语言专家，服务于跨平台 AI 原生应用市场。请根据用户的自然语言描述，编写一个可直接运行的 MoonBit 程序。

# 输出规范
1. 只输出 MoonBit 源码本身。不要任何解释、不要 markdown 代码块标记（不含 ```moonbit 围栏）、不要多余文字。
2. 程序入口必须是 `fn main`。
3. 只能使用 MoonBit 标准库，不要引入任何外部依赖。
4. 代码须符合统一 aiapp 应用包约定：结构清晰、有明确入口、可在主流平台（网页/手机/桌面/车机/电视盒/鸿蒙）运行，避免平台强相关的写法。

# 多端运行约束（重要）
5. 目标运行时是统一 WASM 容器（web/安卓/iOS/鸿蒙/桌面/车机/电视盒共用），因此：
   - 不得使用任何平台专有 API（如直接调用 DOM、Android/iOS SDK、文件系统绝对路径等）；
   - 只依赖 MoonBit 标准库与 `println` 等跨平台 I/O，确保编译为 WASM-GC 后可被统一运行时加载；
   - 若需求涉及原生能力（相机、定位、推送），先用标准输出/占位逻辑表达交互，不要写平台耦合代码。

# 迭代规范（重要，支撑后续持续迭代）
6. 用户输入可能是"新建"或"基于现有应用改进"（update）。若描述里出现"更新/优化/参考……"等迭代意图，视为在既有版本上做增量改进：尽量保持原应用的功能结构与命名，只针对新描述做增强，维持向后兼容。
7. 每次生成的版本都应是自洽可运行的完整源码，而不是片段或示例。

# 自愈修复协议（重要）
8. 当用户的消息以"[自动修复"开头、或包含"未通过校验""请修正""待解决的问题"等字样时，视为**修复请求**：
   - 必须输出**修正后的完整可运行源码**（仍是完整 `fn main` 程序，不是 diff 或片段）；
   - 逐条解决消息中列出的问题，其余功能与命名保持不变；
   - 不要解释、不要代码块标记，直接给修正后的源码。"#;

/// Return the current system prompt.
pub fn default_system_prompt() -> &'static str {
    SYSTEM_PROMPT
}

/// Strip markdown code fences from AI-generated MoonBit source.
/// Handles ```moonbit, ```mbt, and plain ``` fences.
fn strip_fences(s: &str) -> String {
    let t = s.trim();
    let t = t
        .strip_prefix("```moonbit")
        .or_else(|| t.strip_prefix("```mbt"))
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    let t = t
        .strip_suffix("```")
        .map(str::trim)
        .unwrap_or(t)
        .trim();
    t.to_string()
}

/// Lightweight static validation of MoonBit source, without invoking the `moon` toolchain.
/// Returns a list of problems (empty means the source passes basic checks).
///
/// Checks performed:
/// - Non-empty source
/// - Presence of `fn main` entry point
/// - No residual markdown code fences (```)
/// - Balanced curly braces `{}`
/// - Balanced parentheses `()`
pub fn validate_source(src: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let s = src.trim();
    if s.is_empty() {
        problems.push("Generated source is empty".to_string());
        return problems;
    }
    if !s.contains("fn main") {
        problems.push("Missing entry point `fn main`".to_string());
    }
    if s.contains("```") {
        problems.push("Source still contains markdown code fences (```)".to_string());
    }
    if !balanced(s, '{', '}') {
        problems.push("Curly braces `{}` are not balanced".to_string());
    }
    if !balanced(s, '(', ')') {
        problems.push("Parentheses `()` are not balanced".to_string());
    }
    problems
}

/// Check whether `open`/`close` bracket pairs are balanced in the source (heuristic, ignores strings/comments).
fn balanced(s: &str, open: char, close: char) -> bool {
    let mut depth: i32 = 0;
    for c in s.chars() {
        if c == open {
            depth += 1;
        } else if c == close {
            depth -= 1;
            if depth < 0 {
                return false;
            }
        }
    }
    depth == 0
}

/// Call an OpenAI-compatible Chat Completions API and return the generated text.
/// Uses `curl` to make the HTTP request (no extra Rust HTTP client dependency needed).
/// Retries with exponential backoff on transient errors.
fn chat(desc: &str, config: &GenConfig, system_prompt: &str, max_retries: u32) -> Result<String, GenError> {
    if config.api_key.is_empty() {
        return Err(GenError::Backend(
            "openai",
            "AIAPP_OPENAI_API_KEY is not configured. Please set it before using the OpenAI backend.".to_string(),
        ));
    }
    let body = serde_json::json!({
        "model": config.model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": desc },
        ],
        "temperature": 0.2,
    });
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));

    let mut last_err = String::new();
    for attempt in 0..=max_retries {
        let out = Command::new("curl")
            .args([
                "-sS",
                "-f",
                "--max-time",
                "120",
                "-H",
                &format!("Authorization: Bearer {}", config.api_key),
                "-H",
                "Content-Type: application/json",
                "-d",
                &body.to_string(),
                &url,
            ])
            .output()
            .map_err(|e| GenError::Backend("openai", format!("Failed to run curl: {e}")))?;
        if out.status.success() {
            let resp: serde_json::Value = serde_json::from_slice(&out.stdout)
                .map_err(|e| GenError::Backend("openai", format!("Response parse failed: {e}")))?;
            let content = resp["choices"][0]["message"]["content"]
                .as_str()
                .ok_or_else(|| {
                    GenError::Backend("openai", format!("Response missing content field: {resp}"))
                })?;
            return Ok(strip_fences(content));
        }
        last_err = String::from_utf8_lossy(&out.stderr).into_owned();
        if attempt < max_retries {
            // Exponential backoff: 1s, 2s, 4s ...
            thread::sleep(Duration::from_secs(2u64.saturating_pow(attempt)));
        }
    }
    Err(GenError::Backend(
        "openai",
        format!("Retried {max_retries} times still failed: {last_err}"),
    ))
}

/// Call the closed-source Pro generation service (`AIAPP_PRO_URL`) and return the
/// generated / self-healed MoonBit source.
///
/// Contract: `POST {pro_url}/v1/generate`
/// ```json
/// { "description": "…", "system_prompt": "…" }
/// ```
/// Response:
/// ```json
/// { "ok": true, "source": "…", "version": "1.1.0" }
/// { "ok": false, "error": "…" }
/// ```
/// When `pro_api_key` is non-empty, an `X-Api-Key` header is sent (optional server-side validation).
fn generate_via_pro(
    desc: &str,
    config: &GenConfig,
    system_prompt: &str,
) -> Result<String, GenError> {
    let url = format!("{}/v1/generate", config.pro_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "description": desc,
        "system_prompt": system_prompt,
    });

    let mut req = ureq::post(&url).timeout(std::time::Duration::from_secs(300));
    if !config.pro_api_key.is_empty() {
        req = req.set("X-Api-Key", &config.pro_api_key);
    }

    match req.send_json(body) {
        Ok(res) => {
            let v: serde_json::Value = res.into_json().map_err(|e| {
                GenError::Backend("pro", format!("Response parse failed: {e}"))
            })?;
            if v["ok"].as_bool().unwrap_or(false) {
                v["source"]
                    .as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| {
                        GenError::Backend("pro", format!("Response missing source field: {v}"))
                    })
            } else {
                Err(GenError::Backend(
                    "pro",
                    v["error"].as_str().unwrap_or("unknown error").to_string(),
                ))
            }
        }
        Err(ureq::Error::Status(code, res)) => {
            let body = res.into_string().unwrap_or_default();
            Err(GenError::Backend("pro", format!("HTTP {code}: {body}")))
        }
        Err(e) => Err(GenError::Backend("pro", format!("Request failed: {e}"))),
    }
}

/// Generate source with a self-healing loop: generate → validate → retry on failure.
///
/// Works with both `openai` (direct API call) and `pro` (Pro service) backends.
/// The `repair` parameter controls how many fix rounds are attempted (default 2).
pub fn generate_with_repair(
    desc: &str,
    config: &GenConfig,
    system_prompt: &str,
    repair: u32,
) -> Result<String, GenError> {
    let mut source = match config.backend {
        Backend::OpenAi => chat(desc, config, system_prompt, 2)?,
        Backend::Pro => generate_via_pro(desc, config, system_prompt)?,
        Backend::Mock => {
            return Err(GenError::Backend(
                "mock",
                "The mock backend should go through the template branch".to_string(),
            ))
        }
    };

    // Strip fences first, then validate
    source = strip_fences(&source);

    for round in 1..=repair {
        let problems = validate_source(&source);
        if problems.is_empty() {
            return Ok(source);
        }
        // Build a repair request with the validation problems
        let feedback = format!(
            "[自动修复 第{round}轮] 你上一轮生成的 MoonBit 源码未通过校验，请修正后输出**完整可运行**的源码。\n\
             待解决的问题：\n- {}\n\n原始需求：{}",
            problems.join("\n- "),
            desc
        );
        source = match config.backend {
            Backend::OpenAi => chat(&feedback, config, system_prompt, 2)?,
            Backend::Pro => generate_via_pro(&feedback, config, system_prompt)?,
            Backend::Mock => {
                return Err(GenError::Backend(
                    "mock",
                    "The mock backend should go through the template branch".to_string(),
                ))
            }
        };
        source = strip_fences(&source);
    }

    // Final validation
    let problems = validate_source(&source);
    if problems.is_empty() {
        Ok(source)
    } else {
        Err(GenError::Backend(
            "openai",
            format!("Source failed validation after {repair} repair rounds: {:?}", problems),
        ))
    }
}

/// Generate source using an (overridable) system prompt, with self-healing (2 repair rounds).
pub fn generate_with_prompt(
    desc: &str,
    config: &GenConfig,
    system_prompt: &str,
) -> Result<String, GenError> {
    generate_with_repair(desc, config, system_prompt, 2)
}

/// Community edition generation entry point (supports OpenAI and Pro backends, with self-healing).
pub fn generate(desc: &str, config: &GenConfig) -> Result<String, GenError> {
    generate_with_prompt(desc, config, default_system_prompt())
}
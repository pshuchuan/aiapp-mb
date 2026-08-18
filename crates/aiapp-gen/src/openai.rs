//! OpenAI 兼容后端：调用 Chat Completions API 生成 MoonBit 源码。
//!
//! 通过系统 `curl` 发起请求（而非引入重型的 HTTP 客户端），
//! 以便自动继承 `HTTP_PROXY` / `HTTPS_PROXY` 等代理环境变量。

use std::process::Command;

use crate::config::GenConfig;
use crate::GenError;

/// 生成技能提示词的版本号。版本化后便于后台管理端持续迭代，
/// 每次调整提示词都应递增版本号并记录改动，避免影响已有迭代链路。
pub const SYSTEM_PROMPT_VERSION: &str = "1.0.0";

/// 默认系统提示词（编译时内嵌，随包分发）。
///
/// 提示词支持后续持续迭代：文本独立维护、带版本号、结构化分区，
/// 后台管理端可查看与覆盖当前生效的提示词（运行时覆盖优先级更高）。
const SYSTEM_PROMPT: &str = include_str!("system_prompt.md");

/// 返回当前生效的系统提示词。
/// 若某处开启了运行时覆盖（见 `openai::generate_with_prompt`），请传入覆盖版本。
pub fn default_system_prompt() -> &'static str {
    SYSTEM_PROMPT
}

/// 调用 OpenAI 兼容接口生成 MoonBit 源码，使用给定系统提示词
/// （支持后台管理端在线迭代提示词）。
pub fn generate_with_prompt(
    desc: &str,
    config: &GenConfig,
    system_prompt: &str,
) -> Result<String, GenError> {
    if config.api_key.is_empty() {
        return Err(GenError::Backend(
            "openai",
            "未配置 AIAPP_OPENAI_API_KEY，请设置环境变量后重试，或使用 mock 后端"
                .to_string(),
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
        .map_err(|e| GenError::Backend("openai", format!("无法调用 curl: {e}")))?;
    if !out.status.success() {
        return Err(GenError::Backend(
            "openai",
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ));
    }
    let resp: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| GenError::Backend("openai", format!("响应解析失败: {e}")))?;
    let content = resp["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| {
            GenError::Backend("openai", format!("响应中缺少 content 字段: {resp}"))
        })?;
    Ok(strip_fences(content))
}

/// 调用 OpenAI 兼容接口生成 MoonBit 源码（使用默认内置提示词）。
pub fn generate(desc: &str, config: &GenConfig) -> Result<String, GenError> {
    generate_with_prompt(desc, config, default_system_prompt())
}

/// 去除模型可能误加的 markdown 代码块围栏。
fn strip_fences(s: &str) -> String {
    let t = s.trim();
    if let Some(rest) = t.strip_prefix("```") {
        let mut lines = rest.lines().collect::<Vec<_>>();
        if lines
            .first()
            .map(|l| l.trim().starts_with("moonbit") || l.trim().starts_with("moon"))
            == Some(true)
        {
            lines.remove(0);
        }
        if lines.last().map(|l| l.trim() == "```") == Some(true) {
            lines.pop();
        }
        lines.join("\n")
    } else {
        t.to_string()
    }
}

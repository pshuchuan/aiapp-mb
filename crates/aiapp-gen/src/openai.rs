//! OpenAI 兼容后端：调用 Chat Completions API 生成 MoonBit 源码。
//!
//! 通过系统 `curl` 发起请求（而非引入重型的 HTTP 客户端），
//! 以便自动继承 `HTTP_PROXY` / `HTTPS_PROXY` 等代理环境变量。

use std::process::Command;

use crate::config::GenConfig;
use crate::GenError;

const SYSTEM_PROMPT: &str = r#"你是一名 MoonBit 语言专家。根据用户的自然语言描述，生成一个可直接运行的 MoonBit 程序。
要求：
1. 只输出 MoonBit 源码本身，不要任何解释，不要 markdown 代码块标记或额外文字。
2. 程序入口必须是 `fn main`。
3. 只能使用 MoonBit 标准库，不要引入外部依赖。"#;

/// 调用 OpenAI 兼容接口生成 MoonBit 源码。
pub fn generate(desc: &str, config: &GenConfig) -> Result<String, GenError> {
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
            { "role": "system", "content": SYSTEM_PROMPT },
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

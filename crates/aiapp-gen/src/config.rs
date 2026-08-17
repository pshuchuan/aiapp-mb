//! 生成器配置。全部通过环境变量注入，便于流水线/CI 中使用。

use std::env;

/// AI 生成后端。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// 本地示例，无外部依赖（默认）。
    Mock,
    /// OpenAI 兼容 Chat Completions API。
    OpenAi,
}

#[derive(Debug, Clone)]
pub struct GenConfig {
    pub backend: Backend,
    /// OpenAI 兼容 API 基础地址。
    pub base_url: String,
    /// API Key。
    pub api_key: String,
    /// 模型名。
    pub model: String,
}

impl GenConfig {
    /// 从环境变量读取配置。
    ///
    /// | 环境变量 | 说明 | 默认 |
    /// |---|---|---|
    /// | `AIAPP_BACKEND` | `mock` 或 `openai` | `mock` |
    /// | `AIAPP_OPENAI_BASE_URL` | API 基础地址 | `https://api.openai.com/v1` |
    /// | `AIAPP_OPENAI_API_KEY` | API Key | 空 |
    /// | `AIAPP_OPENAI_MODEL` | 模型名 | `gpt-4o-mini` |
    pub fn from_env() -> Self {
        let backend = match env::var("AIAPP_BACKEND").as_deref() {
            Ok("openai") | Ok("openai-compatible") => Backend::OpenAi,
            _ => Backend::Mock,
        };
        Self {
            backend,
            base_url: env::var("AIAPP_OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".into()),
            api_key: env::var("AIAPP_OPENAI_API_KEY").unwrap_or_default(),
            model: env::var("AIAPP_OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into()),
        }
    }
}

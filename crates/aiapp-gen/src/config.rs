//! Generator configuration. All values are injected via environment variables,
//! which makes it convenient to use in pipelines / CI.

use std::env;

/// AI generation backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Local example, no external dependency (default).
    Mock,
    /// OpenAI-compatible Chat Completions API.
    OpenAi,
    /// Closed-source Pro generation service (`aiapp-pro-server`) that calls a
    /// real LLM engine over an HTTP contract. The community edition does not
    /// ship the real engine with the open-source repo; this backend talks to
    /// the Pro service.
    Pro,
}

impl Backend {
    /// Display name.
    pub fn label(self) -> &'static str {
        match self {
            Backend::Mock => "mock",
            Backend::OpenAi => "openai",
            Backend::Pro => "pro",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GenConfig {
    pub backend: Backend,
    /// OpenAI-compatible API base URL.
    pub base_url: String,
    /// API Key.
    pub api_key: String,
    /// Model name.
    pub model: String,
    /// Pro service base URL (`AIAPP_PRO_URL`, e.g. `http://127.0.0.1:8090`).
    pub pro_url: String,
    /// Pro service API key (optional, `X-Api-Key` header).
    pub pro_api_key: String,
    /// Pro service self-healing retry rounds (re-feed the model to regenerate on failure).
    pub pro_repair: u32,
}

impl GenConfig {
    /// Read configuration from environment variables.
    ///
    /// | Environment variable | Description | Default |
    /// |---|---|---|
    /// | `AIAPP_BACKEND` | `mock` / `openai` / `pro` | `mock` |
    /// | `AIAPP_OPENAI_BASE_URL` | OpenAI-compatible API base URL | `https://api.openai.com/v1` |
    /// | `AIAPP_OPENAI_API_KEY` | OpenAI API key | empty |
    /// | `AIAPP_OPENAI_MODEL` | OpenAI model name | `gpt-4o-mini` |
    /// | `AIAPP_PRO_URL` | Closed-source Pro generation service URL | `http://127.0.0.1:8090` |
    /// | `AIAPP_PRO_API_KEY` | Pro service API key (optional) | empty |
    /// | `AIAPP_PRO_REPAIR` | Pro self-healing retry rounds | `2` |
    pub fn from_env() -> Self {
        let backend = match env::var("AIAPP_BACKEND").as_deref() {
            Ok("openai") | Ok("openai-compatible") => Backend::OpenAi,
            Ok("pro") | Ok("pro-service") | Ok("pro-server") => Backend::Pro,
            _ => Backend::Mock,
        };
        let pro_repair = env::var("AIAPP_PRO_REPAIR")
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(2);
        Self {
            backend,
            base_url: env::var("AIAPP_OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".into()),
            api_key: env::var("AIAPP_OPENAI_API_KEY").unwrap_or_default(),
            model: env::var("AIAPP_OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into()),
            pro_url: env::var("AIAPP_PRO_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8090".into()),
            pro_api_key: env::var("AIAPP_PRO_API_KEY").unwrap_or_default(),
            pro_repair,
        }
    }
}

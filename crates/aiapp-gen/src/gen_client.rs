//! 社区版生成客户端。
//!
//! 开源本仓仅提供 **Mock/模板** 等社区离线生成能力。
//! 真正的 OpenAI 兼容生成引擎（含可迭代提示词 DNA）属于闭源 Pro
//! 组件 `pro/crates/aiapp-gen-pro`，不随本仓分发。

use crate::config::GenConfig;
use crate::GenError;

/// 生成技能提示词版本号（社区占位，真实技能见 Pro 组件）。
pub const SYSTEM_PROMPT_VERSION: &str = "community.0";

/// 社区默认提示词：仅用于占位/说明，不含闭源技能 DNA。
const COMMUNITY_PROMPT: &str = "社区版本地生成。如需 AI 自动生成，请部署闭源 Pro 生成服务（pro/aiapp-gen-pro）。";

/// 返回社区占位提示词。
pub fn default_system_prompt() -> &'static str {
    COMMUNITY_PROMPT
}

/// 社区版不支持真实 OpenAI 生成，返回引导提示。
pub fn generate_with_prompt(
    _desc: &str,
    _config: &GenConfig,
    _system_prompt: &str,
) -> Result<String, GenError> {
    Err(GenError::Backend(
        "openai",
        "OpenAI 自动生成属于闭源 Pro 能力（aiapp-gen-pro）。\
         社区版请使用 Mock/模板后端（AIAPP_BACKEND=mock），或部署 Pro 生成服务。"
            .to_string(),
    ))
}

/// 社区版生成入口（同样仅支持本地实现）。
pub fn generate(_desc: &str, _config: &GenConfig) -> Result<String, GenError> {
    Err(GenError::Backend(
        "openai",
        "OpenAI 自动生成属于闭源 Pro 能力（aiapp-gen-pro）。\
         社区版请使用 Mock/模板后端（AIAPP_BACKEND=mock），或部署 Pro 生成服务。"
            .to_string(),
    ))
}
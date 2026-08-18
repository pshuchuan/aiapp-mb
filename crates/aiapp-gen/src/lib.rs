//! aiapp-gen：由自然语言描述生成 MoonBit 工程源码。
//!
//! 流水线第一阶段。支持两种后端：
//! - `mock`：本地示例，不依赖外部服务，便于离线演示与测试；
//! - `openai`：OpenAI 兼容 Chat Completions API。
//!
//! 生成的 MoonBit 工程遵循统一的 `.aiapp` 应用包格式。

pub mod config;
pub mod manifest;
pub mod mock;
pub mod openai;
pub mod templates;

use std::fs;
use std::path::{Path, PathBuf};

pub use config::{Backend, GenConfig};
pub use manifest::AppManifest;
pub use templates::TEMPLATES;

/// 生成器统一错误类型。
#[derive(Debug, thiserror::Error)]
pub enum GenError {
    #[error("AI 后端({0})生成失败: {1}")]
    Backend(&'static str, String),
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("目标目录已存在且非空: {0}")]
    DestNotEmpty(PathBuf),
    #[error("未知模板: {0}，可用模板: {1}")]
    UnknownTemplate(String, String),
}

/// 根据描述调用当前配置的 AI 后端，返回生成的 MoonBit 源码。
/// 当使用 `mock` 后端且指定了模板时，返回模板源码。
pub fn generate_source(desc: &str, config: &GenConfig, template: &str) -> Result<String, GenError> {
    // 如果指定了模板且使用 mock 后端，优先使用模板源码
    if config.backend == Backend::Mock && !template.is_empty() {
        if let Some(source) = templates::get_template_source(template, desc) {
            return Ok(source);
        }
        let available = TEMPLATES
            .iter()
            .map(|(n, _)| *n)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(GenError::UnknownTemplate(template.into(), available));
    }
    match config.backend {
        Backend::Mock => mock::generate(desc),
        Backend::OpenAi => openai::generate(desc, config),
    }
}

/// 由描述推导出合法的 MoonBit 包名（仅保留字母数字，下划线分隔，小写）。
pub fn slugify(desc: &str) -> String {
    let cleaned: String = desc
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect();
    let mut name = cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("_")
        .to_lowercase();
    if name.is_empty() {
        name = "app".into();
    }
    name
}

/// 生成完整的 MoonBit 工程（含 `aiapp.json` 清单）。
///
/// 结构：
/// ```text
/// dest/
///   aiapp.json        # 统一应用包清单
///   moon.mod          # 包声明
///   cmd/main/
///     moon.pkg        # 标记为可执行包
///     main.mbt        # 生成的入口源码
/// ```
pub fn write_project(dest: &Path, desc: &str, source: &str, template: &str) -> Result<(), GenError> {
    if dest.exists() && fs::read_dir(dest)?.next().is_some() {
        return Err(GenError::DestNotEmpty(dest.to_path_buf()));
    }
    let pkg = slugify(desc);
    let cmd_main = dest.join("cmd/main");
    fs::create_dir_all(&cmd_main)?;

    // 写入 MoonBit 包声明
    fs::write(
        dest.join("moon.mod"),
        format!("name = \"aiapp/{pkg}\"\nversion = \"0.1.0\"\n"),
    )?;
    fs::write(cmd_main.join("moon.pkg"), "pkgtype(kind: \"executable\")\n")?;
    fs::write(cmd_main.join("main.mbt"), source)?;

    // 写入 aiapp.json 统一清单
    let manifest = AppManifest::new(desc, template, &pkg);
    fs::write(dest.join("aiapp.json"), manifest.to_json())?;

    Ok(())
}
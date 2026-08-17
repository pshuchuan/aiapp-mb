//! aiapp-gen：由自然语言描述生成 MoonBit 工程源码。
//!
//! 流水线第一阶段。支持两种后端：
//! - `mock`：本地示例，不依赖外部服务，便于离线演示与测试；
//! - `openai`：OpenAI 兼容 Chat Completions API。

pub mod config;
pub mod mock;
pub mod openai;

use std::fs;
use std::path::{Path, PathBuf};

pub use config::{Backend, GenConfig};

/// 生成器统一错误类型。
#[derive(Debug, thiserror::Error)]
pub enum GenError {
    #[error("AI 后端({0})生成失败: {1}")]
    Backend(&'static str, String),
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("目标目录已存在且非空: {0}")]
    DestNotEmpty(PathBuf),
}

/// 根据描述调用当前配置的 AI 后端，返回生成的 MoonBit 源码。
pub fn generate_source(desc: &str, config: &GenConfig) -> Result<String, GenError> {
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

/// 在 `dest` 下生成一个最小的可执行 MoonBit 工程骨架。
///
/// 结构：
/// ```text
/// dest/
///   moon.mod          # 包声明
///   cmd/main/
///     moon.pkg        # 标记为可执行包
///     main.mbt        # 生成的入口源码
/// ```
pub fn write_project(dest: &Path, desc: &str, source: &str) -> Result<(), GenError> {
    if dest.exists() && fs::read_dir(dest)?.next().is_some() {
        return Err(GenError::DestNotEmpty(dest.to_path_buf()));
    }
    let pkg = slugify(desc);
    let cmd_main = dest.join("cmd/main");
    fs::create_dir_all(&cmd_main)?;
    fs::write(
        dest.join("moon.mod"),
        format!("name = \"aiapp/{pkg}\"\nversion = \"0.1.0\"\n"),
    )?;
    fs::write(cmd_main.join("moon.pkg"), "pkgtype(kind: \"executable\")\n")?;
    fs::write(cmd_main.join("main.mbt"), source)?;
    Ok(())
}

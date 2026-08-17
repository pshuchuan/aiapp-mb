//! aiapp-build：调用 MoonBit 工具链把工程编译为 WASM 字节码并定位产物。
//!
//! 流水线第二阶段。底层通过 `moon build --target <target>` 完成编译，
//! 产物定位规则：`<project>/_build/<target>/<mode>/build/cmd/main/main.wasm`。

use std::path::{Path, PathBuf};
use std::process::Command;

/// 构建器统一错误类型。
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("未找到 moon 命令，请先安装 MoonBit 工具链: https://www.moonbitlang.com/download")]
    MoonNotFound,
    #[error("moon build 失败: {0}")]
    BuildFailed(String),
    #[error("构建成功但未定位到 WASM 产物 (main.wasm)")]
    WasmNotFound,
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
}

/// 构建配置。
#[derive(Debug, Clone)]
pub struct BuildConfig {
    /// 编译目标，默认 `wasm-gc`。
    pub target: String,
    /// 是否 release 构建。
    pub release: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            target: "wasm-gc".into(),
            release: false,
        }
    }
}

/// 构建产物。
#[derive(Debug)]
pub struct BuildOutput {
    /// WASM 字节码文件路径。
    pub wasm: PathBuf,
    /// 可选的 source map 文件。
    pub map: Option<PathBuf>,
}

/// 把 MoonBit 工程编译为 WASM 字节码。
pub fn build(project_dir: &Path, config: &BuildConfig) -> Result<BuildOutput, BuildError> {
    // 1. 确认工具链存在。
    if Command::new("moon").arg("--version").output().is_err() {
        return Err(BuildError::MoonNotFound);
    }

    // 2. 执行编译。
    let mut cmd = Command::new("moon");
    cmd.arg("build")
        .arg("--target")
        .arg(&config.target)
        .current_dir(project_dir);
    if config.release {
        cmd.arg("--release");
    }
    let out = cmd
        .output()
        .map_err(|e| BuildError::Io(std::io::Error::other(format!("无法启动 moon: {e}"))))?;
    if !out.status.success() {
        return Err(BuildError::BuildFailed(
            String::from_utf8_lossy(&out.stderr).into_owned(),
        ));
    }

    // 3. 定位产物。
    let mode = if config.release { "release" } else { "debug" };
    let candidate = project_dir
        .join("_build")
        .join(&config.target)
        .join(mode)
        .join("build/cmd/main/main.wasm");
    if candidate.is_file() {
        let map = {
            let m = candidate.with_extension("wasm.map");
            if m.is_file() {
                Some(m)
            } else {
                None
            }
        };
        Ok(BuildOutput { wasm: candidate, map })
    } else {
        Err(BuildError::WasmNotFound)
    }
}

//! aiapp-build：调用 MoonBit 工具链把工程编译为 WASM 字节码并打包为 `.aiapp` 格式。
//!
//! 流水线第二阶段。底层通过 `moon build --target <target>` 完成编译，
//! 产物定位规则：`<project>/_build/<target>/<mode>/build/cmd/main/main.wasm`。
//! 编译后自动打包为 `.aiapp` 统一应用包格式。

use std::fs;
use std::path::{Path, PathBuf};

/// 构建器统一错误类型。
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("未找到 moon 命令，请先安装 MoonBit 工具链: https://www.moonbitlang.com/download")]
    MoonNotFound,
    #[error("moon build 失败: {0}")]
    BuildFailed(String),
    #[error("构建成功但未定位到 WASM 产物 (main.wasm)")]
    WasmNotFound,
    #[error("打包 .aiapp 失败: {0}")]
    PackageFailed(String),
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
    /// 是否打包为 `.aiapp` 格式（默认 true）。
    pub package: bool,
    /// `.aiapp` 输出目录（默认为项目目录上级的 `<name>.aiapp`）。
    pub output: Option<PathBuf>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            target: "wasm-gc".into(),
            release: false,
            package: true,
            output: None,
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
    /// `.aiapp` 包目录路径（如果启用了打包）。
    pub aiapp: Option<PathBuf>,
}

/// 把 MoonBit 工程编译为 WASM 字节码。
pub fn build(project_dir: &Path, config: &BuildConfig) -> Result<BuildOutput, BuildError> {
    // 1. 确认工具链存在。
    if std::process::Command::new("moon")
        .arg("--version")
        .output()
        .is_err()
    {
        return Err(BuildError::MoonNotFound);
    }

    // 2. 执行编译。
    let mut cmd = std::process::Command::new("moon");
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
    if !candidate.is_file() {
        return Err(BuildError::WasmNotFound);
    }

    let map = {
        let m = candidate.with_extension("wasm.map");
        if m.is_file() {
            Some(m)
        } else {
            None
        }
    };

    let mut output = BuildOutput {
        wasm: candidate,
        map,
        aiapp: None,
    };

    // 4. 打包为 `.aiapp` 格式。
    if config.package {
        let aiapp_dir = package_as_aiapp(project_dir, &output.wasm)?;
        output.aiapp = Some(aiapp_dir);
    }

    Ok(output)
}

/// 将编译产物打包为 `.aiapp` 统一应用包格式。
///
/// `.aiapp` 包结构：
/// ```text
/// <name>.aiapp/
///   aiapp.json    # 应用清单（从 MoonBit 工程复制）
///   main.wasm     # 编译后的 WASM 字节码
/// ```
fn package_as_aiapp(project_dir: &Path, wasm_path: &Path) -> Result<PathBuf, BuildError> {
    // 读取工程中的 aiapp.json 清单
    let manifest_src = project_dir.join("aiapp.json");
    if !manifest_src.is_file() {
        return Err(BuildError::PackageFailed(
            "工程目录中缺少 aiapp.json 清单文件".into(),
        ));
    }

    // 确定 .aiapp 输出目录
    let project_name = project_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "app".into());
    let aiapp_dir = project_dir
        .parent()
        .map(|p| p.join(format!("{}.aiapp", project_name)))
        .unwrap_or_else(|| PathBuf::from(format!("{}.aiapp", project_name)));

    // 创建 .aiapp 目录结构
    if aiapp_dir.exists() {
        fs::remove_dir_all(&aiapp_dir)?;
    }
    fs::create_dir_all(&aiapp_dir)?;

    // 复制清单
    fs::copy(&manifest_src, aiapp_dir.join("aiapp.json"))?;

    // 复制 WASM 产物
    fs::copy(wasm_path, aiapp_dir.join("main.wasm"))?;

    // 如果存在 source map，一并复制
    let map_src = wasm_path.with_extension("wasm.map");
    if map_src.is_file() {
        fs::copy(&map_src, aiapp_dir.join("main.wasm.map"))?;
    }

    Ok(aiapp_dir)
}
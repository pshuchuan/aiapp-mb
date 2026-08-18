//! aiapp-cli：aiapp-mb 的命令行入口。
//!
//! 流水线编排：
//! `create` 描述 → MoonBit 工程；`build` MoonBit 工程 → WASM；
//! `go` 一键端到端 描述 → WASM 字节码。

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use aiapp_build::{build, BuildConfig};
use aiapp_gen::{generate_source, write_project, GenConfig};

#[derive(Parser)]
#[command(
    name = "aiapp",
    version,
    about = "aiapp-mb：自然语言一键生成应用 → MoonBit 源码 → WASM 字节码"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 描述 → 生成 MoonBit 工程
    Create {
        /// 应用的自然语言描述
        description: String,
        /// 工程输出目录
        #[arg(short, long, default_value = "generated")]
        output: PathBuf,
    },
    /// MoonBit 工程 → WASM 字节码
    Build {
        /// 工程目录
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// 编译目标
        #[arg(short, long, default_value = "wasm-gc")]
        target: String,
        /// 是否 release 构建
        #[arg(long)]
        release: bool,
        /// 将 WASM 产物复制到指定路径
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// 端到端：描述 → 生成工程 → 编译 WASM
    Go {
        /// 应用的自然语言描述
        description: String,
        /// 工程输出目录
        #[arg(short, long, default_value = "generated")]
        output: PathBuf,
        /// 编译目标
        #[arg(short, long, default_value = "wasm-gc")]
        target: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Create {
            description,
            output,
        } => cmd_create(&description, &output),
        Command::Build {
            dir,
            target,
            release,
            output,
        } => cmd_build(&dir, &target, release, output.as_deref()),
        Command::Go {
            description,
            output,
            target,
        } => cmd_go(&description, &output, &target),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("错误: {e:#}");
            ExitCode::FAILURE
        }
    }
}

/// create：生成 MoonBit 工程。
fn cmd_create(description: &str, output: &PathBuf) -> Result<()> {
    let config = GenConfig::from_env();
    println!(
        "[1/2] 生成 MoonBit 工程（后端: {}）...",
        match config.backend {
            aiapp_gen::Backend::Mock => "mock",
            aiapp_gen::Backend::OpenAi => "openai",
        }
    );
    let source = generate_source(description, &config)
        .with_context(|| "AI 生成 MoonBit 源码失败")?;
    write_project(output, description, &source)
        .with_context(|| format!("写入工程目录 {} 失败", output.display()))?;
    println!("[2/2] 工程已生成: {}", output.display());
    println!("      下一步: aiapp build {}", output.display());
    Ok(())
}

/// build：编译为 WASM 字节码。
fn cmd_build(dir: &PathBuf, target: &str, release: bool, output: Option<&std::path::Path>) -> Result<()> {
    let config = BuildConfig {
        target: target.into(),
        release,
    };
    println!("[1/2] 调用 moon build --target {target} ...");
    let out = build(dir, &config).with_context(|| format!("编译工程 {} 失败", dir.display()))?;
    let bytes = std::fs::metadata(&out.wasm)?.len();
    println!("[2/2] WASM 产物: {} ({} bytes)", out.wasm.display(), bytes);
    if let Some(out_path) = output {
        std::fs::create_dir_all(out_path.parent().unwrap_or_else(|| std::path::Path::new(".")))?;
        std::fs::copy(&out.wasm, out_path)
            .with_context(|| format!("复制产物到 {} 失败", out_path.display()))?;
        println!("      已复制到: {}", out_path.display());
    }
    Ok(())
}

/// go：端到端 描述 → WASM 字节码。
fn cmd_go(description: &str, output: &PathBuf, target: &str) -> Result<()> {
    let config = GenConfig::from_env();
    println!(
        "[1/3] 生成 MoonBit 工程（后端: {}）...",
        match config.backend {
            aiapp_gen::Backend::Mock => "mock",
            aiapp_gen::Backend::OpenAi => "openai",
        }
    );
    let source = generate_source(description, &config)
        .with_context(|| "AI 生成 MoonBit 源码失败")?;
    write_project(output, description, &source)
        .with_context(|| format!("写入工程目录 {} 失败", output.display()))?;
    println!("[2/3] 工程已生成: {}", output.display());

    let build_config = BuildConfig {
        target: target.into(),
        release: false,
    };
    println!("[3/3] 调用 moon build --target {target} ...");
    let out = build(output, &build_config)
        .with_context(|| format!("编译工程 {} 失败", output.display()))?;
    let bytes = std::fs::metadata(&out.wasm)?.len();
    println!("      完成！WASM 字节码: {} ({} bytes)", out.wasm.display(), bytes);
    Ok(())
}

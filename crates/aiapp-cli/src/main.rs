//! aiapp-cli：aiapp-mb 的命令行入口。
//!
//! 流水线编排：
//! `create` 描述 → MoonBit 工程（含 `.aiapp` 清单）；
//! `build` MoonBit 工程 → WASM → `.aiapp` 包；
//! `go` 一键端到端 描述 → `.aiapp` 包。
//!
//! 模板：`aiapp create "描述" -t calculator` 使用预定义模板。

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use aiapp_build::{build, BuildConfig};
use aiapp_gen::{generate_source, write_project, GenConfig, TEMPLATES};

#[derive(Parser)]
#[command(
    name = "aiapp",
    version,
    about = "aiapp-mb：自然语言一键生成应用 → MoonBit 源码 → WASM 字节码 → .aiapp 包"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 描述 → 生成 MoonBit 工程（含 aiapp.json 清单）
    Create {
        /// 应用的自然语言描述
        description: String,
        /// 工程输出目录
        #[arg(short, long, default_value = "generated")]
        output: PathBuf,
        /// 应用模板（可选）
        /// 可用模板: minimal, calculator, todo, image-filter
        #[arg(short, long, default_value = "minimal")]
        template: String,
        /// 列出可用模板
        #[arg(long)]
        list_templates: bool,
    },
    /// MoonBit 工程 → WASM 字节码 → .aiapp 包
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
        /// 跳过 .aiapp 打包
        #[arg(long)]
        no_package: bool,
    },
    /// 端到端：描述 → 生成工程 → 编译 WASM → .aiapp 包
    Go {
        /// 应用的自然语言描述
        description: String,
        /// 工程输出目录
        #[arg(short, long, default_value = "generated")]
        output: PathBuf,
        /// 应用模板
        #[arg(short, long, default_value = "minimal")]
        template: String,
        /// 编译目标
        #[arg(short, long, default_value = "wasm-gc")]
        target: String,
    },
    /// 列出可用模板
    #[command(name = "templates")]
    ListTemplates,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Create {
            description,
            output,
            template,
            list_templates,
        } => {
            if list_templates {
                cmd_list_templates();
                return ExitCode::SUCCESS;
            }
            cmd_create(&description, &output, &template)
        }
        Command::Build {
            dir,
            target,
            release,
            output,
            no_package,
        } => cmd_build(&dir, &target, release, output.as_deref(), no_package),
        Command::Go {
            description,
            output,
            template,
            target,
        } => cmd_go(&description, &output, &template, &target),
        Command::ListTemplates => {
            cmd_list_templates();
            return ExitCode::SUCCESS;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("错误: {e:#}");
            ExitCode::FAILURE
        }
    }
}

/// 列出可用模板。
fn cmd_list_templates() {
    println!("可用模板：");
    println!("{:-<40}", "");
    for (name, desc) in TEMPLATES {
        println!("  {:<15} {}", name, desc);
    }
    println!("{:-<40}", "");
    println!("使用: aiapp create \"描述\" -t <模板名>");
}

/// create：生成 MoonBit 工程（含 aiapp.json 清单）。
fn cmd_create(description: &str, output: &PathBuf, template: &str) -> Result<()> {
    let config = GenConfig::from_env();
    println!(
        "[1/2] 生成 MoonBit 工程（后端: {}, 模板: {}）...",
        match config.backend {
            aiapp_gen::Backend::Mock => "mock",
            aiapp_gen::Backend::OpenAi => "openai",
        },
        template
    );
    let source = generate_source(description, &config, template)
        .with_context(|| "AI 生成 MoonBit 源码失败")?;
    write_project(output, description, &source, template)
        .with_context(|| format!("写入工程目录 {} 失败", output.display()))?;
    println!("[2/2] 工程已生成: {}", output.display());
    println!("      清单: {}/aiapp.json", output.display());
    println!("      下一步: aiapp build {}", output.display());
    Ok(())
}

/// build：编译为 WASM 并打包为 .aiapp。
fn cmd_build(
    dir: &PathBuf,
    target: &str,
    release: bool,
    output: Option<&std::path::Path>,
    no_package: bool,
) -> Result<()> {
    let config = BuildConfig {
        target: target.into(),
        release,
        package: !no_package,
        output: None,
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

    if let Some(aiapp_path) = &out.aiapp {
        let aiapp_bytes = dir_size(aiapp_path);
        println!("      .aiapp 包: {} ({} bytes)", aiapp_path.display(), aiapp_bytes);
    }

    Ok(())
}

/// go：端到端 描述 → .aiapp 包。
fn cmd_go(description: &str, output: &PathBuf, template: &str, target: &str) -> Result<()> {
    let config = GenConfig::from_env();
    println!(
        "[1/3] 生成 MoonBit 工程（后端: {}, 模板: {}）...",
        match config.backend {
            aiapp_gen::Backend::Mock => "mock",
            aiapp_gen::Backend::OpenAi => "openai",
        },
        template
    );
    let source = generate_source(description, &config, template)
        .with_context(|| "AI 生成 MoonBit 源码失败")?;
    write_project(output, description, &source, template)
        .with_context(|| format!("写入工程目录 {} 失败", output.display()))?;
    println!("[2/3] 工程已生成: {}", output.display());

    let build_config = BuildConfig {
        target: target.into(),
        release: false,
        package: true,
        output: None,
    };
    println!("[3/3] 调用 moon build --target {target} ...");
    let out = build(output, &build_config)
        .with_context(|| format!("编译工程 {} 失败", output.display()))?;
    let bytes = std::fs::metadata(&out.wasm)?.len();
    let aiapp_msg = match &out.aiapp {
        Some(p) => format!(".aiapp 包: {}", p.display()),
        None => "".into(),
    };
    println!("      WASM 字节码: {} ({} bytes) {}", out.wasm.display(), bytes, aiapp_msg);
    println!("      完成！");
    Ok(())
}

/// 计算目录大小（字节）。
fn dir_size(path: &std::path::Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                total += std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}
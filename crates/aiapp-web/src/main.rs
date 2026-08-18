//! 员工自助生成 Web 原型后端。
//!
//! 能力：输入自然语言描述 + 选择模板 → 复用 aiapp-gen / aiapp-build 流水线
//! 生成 MoonBit 应用工程，返回源码与清单；可选调用 moon 编译为 `.aiapp` 包。

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::services::ServeDir;

use aiapp_gen::{generate_source, write_project, GenConfig, TEMPLATES};

/// 应用状态：共享工作目录。
#[derive(Clone)]
struct AppState {
    workdir: PathBuf,
}

/// 生成请求体。
#[derive(Deserialize)]
struct GenerateRequest {
    description: String,
    #[serde(default)]
    template: String,
    /// 是否尝试调用 moon 编译为 .aiapp（默认 false，避免无工具链时报错）。
    #[serde(default)]
    build: bool,
}

/// 生成/构建响应。
#[derive(Serialize)]
struct GenerateResponse {
    ok: bool,
    /// 生成的 MoonBit 源码。
    source: String,
    /// 应用清单（aiapp.json 内容）。
    manifest: serde_json::Value,
    /// 工程目录名。
    project_dir: String,
    /// 编译信息（build=true 且成功时）.
    build_result: Option<String>,
    /// 错误信息（ok=false 时）。
    error: Option<String>,
}

/// 模板列表响应。
#[derive(Serialize)]
struct TemplatesResponse {
    templates: Vec<TemplateInfo>,
}

#[derive(Serialize)]
struct TemplateInfo {
    name: &'static str,
    description: &'static str,
    image: String,
}

/// 构建路由。
pub fn router() -> Router {
    let workdir = std::env::current_dir()
        .expect("获取当前目录失败")
        .join("generated_web");
    std::fs::create_dir_all(&workdir).expect("创建工作目录失败");
    let state = Arc::new(AppState { workdir });

    // 模板图片目录（编译时获取绝对路径）
    let static_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/templates");
    let serve_static = ServeDir::new(static_dir);

    Router::new()
        .route("/", get(serve_index))
        .route("/api/templates", get(list_templates))
        .route("/api/generate", post(generate))
        .nest_service("/static/templates", serve_static)
        .with_state(state)
}

/// 服务首页（内嵌前端）。
async fn serve_index() -> axum::response::Html<&'static str> {
    axum::response::Html(include_str!("index.html"))
}

/// 列出可用模板。
async fn list_templates() -> Json<TemplatesResponse> {
    let templates = TEMPLATES
        .iter()
        .map(|(name, description)| TemplateInfo {
            name,
            description,
            image: &format!("/static/templates/{}.png", name),
        })
        .collect();
    Json(TemplatesResponse { templates })
}

/// 生成应用工程。
async fn generate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GenerateRequest>,
) -> Json<GenerateResponse> {
    let description = req.description.trim().to_string();
    if description.is_empty() {
        return Json(GenerateResponse {
            ok: false,
            source: String::new(),
            manifest: serde_json::Value::Null,
            project_dir: String::new(),
            build_result: None,
            error: Some("请先描述你要做的应用".into()),
        });
    }

    // 未指定模板时默认 minimal
    let template = if req.template.is_empty() {
        "minimal"
    } else {
        &req.template
    };

    // 用一个唯一子目录存放本次生成的工程
    let slug = aiapp_gen::slugify(&description);
    let dir_name = format!(
        "{}_{}",
        slug,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    );
    let project_dir = state.workdir.join(&dir_name);

    let config = GenConfig::from_env();

    let result = match generate_source(&description, &config, template) {
        Ok(source) => write_project(&project_dir, &description, &source, template)
            .map(|()| source)
            .map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };

    match result {
        Ok(source) => {
            // 读取清单
            let manifest_path = project_dir.join("aiapp.json");
            let manifest = std::fs::read_to_string(&manifest_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or(serde_json::json!({}));

            // 可选编译
            let mut build_result = None;
            if req.build {
                match moon_build(&project_dir) {
                    Ok(info) => build_result = Some(info),
                    Err(e) => build_result = Some(format!("编译跳过：{e}")),
                }
            }

            Json(GenerateResponse {
                ok: true,
                source,
                manifest,
                project_dir: dir_name,
                build_result,
                error: None,
            })
        }
        Err(e) => Json(GenerateResponse {
            ok: false,
            source: String::new(),
            manifest: serde_json::Value::Null,
            project_dir: String::new(),
            build_result: None,
            error: Some(e),
        }),
    }
}

/// 尝试调用 moon 编译工程为 .aiapp 包。
fn moon_build(project_dir: &std::path::Path) -> Result<String, String> {
    let config = aiapp_build::BuildConfig {
        target: "wasm-gc".into(),
        release: false,
        package: true,
        output: None,
    };
    let out = aiapp_build::build(project_dir, &config).map_err(|e| e.to_string())?;
    let bytes = std::fs::metadata(&out.wasm)
        .map(|m| m.len())
        .unwrap_or(0);
    let mut msg = format!("已编译 WASM（{} bytes）", bytes);
    if let Some(aiapp_dir) = &out.aiapp {
        msg.push_str(&format!("，\\.aiapp 包：{}", aiapp_dir.display()));
    }
    Ok(msg)
}

/// 启动服务。
#[tokio::main]
async fn main() {
    let app = router();
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("aiapp-mb 原型已启动：http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("端口 8080 被占用");
    axum::serve(listener, app).await.expect("服务异常退出");
}
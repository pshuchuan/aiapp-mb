//! 员工自助生成 Web 原型后端。
//!
//! 能力：输入自然语言描述 + 选择模板 → 复用 aiapp-gen / aiapp-build 流水线
//! 生成 MoonBit 应用工程，返回源码与清单；可选调用 moon 编译为 `.aiapp` 包。

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tokio::sync::Mutex;

use aiapp_gen::{generate_source, write_project, GenConfig, TEMPLATES};

/// 应用市场条目。
#[derive(Clone, Serialize, Deserialize)]
struct MarketApp {
    id: String,
    name: String,
    description: String,
    tags: Vec<String>,
    platforms: Vec<String>,
    template: String,
    source: String,
    created_at: String,
}

/// 应用状态：共享工作目录 + 市场列表。
#[derive(Clone)]
struct AppState {
    workdir: PathBuf,
    market: Arc<Mutex<Vec<MarketApp>>>,
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
    source: String,
    manifest: serde_json::Value,
    project_dir: String,
    build_result: Option<String>,
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

/// 市场列表响应。
#[derive(Serialize)]
struct MarketResponse {
    apps: Vec<MarketApp>,
    tags: Vec<String>,
    platforms: Vec<String>,
}

/// 应用详情响应。
#[derive(Serialize)]
struct AppDetailResponse {
    ok: bool,
    app: Option<MarketApp>,
    /// 模拟应用内容（基于模板）。
    mock_content: Option<String>,
    error: Option<String>,
}

/// 预置示例应用。
fn seed_market_apps() -> Vec<MarketApp> {
    vec![
        MarketApp {
            id: "daily-sales".into(),
            name: "每日销售看板".into(),
            description: "按日期汇总销售额，展示趋势图和TOP商品排行".into(),
            tags: vec!["数据看板".into(), "办公效率".into()],
            platforms: vec!["网页".into(), "手机".into(), "车机".into()],
            template: "calculator".into(),
            source: String::new(),
            created_at: "2026-08-18 10:00".into(),
        },
        MarketApp {
            id: "todo-team".into(),
            name: "团队待办协作".into(),
            description: "同事间共享待办清单，分配任务、跟踪进度".into(),
            tags: vec!["办公效率".into(), "协作".into()],
            platforms: vec!["网页".into(), "手机".into(), "电脑".into(), "鸿蒙".into()],
            template: "todo".into(),
            source: String::new(),
            created_at: "2026-08-18 09:30".into(),
        },
        MarketApp {
            id: "photo-filter".into(),
            name: "极速图片滤镜".into(),
            description: "一键给图片添加滤镜效果，支持灰度、暖色、冷色等".into(),
            tags: vec!["创意工具".into(), "实用工具".into()],
            platforms: vec!["网页".into(), "手机".into(), "电脑".into()],
            template: "image-filter".into(),
            source: String::new(),
            created_at: "2026-08-17 16:20".into(),
        },
        MarketApp {
            id: "expense-tracker".into(),
            name: "月度开支记账".into(),
            description: "记录日常开支，分类统计，生成月度报表".into(),
            tags: vec!["实用工具".into(), "数据看板".into()],
            platforms: vec!["网页".into(), "手机".into(), "车机".into(), "电视盒".into()],
            template: "calculator".into(),
            source: String::new(),
            created_at: "2026-08-17 14:00".into(),
        },
        MarketApp {
            id: "inventory-check".into(),
            name: "库存盘点助手".into(),
            description: "扫码或手动录入库存数据，自动比对差异".into(),
            tags: vec!["行业应用".into(), "实用工具".into()],
            platforms: vec!["手机".into(), "车机".into()],
            template: "minimal".into(),
            source: String::new(),
            created_at: "2026-08-16 11:10".into(),
        },
        MarketApp {
            id: "meeting-notes".into(),
            name: "会议纪要整理".into(),
            description: "记录会议要点，自动生成待办事项和会议结论".into(),
            tags: vec!["办公效率".into(), "协作".into()],
            platforms: vec!["网页".into(), "电脑".into(), "鸿蒙".into()],
            template: "todo".into(),
            source: String::new(),
            created_at: "2026-08-16 09:00".into(),
        },
        MarketApp {
            id: "weather-dash".into(),
            name: "天气仪表盘".into(),
            description: "实时天气展示，包含温度曲线、风速、湿度等信息".into(),
            tags: vec!["数据看板".into(), "实用工具".into()],
            platforms: vec!["网页".into(), "手机".into(), "车机".into(), "电视盒".into(), "鸿蒙".into()],
            template: "calculator".into(),
            source: String::new(),
            created_at: "2026-08-15 08:30".into(),
        },
        MarketApp {
            id: "timer-app".into(),
            name: "多任务计时器".into(),
            description: "同时运行多个倒计时提醒，适合厨房、运动等场景".into(),
            tags: vec!["实用工具".into()],
            platforms: vec!["手机".into(), "车机".into(), "电视盒".into()],
            template: "minimal".into(),
            source: String::new(),
            created_at: "2026-08-14 19:45".into(),
        },
    ]
}

/// 构建路由。
pub fn router() -> Router {
    let workdir = std::env::current_dir()
        .expect("获取当前目录失败")
        .join("generated_web");
    std::fs::create_dir_all(&workdir).expect("创建工作目录失败");
    let market = Arc::new(Mutex::new(seed_market_apps()));
    let state = Arc::new(AppState { workdir, market });

    // 模板图片目录（编译时获取绝对路径）
    let static_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/templates");
    let serve_static = ServeDir::new(static_dir);

    Router::new()
        .route("/", get(serve_index))
        .route("/api/templates", get(list_templates))
        .route("/api/generate", post(generate))
        .route("/api/market", get(list_market))
        .route("/api/market/:id", get(app_detail))
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
            image: format!("/static/templates/{}.png", name),
        })
        .collect();
    Json(TemplatesResponse { templates })
}

/// 获取应用市场列表。
async fn list_market(State(state): State<Arc<AppState>>) -> Json<MarketResponse> {
    let apps = state.market.lock().await.clone();
    // 收集所有去重标签
    let mut tags: Vec<String> = apps
        .iter()
        .flat_map(|a| a.tags.clone())
        .collect();
    tags.sort();
    tags.dedup();
    // 收集所有去重平台
    let mut platforms: Vec<String> = apps
        .iter()
        .flat_map(|a| a.platforms.clone())
        .collect();
    platforms.sort();
    platforms.dedup();
    Json(MarketResponse { apps, tags, platforms })
}

/// 获取应用详情（打开应用时调用）。
async fn app_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<AppDetailResponse> {
    let market = state.market.lock().await;
    let app = market.iter().find(|a| a.id == id).cloned();
    match app {
        Some(app) => {
            let mock_content = Some(mock_app_content(&app.template, &app.name));
            Json(AppDetailResponse { ok: true, app: Some(app), mock_content, error: None })
        }
        None => Json(AppDetailResponse {
            ok: false,
            app: None,
            mock_content: None,
            error: Some("应用不存在".into()),
        }),
    }
}

/// 根据模板生成模拟的应用内容展示。
fn mock_app_content(template: &str, name: &str) -> String {
    match template {
        "calculator" => format!(
            "应用「{}」已启动 [计算器模式]\n\n\
             ╔═══════════════╗\n\
             ║     0         ║\n\
             ╠═══════════════╣\n\
             ║ 7 │ 8 │ 9 │ + ║\n\
             ║───┼───┼───┼───╢\n\
             ║ 4 │ 5 │ 6 │ - ║\n\
             ║───┼───┼───┼───╢\n\
             ║ 1 │ 2 │ 3 │ × ║\n\
             ║───┼───┼───┼───╢\n\
             ║ 0 │ . │ = │ ÷ ║\n\
             ╚═══════════════╝\n\n\
             支持键盘输入，按 Esc 退出应用",
            name
        ),
        "todo" => format!(
            "应用「{}」已启动 [待办事项]\n\n\
             ┌─────────────────────────────┐\n\
             │ 📋 我的待办              +  │\n\
             ├─────────────────────────────┤\n\
             │ ☑ 完成周报              ✓  │\n\
             │ ☐ 准备会议材料          ○  │\n\
             │ ☐ 回复客户邮件          ○  │\n\
             │ ☐ 更新项目进度          ○  │\n\
             │ ☐ 团队代码审查          ○  │\n\
             └─────────────────────────────┘\n\n\
             点击 ○ 勾选完成，按 + 添加新事项",
            name
        ),
        "image-filter" => format!(
            "应用「{}」已启动 [图片滤镜]\n\n\
             ┌─────────────────────────────┐\n\
             │ 🖼 图片滤镜              ⚙  │\n\
             ├─────────────────────────────┤\n\
             │                             │\n\
             │    [ 点击选择图片 ]         │\n\
             │                             │\n\
             ├─────────────────────────────┤\n\
             │ 原图 │ 灰度 │ 暖色 │ 冷色 │\n\
             └─────────────────────────────┘\n\n\
             选择滤镜效果，实时预览处理结果",
            name
        ),
        _ => format!(
            "应用「{}」已启动 [Hello World]\n\n\
             ┌─────────────────────────────┐\n\
             │ 欢迎使用 {}       │\n\
             ├─────────────────────────────┤\n\
             │                             │\n\
             │    ✨ 应用运行中 ✨          │\n\
             │                             │\n\
             │    当前版本: 1.0.0          │\n\
             │    状态: 正常运行           │\n\
             │                             │\n\
             └─────────────────────────────┘\n\n\
             该应用已就绪，可在设定的平台上运行",
            name, name
        ),
    }
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

            // 存入应用市场
            {
                let mut market = state.market.lock().await;
                let now = chrono_now();
                let app_id = format!("gen_{}", market.len() + 1);
                // 从清单中提取信息
                let name = manifest.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&description)
                    .to_string();
                let app_entry = MarketApp {
                    id: app_id,
                    name,
                    description: description.clone(),
                    tags: vec!["办公效率".into()], // 默认标签，后续可让用户选择
                    platforms: vec!["网页".into(), "手机".into(), "电脑".into()],
                    template: template.to_string(),
                    source: source.clone(),
                    created_at: now,
                };
                market.push(app_entry);
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

/// 获取当前时间字符串。
fn chrono_now() -> String {
    // 不用 chrono crate，手动格式
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = d.as_secs();
    // 简单计算年月日时分
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let mins = (time_of_day % 3600) / 60;
    // 从 1970-01-01 算起
    let year = 1970 + (days as f64 / 365.25) as u64;
    format!("{}-{:02}-{:02} {:02}:{:02}", year, 1 + (days / 30) % 12, 1 + days % 30, hours, mins)
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
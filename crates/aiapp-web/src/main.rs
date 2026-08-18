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
    /// 版本号，如 "1.0.0"。
    version: String,
    /// 所有者标识。内置示例为 "官方"；用户生成的应用所有者为 "me"。
    owner: String,
    /// 可见性："public" 已发布到市场；"private" 仅所有者可见使用。
    visibility: String,
}

/// 当前登录用户（单用户模拟）。
const OWNER_ME: &str = "me";

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
    /// 生成模式："new" 新建应用（默认）；"update" 更新既有应用。
    #[serde(default = "default_mode")]
    mode: String,
    /// 更新模式下要更新的应用 id（须为当前用户所有）。
    #[serde(default)]
    target_id: String,
}

fn default_mode() -> String {
    "new".to_string()
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
    /// 新建模式返回新建的应用；更新模式返回更新后的应用。
    app: Option<MarketApp>,
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

/// 我的应用列表响应。
#[derive(Serialize)]
struct MyAppsResponse {
    apps: Vec<MarketApp>,
}

/// 发布/更新请求体。
#[derive(Deserialize)]
struct PublishRequest {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    platforms: Vec<String>,
}

/// 操作结果响应。
#[derive(Serialize)]
struct ActionResponse {
    ok: bool,
    error: Option<String>,
    app: Option<MarketApp>,
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
    fn app(
        id: &str, name: &str, description: &str, tags: &[&str], platforms: &[&str],
        template: &str, version: &str, owner: &str, visibility: &str,
    ) -> MarketApp {
        MarketApp {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
            platforms: platforms.iter().map(|s| s.to_string()).collect(),
            template: template.into(),
            source: String::new(),
            created_at: "2026-08-18 10:00".into(),
            version: version.into(),
            owner: owner.into(),
            visibility: visibility.into(),
        }
    }
    vec![
        app("expense-book", "日常记账本",
            "随手记下每笔收支，按类别自动汇总，月末生成清晰报表",
            &["实用工具", "数据看板"], &["网页", "手机", "macOS", "Windows", "鸿蒙"],
            "calculator", "1.0.0", "官方", "public"),
        app("todo-team", "团队待办协作",
            "和同事共享待办清单，分配任务、设置截止时间、跟踪完成进度",
            &["办公效率", "协作"], &["网页", "手机", "电脑", "鸿蒙"],
            "todo", "1.0.0", "官方", "public"),
        app("money-calculator", "全能计算器",
            "四则运算、百分比例、开方平方，界面清爽，适合日常办公和购物比价",
            &["实用工具"], &["网页", "手机", "macOS", "Windows", "车机", "电视盒"],
            "calculator", "1.0.0", "官方", "public"),
        app("photo-filter", "照片滤镜工坊",
            "一键给照片添加灰度、暖色调、冷色调等滤镜，还能调节亮度和对比度",
            &["创意工具", "实用工具"], &["网页", "手机", "macOS", "Windows", "鸿蒙"],
            "image-filter", "1.0.0", "官方", "public"),
        app("meeting-notes", "会议纪要整理",
            "记录会议关键要点，自动拆出待办事项和责任人，会后一键同步给团队",
            &["办公效率", "协作"], &["网页", "电脑", "macOS", "Windows", "鸿蒙"],
            "todo", "1.0.0", "官方", "public"),
        app("daily-weather", "今日出行天气",
            "实时天气与七日内预报，包含温度曲线、风力、紫外线，出门前看一眼",
            &["出行导航", "实用工具"], &["车机", "手机", "电视盒"],
            "minimal", "1.0.0", "官方", "public"),
        app("car-showroom", "车商展厅",
            "车辆参数、报价一览，支持车型对比，适合在门店或车上给客户演示",
            &["行业应用", "数据看板"], &["车机", "电视盒"],
            "minimal", "1.0.0", "官方", "public"),
        app("family-movie", "家庭影音点播",
            "全家共享的影片片单，按类型分类浏览，遥控器即可操作，字体大不费眼",
            &["影音娱乐", "无障碍"], &["电视盒", "车机"],
            "image-filter", "1.0.0", "官方", "public"),
        app("grandma-menu", "老人简易菜单",
            "超大字体、极简按钮，常用功能一键直达，方便老人轻松使用电视盒",
            &["无障碍", "影音娱乐"], &["电视盒", "车机", "鸿蒙"],
            "minimal", "1.0.0", "官方", "public"),
        app("route-navi", "沿途兴趣点导航",
            "根据当前行程推荐沿路的加油站、充电站、餐厅和景点",
            &["出行导航", "行业应用"], &["车机", "手机"],
            "minimal", "1.0.0", "官方", "public"),
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
        .route("/api/my-apps", get(list_my_apps))
        .route("/api/publish", post(publish_app))
        .route("/api/delete", post(delete_app))
        .route("/api/app/:id", get(app_detail))
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

/// 获取应用市场列表（仅公开应用）。
async fn list_market(State(state): State<Arc<AppState>>) -> Json<MarketResponse> {
    let all = state.market.lock().await.clone();
    let apps: Vec<MarketApp> = all
        .iter()
        .filter(|a| a.visibility == "public")
        .cloned()
        .collect();
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

/// 我的应用列表（当前用户拥有，含私有与已发布）。
async fn list_my_apps(State(state): State<Arc<AppState>>) -> Json<MyAppsResponse> {
    let all = state.market.lock().await.clone();
    let apps = all
        .into_iter()
        .filter(|a| a.owner == OWNER_ME)
        .collect();
    Json(MyAppsResponse { apps })
}

/// 发布应用：补充基础信息后公开到市场。
async fn publish_app(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PublishRequest>,
) -> Json<ActionResponse> {
    let mut market = state.market.lock().await;
    let idx = match market.iter().position(|a| a.id == req.id && a.owner == OWNER_ME) {
        Some(i) => i,
        None => {
            return Json(ActionResponse {
                ok: false,
                error: Some("应用不存在或无权操作".into()),
                app: None,
            })
        }
    };
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Json(ActionResponse {
            ok: false,
            error: Some("请填写应用名称".into()),
            app: None,
        });
    }
    let app = &mut market[idx];
    app.name = name;
    app.description = req.description.trim().to_string();
    app.tags = req.tags.iter().filter(|s| !s.is_empty()).cloned().collect();
    app.platforms = req.platforms.iter().filter(|s| !s.is_empty()).cloned().collect();
    if app.platforms.is_empty() {
        app.platforms = vec!["网页".into(), "手机".into(), "电脑".into()];
    }
    app.visibility = "public".into();
    let cloned = app.clone();
    Json(ActionResponse { ok: true, error: None, app: Some(cloned) })
}

/// 删除应用（仅限自己的应用）。
async fn delete_app(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PublishRequest>,
) -> Json<ActionResponse> {
    let mut market = state.market.lock().await;
    match market.iter().position(|a| a.id == req.id && a.owner == OWNER_ME) {
        Some(i) => {
            let removed = market.remove(i);
            Json(ActionResponse { ok: true, error: None, app: Some(removed) })
        }
        None => Json(ActionResponse {
            ok: false,
            error: Some("应用不存在或无权操作".into()),
            app: None,
        }),
    }
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

/// 生成应用工程（新建或更新）。
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
            app: None,
        });
    }

    // 未指定模板时默认 minimal
    let template = if req.template.is_empty() {
        "minimal"
    } else {
        &req.template
    };

    // 更新模式：先校验应用归属
    let update_target: Option<MarketApp> = if req.mode == "update" {
        if req.target_id.is_empty() {
            return Json(GenerateResponse {
                ok: false,
                source: String::new(),
                manifest: serde_json::Value::Null,
                project_dir: String::new(),
                build_result: None,
                error: Some("更新应用需要指定要更新的应用".into()),
                app: None,
            });
        }
        let market = state.market.lock().await;
        let found = market
            .iter()
            .find(|a| a.id == req.target_id && a.owner == OWNER_ME)
            .cloned();
        match found {
            Some(a) => Some(a),
            None => {
                return Json(GenerateResponse {
                    ok: false,
                    source: String::new(),
                    manifest: serde_json::Value::Null,
                    project_dir: String::new(),
                    build_result: None,
                    error: Some("找不到要更新的应用，或你不是它的所有者".into()),
                    app: None,
                })
            }
        }
    } else {
        None
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

            let now = chrono_now();
            let app_result = {
                let mut market = state.market.lock().await;
                if let Some(target) = &update_target {
                    // 更新模式：版本 +1，所有者与可见性保持，更新源码与描述
                    let idx = market
                        .iter()
                        .position(|a| a.id == target.id)
                        .expect("目标应用应存在");
                    let app = &mut market[idx];
                    app.source = source.clone();
                    app.description = description.clone();
                    app.template = template.to_string();
                    app.version = bump_version(&app.version);
                    app.created_at = now;
                    app.clone()
                } else {
                    // 新建模式：默认为当前用户私有，可自行使用
                    let name = manifest.get("name")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.trim().is_empty())
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|| truncate(&description, 16));
                    let app_id = format!("gen_{}", market.len() + 1);
                    let app_entry = MarketApp {
                        id: app_id.clone(),
                        name,
                        description: description.clone(),
                        tags: vec![],          // 发布时填写
                        platforms: vec!["网页".into(), "手机".into(), "电脑".into()],
                        template: template.to_string(),
                        source: source.clone(),
                        created_at: now,
                        version: "1.0.0".into(),
                        owner: OWNER_ME.into(),
                        visibility: "private".into(),
                    };
                    let cloned = app_entry.clone();
                    market.push(app_entry);
                    cloned
                }
            };

            Json(GenerateResponse {
                ok: true,
                source,
                manifest,
                project_dir: dir_name,
                build_result,
                error: None,
                app: Some(app_result),
            })
        }
        Err(e) => Json(GenerateResponse {
            ok: false,
            source: String::new(),
            manifest: serde_json::Value::Null,
            project_dir: String::new(),
            build_result: None,
            error: Some(e),
            app: None,
        }),
    }
}

/// 简单截断长字符串。
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max).collect::<String>() + "..."
    }
}

/// 版本号递增：1.2.3 -> 1.2.4。
fn bump_version(v: &str) -> String {
    let parts: Vec<&str> = v.trim().split('.').collect();
    let major = parts.first().unwrap_or(&"1").parse::<u32>().unwrap_or(1);
    let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    format!("{major}.{minor}.{}", patch + 1)
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
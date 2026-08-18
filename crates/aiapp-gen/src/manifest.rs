//! .aiapp 统一应用包清单格式定义。
//!
//! 每个 AI 生成的应用都包含一个 `aiapp.json` 清单文件，描述应用的元数据、
//! 权限声明和版本信息。这是平台统一的 app 包格式。

use serde::{Deserialize, Serialize};

/// 统一应用包清单。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManifest {
    /// 唯一应用 ID，格式 `app_<slug>`。
    pub app_id: String,
    /// 应用名称（由描述推导）。
    pub name: String,
    /// 语义版本号。
    pub version: String,
    /// 应用描述（用户输入的自然语言描述）。
    pub description: String,
    /// 所需宿主 SDK 的最低版本。
    pub host_sdk_version: String,
    /// 运行时权限声明。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
    /// 应用模板类型。
    pub template: String,
    /// 创建时间（RFC 3339）。
    pub created_at: String,
}

impl AppManifest {
    /// 从应用描述和模板类型构建清单。
    pub fn new(desc: &str, template: &str, slug: &str) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // 使用时间戳哈希生成简短 app_id
        let hash = (now as u64).wrapping_mul(2654435761).to_string();
        let app_id = format!("app_{}", &hash[..8.min(hash.len())]);
        Self {
            app_id,
            name: slug.replace('_', " "),
            version: "0.1.0".into(),
            description: desc.to_string(),
            host_sdk_version: ">=0.1.0".into(),
            permissions: default_permissions(template),
            template: template.to_string(),
            created_at: now_to_rfc3339(now),
        }
    }

    /// 序列化为格式化的 JSON 字符串。
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("序列化 AppManifest 失败")
    }
}

/// 根据模板类型返回默认权限声明。
fn default_permissions(template: &str) -> Vec<String> {
    match template {
        "todo" => vec!["storage".into()],
        "image-filter" => vec!["storage".into()],
        _ => vec![],
    }
}

fn now_to_rfc3339(secs: u64) -> String {
    // 从 UNIX epoch 计算日期
    let days = secs / 86400;
    let remaining = secs % 86400;
    let h = remaining / 3600;
    let m = (remaining % 3600) / 60;
    let s = remaining % 60;
    let (year, month, day) = days_to_date(days as u32);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, h, m, s)
}

/// 将 UNIX epoch 天数转换为日历日期。
fn days_to_date(mut days: u32) -> (u32, u32, u32) {
    // 简化：从 1970-01-01 开始逐月累加
    let mut y = 1970u32;
    let mut m = 1u32;
    loop {
        let dim = days_in_month(y, m);
        if days < dim {
            break;
        }
        days -= dim;
        m += 1;
        if m > 12 {
            m = 1;
            y += 1;
        }
    }
    (y, m, days + 1) // day is 1-based
}

fn days_in_month(y: u32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}
//! 本地 mock 后端：不依赖外部 AI，返回固定示例源码，便于离线演示与测试。

use crate::GenError;

/// 返回一段可直接运行的 MoonBit 示例程序。
pub fn generate(desc: &str) -> Result<String, GenError> {
    let note = desc.trim();
    let line = if note.is_empty() {
        "  println(\"Hello from aiapp-mb\")\n".to_string()
    } else {
        format!("  println(\"aiapp-mb demo: {}\")\n", sanitize(note))
    };
    Ok(format!(
        "///| 由 aiapp-mb mock 后端生成，描述：{note}\nfn main {{\n{line}}}\n"
    ))
}

/// 去除会破坏字符串字面量的字符。
fn sanitize(s: &str) -> String {
    s.replace('"', "'").replace('\\', "")
}

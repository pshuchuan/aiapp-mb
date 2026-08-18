//! MoonBit 应用模板库。
//!
//! 提供多种预定义的 MoonBit 应用模板，作为 AI 生成的基础和示例。
//! Phase 1 支持：minimal（最小模板）、calculator（计算器）、todo（待办事项）、image-filter（图片滤镜）。

/// 可用模板列表。
pub const TEMPLATES: &[(&str, &str)] = &[
    ("minimal", "最小模板：Hello World 风格应用"),
    ("calculator", "计算器：支持四则运算的交互式计算器"),
    ("todo", "待办事项：支持增删改查的待办事项管理"),
    ("image-filter", "图片滤镜：图片滤镜处理应用"),
];

/// 根据模板名称返回对应的 MoonBit 源码。
pub fn get_template_source(name: &str, desc: &str) -> Option<String> {
    let source = match name {
        "minimal" => minimal_template(desc),
        "calculator" => calculator_template(desc),
        "todo" => todo_template(desc),
        "image-filter" => image_filter_template(desc),
        _ => return None,
    };
    Some(source)
}

/// 生成最小模板的 MoonBit 源码。
fn minimal_template(desc: &str) -> String {
    format!(
        r#"///| 应用：{desc}
///| 模板：minimal
///| 由 aiapp-mb 生成

fn main {{
  println("Hello from aiapp-mb!")
  println("描述：{sanitized}")
}}

/// 应用信息
pub fn app_info() -> String {{
  "aiapp-mb minimal app: {sanitized}"
}}
"#,
        desc = desc,
        sanitized = sanitize(desc)
    )
}

/// 生成计算器模板的 MoonBit 源码。
fn calculator_template(desc: &str) -> String {
    format!(
        r#"///| 应用：{desc}
///| 模板：calculator
///| 由 aiapp-mb 生成

/// 计算器：支持基本四则运算
fn main {{
  println("=== 计算器 ===")
  println("描述：{sanitized}")
  println("支持的运算：+ - * /")
  println("示例：3 + 4 = 7")
}}

/// 加法
pub fn add(a: Int, b: Int) -> Int {{
  a + b
}}

/// 减法
pub fn sub(a: Int, b: Int) -> Int {{
  a - b
}}

/// 乘法
pub fn mul(a: Int, b: Int) -> Int {{
  a * b
}}

/// 除法（返回整数结果）
pub fn div(a: Int, b: Int) -> Int {{
  a / b
}}

/// 应用信息
pub fn app_info() -> String {{
  "aiapp-mb calculator app: {sanitized}"
}}
"#,
        sanitized = sanitize(desc)
    )
}

/// 生成待办事项模板的 MoonBit 源码。
fn todo_template(desc: &str) -> String {
    format!(
        r#"///| 应用：{desc}
///| 模板：todo
///| 由 aiapp-mb 生成

/// 待办事项条目
struct Todo {{
  id: Int
  title: String
  done: Bool
}}

/// 待办事项列表
struct TodoList {{
  todos: Array[Todo]
  next_id: Int
}}

/// 创建一个空的待办列表
pub fn TodoList::new() -> TodoList {{
  TodoList {{ todos: [], next_id: 1 }}
}}

/// 添加待办事项
pub fn add(self: TodoList, title: String) {{
  let todo = Todo {{ id: self.next_id, title: title, done: false }}
  self.todos.push(todo)
  self.next_id = self.next_id + 1
}}

/// 标记为已完成
pub fn complete(self: TodoList, id: Int) {{
  for todo in self.todos {{
    if todo.id == id {{
      todo.done = true
    }}
  }}
}}

/// 列出所有待办事项
pub fn list(self: TodoList) -> String {{
  let mut result = "待办事项：\n"
  for todo in self.todos {{
    let status = if todo.done {{ "[✓]" }} else {{ "[ ]" }}
    result = result + status + " " + todo.id.to_string() + ". " + todo.title + "\n"
  }}
  result
}}

fn main {{
  println("=== 待办事项 ===")
  println("描述：{sanitized}")
  let list = TodoList::new()
  list.add("学习 MoonBit")
  list.add("生成 WASM")
  list.add("部署应用")
  list.complete(1)
  println(list.list())
}}

/// 应用信息
pub fn app_info() -> String {{
  "aiapp-mb todo app: {sanitized}"
}}
"#,
        sanitized = sanitize(desc)
    )
}

/// 生成图片滤镜模板的 MoonBit 源码。
fn image_filter_template(desc: &str) -> String {
    format!(
        r#"///| 应用：{desc}
///| 模板：image-filter
///| 由 aiapp-mb 生成

/// 图片像素
struct Pixel {{
  r: Int
  g: Int
  b: Int
}}

/// 图片
struct Image {{
  width: Int
  height: Int
  pixels: Array[Array[Pixel]]
}}

/// 创建一张测试图片
pub fn make_test_image(w: Int, h: Int) -> Image {{
  let pixels = Array::make(h, Array::make(w, Pixel {{ r: 0, g: 0, b: 0 }}))
  for i in 0..h {{
    for j in 0..w {{
      pixels[i][j] = Pixel {{
        r: ((i * 255) / h).to_int(),
        g: ((j * 255) / w).to_int(),
        b: ((i + j) * 255 / (w + h)).to_int(),
      }}
    }}
  }}
  Image {{ width: w, height: h, pixels: pixels }}
}}

/// 灰度滤镜
pub fn to_grayscale(img: Image) -> Image {{
  for i in 0..img.height {{
    for j in 0..img.width {{
      let p = img.pixels[i][j]
      let gray = (p.r + p.g + p.b) / 3
      img.pixels[i][j] = Pixel {{ r: gray, g: gray, b: gray }}
    }}
  }}
  img
}}

/// 反转颜色
pub fn invert(img: Image) -> Image {{
  for i in 0..img.height {{
    for j in 0..img.width {{
      let p = img.pixels[i][j]
      img.pixels[i][j] = Pixel {{ r: 255 - p.r, g: 255 - p.g, b: 255 - p.b }}
    }}
  }}
  img
}}

fn main {{
  println("=== 图片滤镜 ===")
  println("描述：{sanitized}")
  let img = make_test_image(4, 4)
  let w = img.width
  let h = img.height
  println("原始图片：" + w.to_string() + "x" + h.to_string())
  let gray = to_grayscale(img)
  println("灰度滤镜已应用")
  let inverted = invert(gray)
  println("颜色反转已应用")
  println("滤镜处理完成！")
}}

/// 应用信息
pub fn app_info() -> String {{
  "aiapp-mb image-filter app: {sanitized}"
}}
"#,
        sanitized = sanitize(desc)
    )
}

/// 去除会破坏字符串字面量的字符。
fn sanitize(s: &str) -> String {
    s.replace('"', "'").replace('\\', "").replace('\n', " ")
}
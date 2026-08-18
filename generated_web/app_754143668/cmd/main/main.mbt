///| 应用：统计本月到账金额的小应用
///| 模板：calculator
///| 由 aiapp-mb 生成

/// 计算器：支持基本四则运算
fn main {
  println("=== 计算器 ===")
  println("描述：统计本月到账金额的小应用")
  println("支持的运算：+ - * /")
  println("示例：3 + 4 = 7")
}

/// 加法
pub fn add(a: Int, b: Int) -> Int {
  a + b
}

/// 减法
pub fn sub(a: Int, b: Int) -> Int {
  a - b
}

/// 乘法
pub fn mul(a: Int, b: Int) -> Int {
  a * b
}

/// 除法（返回整数结果）
pub fn div(a: Int, b: Int) -> Int {
  a / b
}

/// 应用信息
pub fn app_info() -> String {
  "aiapp-mb calculator app: 统计本月到账金额的小应用"
}

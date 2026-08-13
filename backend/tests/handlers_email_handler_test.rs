use bingxi_backend::handlers::email_handler::*;

#[test]
fn 测试渲染模板_基本替换() {
    let template = "你好 {{name}}，订单号 {{order_no}} 已确认";
    let params = serde_json::json!({"name": "张三", "order_no": "ORD001"});
    let result = render_template(template, &params);
    assert_eq!(result, "你好 张三，订单号 ORD001 已确认");
}

#[test]
fn 测试渲染模板_带空格占位符() {
    let template = "你好 {{ name }}，订单号 {{ order_no }} 已确认";
    let params = serde_json::json!({"name": "李四", "order_no": "ORD002"});
    let result = render_template(template, &params);
    assert_eq!(result, "你好 李四，订单号 ORD002 已确认");
}

#[test]
fn 测试渲染模板_未匹配占位符保持原样() {
    let template = "你好 {{name}}，{{unknown_key}}";
    let params = serde_json::json!({"name": "王五"});
    let result = render_template(template, &params);
    assert_eq!(result, "你好 王五，{{unknown_key}}");
}

#[test]
fn 测试渲染模板_非字符串值使用_json表示() {
    let template = "数量：{{count}}，金额：{{amount}}";
    let params = serde_json::json!({"count": 100, "amount": 99.5});
    let result = render_template(template, &params);
    assert_eq!(result, "数量：100，金额：99.5");
}

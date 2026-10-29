//! HTML 转义与危险标记检测工具用例
//!
//! 用例全部为纯构造型，不连库、不发网络请求。

use bingxi_backend::utils::html_escape::{
    MarkupViolation, ensure_markup_safe, escape_html, scan_dangerous_markup,
};

const 恶意供应商名: &str = "<img src=x onerror=alert(1)>";

#[test]
fn 测试转义_恶意标签落为文本() {
    let escaped = escape_html(恶意供应商名);
    assert!(
        !escaped.contains('<') && !escaped.contains('>'),
        "转义结果仍含字面尖括号：{}",
        escaped
    );
    assert!(
        escaped.contains("&lt;img") && escaped.contains("&gt;"),
        "未按 HTML 文本上下文转义：{}",
        escaped
    );
}

#[test]
fn 测试转义_引号与斜杠一并转义() {
    assert_eq!(escape_html("a\"b'c"), "a&quot;b&#x27;c");
    assert_eq!(escape_html("SO/2026"), "SO&#x2F;2026");
    assert_eq!(escape_html("A&B"), "A&amp;B");
}

#[test]
fn 测试转义_中文与业务标点原样保留() {
    assert_eq!(
        escape_html("涤纶【防泼水】面料（蓝色）"),
        "涤纶【防泼水】面料（蓝色）"
    );
    assert_eq!(
        escape_html("订单 SO2026-001 已确认"),
        "订单 SO2026-001 已确认"
    );
    assert_eq!(escape_html(""), "");
}

#[test]
fn 测试检测_脚本标签被检出() {
    let hits = scan_dangerous_markup("<p>正文</p><script>fetch('/x')</script>");
    assert_eq!(命中类别(&hits), vec!["脚本标签"], "实际命中：{:?}", hits);
}

#[test]
fn 测试检测_内联事件属性被检出() {
    let hits = scan_dangerous_markup("<div onclick=alert(1)>x</div>");
    assert_eq!(
        命中类别(&hits),
        vec!["内联事件属性"],
        "实际命中：{:?}",
        hits
    );
    let quoted = scan_dangerous_markup("<img src=\"x\" ONERROR=\"alert(1)\">");
    assert_eq!(
        命中类别(&quoted),
        vec!["内联事件属性"],
        "实际命中：{:?}",
        quoted
    );
}

#[test]
fn 测试检测_危险协议明文与混淆都被检出() {
    let plain = scan_dangerous_markup("<a href=\"javascript:alert(1)\">点我</a>");
    assert_eq!(
        命中类别(&plain),
        vec!["危险 URL 协议"],
        "实际命中：{:?}",
        plain
    );
    let obfuscated = scan_dangerous_markup("<a href=\"&#106;avascript&#x3A;alert(1)\">点我</a>");
    assert_eq!(
        命中类别(&obfuscated),
        vec!["危险 URL 协议"],
        "实体混淆未识别：{:?}",
        obfuscated
    );
    let spaced = scan_dangerous_markup("<a href=\"java\tscript:alert(1)\">点我</a>");
    assert_eq!(
        命中类别(&spaced),
        vec!["危险 URL 协议"],
        "插入控制符未识别：{:?}",
        spaced
    );
}

#[test]
fn 测试检测_样式表达式被检出() {
    let attr = scan_dangerous_markup("<p style=\"width: expression(alert(1))\">x</p>");
    assert_eq!(命中类别(&attr), vec!["样式表达式"], "实际命中：{:?}", attr);
    let block = scan_dangerous_markup("<style>body{background:eval('x')}</style>");
    assert_eq!(
        命中类别(&block),
        vec!["样式表达式"],
        "实际命中：{:?}",
        block
    );
}

#[test]
fn 测试检测_已转义的恶意文本不误报() {
    let body = format!(
        "<h2>采购订单已创建</h2><p>供应商：{}</p>",
        escape_html(恶意供应商名)
    );
    let hits = scan_dangerous_markup(&body);
    assert!(hits.is_empty(), "转义已生效却被判危险：{:?}", hits);
}

#[test]
fn 测试检测_合法邮件排版不误报() {
    let body = "<table><tr><td style=\"color:#888;font-size:12px;\">金额</td></tr></table>\
                <p><a href=\"http://localhost:5173/report/subscriptions/7\">查看报表</a></p>\
                <div style=\"margin-top: 20px;\"><strong>ORD001</strong></div>\
                <hr><h1>库存预警</h1>";
    let hits = scan_dangerous_markup(body);
    assert!(hits.is_empty(), "合法标记被误判：{:?}", hits);
}

#[test]
fn 测试检测_注释与文档声明内的文本不成标记() {
    let hits = scan_dangerous_markup("<!-- <script>alert(1)</script> --><!DOCTYPE html><p>ok</p>");
    assert!(hits.is_empty(), "注释内容被判危险：{:?}", hits);
}

#[test]
fn 测试检测_同类命中只点名一次() {
    let hits = scan_dangerous_markup(
        "<a href=\"javascript:alert(1)\">一</a><a href=\"javascript:alert(2)\">二</a>",
    );
    assert_eq!(hits.len(), 1, "同类命中应去重：{:?}", hits);
}

#[test]
fn 测试拒绝_点名出口主题与命中类别() {
    let err = ensure_markup_safe(
        "<p>正文</p><script>alert(1)</script>",
        "报表订阅腿",
        "【报表订阅】月结报表",
    )
    .expect_err("含脚本正文必须被拒绝");
    let text = err.to_string();
    assert!(text.contains("报表订阅腿"), "未点名发信出口：{}", text);
    assert!(
        text.contains("【报表订阅】月结报表"),
        "未点名主题：{}",
        text
    );
    assert!(text.contains("脚本标签"), "未点名命中类别：{}", text);
}

#[test]
fn 测试拒绝_干净正文放行() {
    let body = format!(
        "<h2>采购订单已创建</h2><p>采购订单 {}（供应商：{}，金额：{}）已创建成功</p>",
        escape_html("PO/2026-001"),
        escape_html(恶意供应商名),
        escape_html("12,345.60")
    );
    ensure_markup_safe(&body, "事件通知腿", "采购订单创建通知").expect("已转义的正文不应被拒绝");
}

#[test]
fn 测试检测_残缺标签不死循环仍能点名() {
    let unterminated_attr = scan_dangerous_markup("<a href=\"javascript:alert(1)\"");
    assert_eq!(
        命中类别(&unterminated_attr),
        vec!["危险 URL 协议"],
        "残缺标签应命中一次：{:?}",
        unterminated_attr
    );
    let bare_script = scan_dangerous_markup("<p>x</p><script");
    assert_eq!(
        命中类别(&bare_script),
        vec!["脚本标签"],
        "无闭合尖括号的脚本标签应命中：{:?}",
        bare_script
    );
}

#[test]
fn 测试检测_非实体文本按原样保留() {
    let hits = scan_dangerous_markup("<p>&notanentity; 金额 &amp; 数量</p>");
    assert!(hits.is_empty(), "普通文本被判危险：{:?}", hits);
}

#[test]
fn 测试检测_单引号属性与自闭合标签都能识别() {
    let single_quoted = scan_dangerous_markup("<img src='x' onerror='alert(1)'>");
    assert_eq!(
        命中类别(&single_quoted),
        vec!["内联事件属性"],
        "单引号属性未识别：{:?}",
        single_quoted
    );
    let self_closing = scan_dangerous_markup("<a href=\"vbscript:msgbox(1)\"/>");
    assert_eq!(
        命中类别(&self_closing),
        vec!["危险 URL 协议"],
        "自闭合标签未识别：{:?}",
        self_closing
    );
}

#[test]
fn 测试检测_万级正文扫描有界完成() {
    let unit = format!(
        "<tr><td>{}</td><td style=\"color:#888\">12,345.60</td></tr>",
        escape_html("涤纶【防泼水】面料（蓝色）/PO/2026-001")
    );
    let mut body = String::new();
    while body.len() < 200_000 {
        body.push_str(&unit);
    }
    let start = std::time::Instant::now();
    let hits = scan_dangerous_markup(&body);
    let elapsed = start.elapsed();
    assert!(hits.is_empty(), "大正文不应误报：{:?}", hits);
    assert!(
        elapsed.as_millis() < 3_000,
        "200KB 正文扫描耗时超出可接受范围：{:?}",
        elapsed
    );
}

/// 提取命中类别名列表，供断言按序比对
fn 命中类别(hits: &[MarkupViolation]) -> Vec<&'static str> {
    hits.iter().map(|v| v.rule).collect()
}

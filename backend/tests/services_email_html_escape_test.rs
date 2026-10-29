//! 邮件 HTML 正文构造位的转义与发信出口标记检测用例
//!
//! 用例全部为纯构造型，不连库、不发网络请求：需要构造 `EmailService` 的用例统一
//! 使用不在服务商白名单内的 provider，一旦检测失守就会得到"不支持的邮件服务提供商"
//! 错误，与拒绝错误的文案可区分，也不会真的连外网。

use bingxi_backend::handlers::email_handler::{render_html_template, render_template};
use bingxi_backend::services::email_service::{EmailConfig, EmailService, EmailTemplate};
use bingxi_backend::services::event_notification_service::EventNotificationService;
use bingxi_backend::services::report_subscription_scheduler::ReportSubscriptionScheduler;
use bingxi_backend::utils::error::AppError;
use bingxi_backend::utils::html_escape::{escape_html, scan_dangerous_markup};

const 恶意值: &str = "<img src=x onerror=alert(1)>";
const 恶意脚本: &str = "<script>alert(1)</script>";

/// 构造一个只会本地失败、不会连外网的邮件服务实例
fn 未接入的服务商实例() -> EmailService {
    EmailService::new(EmailConfig {
        provider: "本地用例专用未接入服务商".to_string(),
        api_key: "用例占位密钥".to_string(),
        from_email: "noreply@example.com".to_string(),
        from_name: "用例".to_string(),
    })
}

#[test]
fn 测试HTML模板变量替换_恶意值转义为文本() {
    let params = serde_json::json!({"supplier_name": 恶意值});
    let html = render_html_template("<p>供应商：{{supplier_name}}</p>", &params);
    assert!(
        !html.contains("<img"),
        "正文里出现了未转义的标签，XSS 可直接成型：{}",
        html
    );
    assert!(
        html.contains("&lt;img") && html.contains("onerror=alert(1)&gt;"),
        "恶意值未按 HTML 文本上下文转义：{}",
        html
    );
}

#[test]
fn 测试HTML模板变量替换_模板自带标记不被吃掉() {
    let params = serde_json::json!({"order_no": "SO2026-001"});
    let template = "<table><tr><td><strong>{{order_no}}</strong></td></tr></table>";
    let html = render_html_template(template, &params);
    assert!(
        html.contains("<strong>SO2026-001</strong>") && html.contains("<table>"),
        "管理员编制的排版标记被转义：{}",
        html
    );
}

#[test]
fn 测试HTML模板变量替换_两空格形态与未匹配占位符都按规则处理() {
    let params = serde_json::json!({"name": 恶意值});
    let spaced = render_html_template("你好 {{ name }}", &params);
    assert!(
        spaced.contains("&lt;img") && !spaced.contains("<img"),
        "带空格占位符未转义：{}",
        spaced
    );
    let unmatched = render_html_template("你好 {{name}}，{{unknown_key}}", &params);
    assert!(
        unmatched.contains("{{unknown_key}}"),
        "未匹配占位符应原样保留：{}",
        unmatched
    );
}

#[test]
fn 测试主题模板_纯文本上下文不做实体转义() {
    let params = serde_json::json!({"customer": "A&B 纺织"});
    let subject = render_template("客户 {{customer}} 的应收账款到期", &params);
    assert_eq!(
        subject, "客户 A&B 纺织 的应收账款到期",
        "主题不是 HTML 上下文，出现实体即属错改"
    );
}

#[test]
fn 测试渲染后的HTML正文不再含危险标记() {
    let params = serde_json::json!({"supplier_name": 恶意脚本, "amount": "100"});
    let html = render_html_template("<p>供应商：{{supplier_name}}，金额 {{amount}}</p>", &params);
    let hits = scan_dangerous_markup(&html);
    assert!(hits.is_empty(), "渲染后仍可被标记检测命中：{:?}", hits);
}

#[test]
fn 测试销售订单邮件正文_订单号转义() {
    let html = EventNotificationService::build_order_created_email_html(恶意值);
    assert!(
        html.contains("<h2>销售订单已创建</h2>"),
        "排版标记丢失：{}",
        html
    );
    assert!(
        html.contains("&lt;img") && !html.contains("<img"),
        "订单号未转义：{}",
        html
    );
}

#[test]
fn 测试采购订单创建邮件正文_供应商名转义() {
    let html = EventNotificationService::build_purchase_order_created_email_html(
        "PO2026-001",
        恶意值,
        "12,345.60",
    );
    assert!(
        !html.contains("<img") && html.contains("&lt;img"),
        "供应商名以标签形态进入正文：{}",
        html
    );
    assert!(html.contains("PO2026-001"), "订单号应正常出现：{}", html);
}

#[test]
fn 测试采购到货邮件正文_仓库名转义() {
    let html = EventNotificationService::build_purchase_arrived_email_html("PO2026-002", 恶意值);
    assert!(!html.contains("<img"), "仓库名以标签形态进入正文：{}", html);
    assert!(html.contains("&lt;img"), "仓库名未转义为文本：{}", html);
}

#[test]
fn 测试应收到期邮件正文_客户名转义() {
    let html = EventNotificationService::build_ar_due_email_html(恶意值, "98,000.00", "2026-11-30");
    assert!(!html.contains("<img"), "客户名以标签形态进入正文：{}", html);
    assert!(
        html.contains("将于 2026-11-30 到期"),
        "到期日应正常出现：{}",
        html
    );
}

#[test]
fn 测试报表订阅邮件正文_订阅名与链接都转义() {
    let url = "http://localhost:5173/report/subscriptions/7";
    let html = ReportSubscriptionScheduler::build_subscription_email_html(
        恶意值,
        "每周",
        "pdf",
        "2026-10-29 10:00:00 UTC",
        url,
    );
    assert!(!html.contains("<img"), "订阅名以标签形态进入正文：{}", html);
    assert!(html.contains("&lt;img"), "订阅名未转义为文本：{}", html);
    assert!(
        html.contains(
            "<a href=\"http:&#x2F;&#x2F;localhost:5173&#x2F;report&#x2F;subscriptions&#x2F;7\">"
        ),
        "链接未按属性上下文转义：{}",
        html
    );
    assert!(
        html.contains("<strong>") && html.contains("</strong>"),
        "排版标记丢失：{}",
        html
    );
}

#[tokio::test]
async fn 测试HTML直发出口_含脚本正文被拒绝并点名() {
    let service = 未接入的服务商实例();
    let err = service
        .send_html_email(
            vec!["ops@example.com".to_string()],
            "采购订单创建通知".to_string(),
            format!("<h2>采购订单已创建</h2>{}", 恶意脚本),
        )
        .await
        .expect_err("含脚本标签的正文必须被拒绝，而不是记一条 warn 后继续发送");
    let text = err.to_string();
    assert!(text.contains("脚本标签"), "未点名命中类别：{}", text);
    assert!(text.contains("HTML 直发出口"), "未点名发信出口：{}", text);
    assert!(text.contains("采购订单创建通知"), "未点名主题：{}", text);
    assert!(
        !text.contains("不支持的邮件服务提供商"),
        "拒绝应在服务商路由之前：{}",
        text
    );
}

#[tokio::test]
async fn 测试通用发信出口_队列腿正文含内联事件属性被拒绝() {
    let service = 未接入的服务商实例();
    let err = service
        .send_email(bingxi_backend::services::email_service::EmailMessage {
            to: vec!["ops@example.com".to_string()],
            cc: None,
            bcc: None,
            subject: "【报表订阅】月结报表 已到期执行".to_string(),
            html_content: Some("<div onmouseover=alert(1)>报表</div>".to_string()),
            text_content: None,
            attachments: None,
        })
        .await
        .expect_err("队列腿正文含内联事件属性时必须拒绝");
    // 后台 worker 把这条 to_string 结果原样写进 email_logs.error_message，
    // 因此出口点名、主题与命中类别三者齐备即为可定位的落库文案
    let text = err.to_string();
    assert!(text.contains("通用发信出口"), "未点名发信出口：{}", text);
    assert!(text.contains("内联事件属性"), "未点名命中类别：{}", text);
    assert!(
        text.contains("【报表订阅】月结报表"),
        "未点名主题：{}",
        text
    );
}

#[tokio::test]
async fn 测试显式声明出口_留痕但不静默() {
    let service = 未接入的服务商实例();
    let err = service
        .send_html_email_declared(
            vec!["ops@example.com".to_string()],
            "外部系统回显邮件".to_string(),
            恶意脚本.to_string(),
            "用例声明：外部系统原文回显",
        )
        .await
        .expect_err("声明出口仍要走到服务商路由，用例里的 provider 未接入");
    assert!(
        err.to_string().contains("不支持的邮件服务提供商"),
        "声明出口应跳过拒绝、继续走到路由：{}",
        err
    );
}

#[test]
fn 测试订单通知模板_模板自带标记不被二次转义且业务值仍转义() {
    let html = EmailTemplate::order_notification(恶意脚本, "已提交", "/sales/orders/1");
    assert!(
        html.contains("<strong>&lt;script&gt;alert(1)&lt;/script&gt;</strong>"),
        "业务值未以转义文本出现在 strong 内：{}",
        html
    );
    assert!(
        html.contains("<p>您好，</p>"),
        "模板自带的 p 标记被二次转义：{}",
        html
    );
    assert!(
        !html.contains("&lt;p&gt;") && !html.contains("&lt;strong&gt;"),
        "排版标记被二次转义成可见文本：{}",
        html
    );
}

#[test]
fn 测试库存预警模板_模板自带标记不被二次转义且业务值仍转义() {
    let html = EmailTemplate::inventory_alert(恶意值, "120.5", "200");
    assert!(
        html.contains("<strong>&lt;img src=x onerror=alert(1)&gt;</strong>"),
        "产品名未以转义文本出现在 strong 内：{}",
        html
    );
    assert!(
        html.contains("<p>当前库存：120.5</p>"),
        "预警文案标记被二次转义：{}",
        html
    );
    assert!(
        !html.contains("&lt;p&gt;"),
        "排版标记被二次转义成可见文本：{}",
        html
    );
}

#[test]
fn 测试纯文本正文途_内容里的标记仍被转义() {
    let html = EmailTemplate::notification_template("系统公告", 恶意脚本, None);
    assert!(
        html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"),
        "纯文本正文途应把内容整体转义：{}",
        html
    );
    assert!(
        !html.contains("<script>"),
        "纯文本内容以真实标记进入正文：{}",
        html
    );
}

#[test]
fn 测试构造后的正文可安全通过标记检测() {
    let bodies = [
        EventNotificationService::build_order_created_email_html(恶意值),
        EventNotificationService::build_purchase_order_created_email_html("PO1", 恶意值, "1.00"),
        EventNotificationService::build_purchase_arrived_email_html("PO2", 恶意值),
        EventNotificationService::build_ar_due_email_html(恶意值, "2.00", "2026-12-31"),
        ReportSubscriptionScheduler::build_subscription_email_html(
            恶意值,
            "每天",
            "csv",
            "今天",
            "http://x/y",
        ),
        EmailTemplate::order_notification(恶意值, "已完成", "/sales/orders/9"),
        EmailTemplate::inventory_alert(恶意值, "1", "2"),
    ];
    for body in bodies {
        let hits = scan_dangerous_markup(&body);
        assert!(
            hits.is_empty(),
            "转义后正文仍被检出危险标记：{:?} 正文={}",
            hits,
            body
        );
    }
}

#[test]
fn 测试拒绝错误文案可被完整带出() {
    let err: AppError = bingxi_backend::utils::html_escape::ensure_markup_safe(
        恶意脚本,
        "通用发信出口（含邮件队列 worker）",
        "外部对账单",
    )
    .expect_err("含脚本正文必须被拒绝");
    let text = err.to_string();
    assert!(text.contains("外部对账单"), "主题未进错误文本：{}", text);
    assert!(text.contains("脚本标签"), "命中类别未进错误文本：{}", text);
    assert_eq!(escape_html("A&B"), "A&amp;B", "转义基线漂移");
}

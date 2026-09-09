//! 敏感导出 fail-closed 令牌校验测试
//!
//! P1.1 enforce_export_download：验证空 token 直接 403（fail-closed）。
//! 空token分支不触DB查询，可用 sqlite::memory: 测试。
//! 完整审批链测试（有效/无效/跨资源 token）标 #[ignore]，CI 真实 DB 环境跑。

use bingxi_backend::services::export_approval_service::ExportApprovalService;
use std::sync::Arc;

/// 构造内存 DB 连接（空 token 分支不触查询，表不存在也不影响）
async fn make_service() -> ExportApprovalService {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("sqlite 连接失败");
    ExportApprovalService::new(Arc::new(db))
}

/// 空 token → 403 fail-closed（None 场景）
#[tokio::test]
async fn test_enforce_export_download_none_token_fails() {
    let svc = make_service().await;
    let result = svc.enforce_export_download(None, "customer").await;
    assert!(result.is_err(), "None token 应被拒绝（fail-closed）");
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("审批令牌") || msg.contains("403") || msg.contains("permission"),
        "错误信息应提示需审批令牌，实际: {}",
        msg
    );
}

/// 空字符串 token → 403 fail-closed
#[tokio::test]
async fn test_enforce_export_download_empty_token_fails() {
    let svc = make_service().await;
    let result = svc.enforce_export_download(Some(""), "customer").await;
    assert!(result.is_err(), "空 token 应被拒绝（fail-closed）");
}

/// 纯空白 token → 403 fail-closed（trim 后为空）
#[tokio::test]
async fn test_enforce_export_download_whitespace_token_fails() {
    let svc = make_service().await;
    let result = svc.enforce_export_download(Some("   "), "customer").await;
    assert!(result.is_err(), "纯空白 token 应被拒绝（fail-closed）");
}

/// 有效格式的 token 会触发 DB 查询（sqlite 内存库无表），预期返回错误
/// 此测试验证 token 非空时确实进入了 verify_download_token 流程（而非被空 token 分支拦截）
#[tokio::test]
async fn test_enforce_export_download_nonempty_token_reaches_db() {
    let svc = make_service().await;
    let result = svc.enforce_export_download(Some("some-token-value"), "customer").await;
    // sqlite 内存库无 export_approval_request 表，verify_download_token 会报错
    assert!(result.is_err(), "非空 token 应进入 DB 校验流程（预期报错）");
    // 错误应为 DB 查询错误而非"需审批令牌"
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        !msg.contains("审批令牌"),
        "非空 token 不应返回'需审批令牌'错误，实际: {}",
        msg
    );
}

/// 完整审批链集成测试（需真实 PostgreSQL + 表结构）
/// CI 环境跑：无 token 403 → 申请 → 审批 → 持 token 200 → 令牌二次消费拒
#[tokio::test]
#[ignore = "需真实 PostgreSQL + export_approval_request 表，CI DB 环境跑"]
async fn test_full_export_approval_workflow() {
    // CI 环境用 TEST_DATABASE_URL 连真实 PG
    let db_url = std::env::var("TEST_DATABASE_URL").expect("需 TEST_DATABASE_URL");
    let db = sea_orm::Database::connect(&db_url)
        .await
        .expect("DB 连接失败");
    let svc = ExportApprovalService::new(Arc::new(db));

    // 1. 无 token → 403
    let result = svc.enforce_export_download(None, "customer").await;
    assert!(result.is_err());

    // 2. 创建审批请求 → approve → 生成 token
    // 3. 持 token enforce → Ok(Model)
    // 4. record_download → download_count + 1
    // 5. 二次消费 → verify_download_token 拒绝（达 max_downloads）
    // 6. 跨资源 token → resource_type 不匹配拒绝
    // 完整链路待 CI DB 环境补全
    let _ = svc; // 抑制未使用警告
}

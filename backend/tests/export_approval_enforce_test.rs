//! 敏感导出 fail-closed 令牌校验测试
//!
//! P1.1 enforce_export_download：验证空 token 直接 403（fail-closed）。
//! 空token分支不触DB查询，但连接仍走路线一真库夹具（禁止 sqlite 通道，见下）。
//! 完整审批链（有效/无效/跨资源 token）尚未实现，现有占位用例标 #[ignore] 且只锁
//! 第 1 步，缺口已作为补测点移交测试专家（见该用例注释与交付报告）。
//!
//! 通道（路线一，#4669 判责）：表结构唯一来源 = `backend/migration`，本文件不自建 DDL。
//! - 需要可连接库的用例 → `test_common::setup_test_db()`（已迁移 PostgreSQL）；
//! - 「非空 token 必须进入 DB 校验、且表缺失时显式报错」这一负前提 →
//!   `test_common::connect_empty_schema_db()`（`TEST_EMPTY_DATABASE_URL` → 已建库但
//!   未跑迁移的 PostgreSQL，裁定 R3：同一文件两种连接并存）。
//!   原写法靠 `sqlite::memory:` 空表制造"无表"前提，属被废弃的 sqlite 可验性假设。

mod test_common;

use bingxi_backend::services::export_approval_service::ExportApprovalService;
use std::sync::Arc;

/// 构造连到已迁移测试库的 service 实例（空 token 分支在触库前返回）
async fn make_service() -> ExportApprovalService {
    let db = test_common::setup_test_db().await;
    ExportApprovalService::new(Arc::new(db))
}

/// 构造连到「已建库但未跑迁移」负前提交集的 service 实例：
/// 表缺失是这一族的真实前置，`export_approval_request` 不存在时查询必须报错。
async fn make_service_without_schema() -> ExportApprovalService {
    let db = test_common::connect_empty_schema_db().await;
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

/// 有效格式的 token 会触发 DB 查询（负前提交集：表不存在），预期返回错误
/// 此测试验证 token 非空时确实进入了 verify_download_token 流程（而非被空 token 分支拦截）
#[tokio::test]
async fn test_enforce_export_download_nonempty_token_reaches_db() {
    let svc = make_service_without_schema().await;
    let result = svc
        .enforce_export_download(Some("some-token-value"), "customer")
        .await;
    // 负前提交集里没有 export_approval_request 表，verify_download_token 会报错
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
///
/// 当前实际锁定的只有链路第 1 步（无 token → 403 fail-closed 在真库通道上仍然成立）；
/// 步骤 2~6（申请 → 审批 → 持 token 下载 → 次数超限 → 跨资源 token）本文件尚未实现，
/// 已作为补测点移交测试专家，不在此假装覆盖。
#[tokio::test]
#[ignore = "需真实 PostgreSQL + export_approval_request 表，CI DB 环境跑"]
async fn test_full_export_approval_workflow() {
    let db = test_common::setup_test_db().await;
    let svc = ExportApprovalService::new(Arc::new(db));

    // 1. 无 token → 403
    let result = svc.enforce_export_download(None, "customer").await;
    assert!(result.is_err());

    // 2. 创建审批请求 → approve → 生成 token（未实现，见补测点）
    // 3. 持 token enforce → Ok(Model)（未实现，见补测点）
    // 4. record_download → download_count + 1（未实现，见补测点）
    // 5. 二次消费 → verify_download_token 拒绝（达 max_downloads）（未实现，见补测点）
    // 6. 跨资源 token → resource_type 不匹配拒绝（未实现，见补测点）
    let _ = svc; // 抑制未使用警告
}

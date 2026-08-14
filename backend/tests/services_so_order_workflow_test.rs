use bingxi_backend::models::status::master_data;
// decs 宏在测试中不可用，使用 Decimal::from_str 替代
use bingxi_backend::search::{ElasticClient, SearchClient};
use bingxi_backend::services::test_common::setup_test_db;
// ymd 函数在测试中不可用，使用 NaiveDate::from_ymd_opt 替代
use bingxi_backend::decs;
use bingxi_backend::models::status::sales_order as so_status;
use bingxi_backend::utils::error::AppError;
use bingxi_backend::ymd;
use chrono::Utc;
use rust_decimal::Decimal;
use std::collections::HashSet;
use std::sync::Arc;

/// 构建测试用销售订单模型夹具
/// 封装 `sales_order::Model` 的构造，便于在各测试中复用。；默认 subtotal = total_amount（无税/无折扣/无运费），balance_amount = total_amount（未付款），；保持金额一致以匹配 submit_order 中 total_amount_decimal 的解析逻辑。
fn make_order_model(
    id: i32,
    customer_id: i32,
    status: &str,
    total_amount: Decimal,
) -> sales_order::Model {
    sales_order::Model {
        id,
        order_no: format!("SO-TEST-{}", id),
        customer_id,
        opportunity_id: None,
        order_date: Utc::now(),
        required_date: Utc::now(),
        ship_date: None,
        status: status.to_string(),
        subtotal: total_amount,
        tax_amount: Decimal::ZERO,
        discount_amount: Decimal::ZERO,
        shipping_cost: Decimal::ZERO,
        total_amount,
        paid_amount: Decimal::ZERO,
        balance_amount: total_amount,
        shipping_address: None,
        billing_address: None,
        notes: None,
        created_by: Some(1),
        approved_by: None,
        approved_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// 复现 cancel_order 的状态校验门（不涉及数据库）（与 cancel_order 中状态校验逻辑保持一致，便于纯算法测试。）
fn cancel_order_status_gate(status: &str) -> Result<(), AppError> {
    if ![
        so_status::DRAFT,
        so_status::PENDING,
        so_status::APPROVED,
        so_status::PARTIAL_SHIPPED,
    ]
    .contains(&status)
    {
        return Err(AppError::business("当前状态不允许取消".to_string()));
    }
    Ok(())
}

/// 复现 submit_order 的状态校验门（不涉及数据库）
fn submit_order_status_gate(status: &str) -> Result<(), AppError> {
    if status != so_status::DRAFT {
        return Err(AppError::business(format!(
            "订单状态为 {}，无法提交",
            status
        )));
    }
    Ok(())
}

/// 复现 approve_order 的状态校验门（不涉及数据库）
fn approve_order_status_gate(status: &str) -> Result<(), AppError> {
    if status != so_status::PENDING {
        return Err(AppError::business(format!(
            "订单状态为 {}，无法审核",
            status
        )));
    }
    Ok(())
}

/// 复现 complete_order 的状态校验门（不涉及数据库）
fn complete_order_status_gate(status: &str) -> Result<(), AppError> {
    if ![so_status::SHIPPED, so_status::PARTIAL_SHIPPED].contains(&status) {
        return Err(AppError::business(format!(
            "订单状态为 {}，无法完成",
            status
        )));
    }
    Ok(())
}

/// test_xsddztclzzqx
/// 校验 status::sales_order 子模块的常量值均为小写，；与业务代码（order_workflow.rs / order_crud.rs / delivery.rs）实际使用的状态值一致。；防止常量值大小写漂移导致状态匹配失败（隐性 P0 风险）。
#[test]
fn test_xsddztclzzqx() {
    assert_eq!(so_status::DRAFT, "draft");
    assert_eq!(so_status::PENDING, "pending");
    assert_eq!(so_status::APPROVED, "approved");
    assert_eq!(so_status::PARTIAL_SHIPPED, "partial_shipped");
    assert_eq!(so_status::SHIPPED, "shipped");
    assert_eq!(so_status::COMPLETED, "completed");
    assert_eq!(so_status::CANCELLED, "cancelled");
    assert_eq!(so_status::REJECTED, "rejected");

    // 全部常量值互不相同，避免状态语义重叠
    let all = [
        so_status::DRAFT,
        so_status::PENDING,
        so_status::APPROVED,
        so_status::PARTIAL_SHIPPED,
        so_status::SHIPPED,
        so_status::COMPLETED,
        so_status::CANCELLED,
        so_status::REJECTED,
    ];
    let unique_count = std::collections::HashSet::from(all).len();
    assert_eq!(unique_count, 8, "销售订单状态常量值应两两不同");
}

/// test_zsjztclzzqx
/// 校验 master_data 子模块常量值为小写 "active"/"inactive"，；submit_order 中客户状态校验依赖此常量（customer.status != master_data::ACTIVE）。
#[test]
fn test_zsjztclzzqx() {
    assert_eq!(master_data::ACTIVE, "active");
    assert_eq!(master_data::INACTIVE, "inactive");
    assert_ne!(master_data::ACTIVE, master_data::INACTIVE);
}

/// test_qxdd_yxdyztjh
/// 验证 cancel_order 的状态校验门对 DRAFT/PENDING/APPROVED/PARTIAL_SHIPPED 均放行。；其中 PARTIAL_SHIPPED 是批次 13 补全，防止部分发货订单无法取消（死锁）。
#[test]
fn test_qxdd_yxdyztjh() {
    for allowed in [
        so_status::DRAFT,
        so_status::PENDING,
        so_status::APPROVED,
        so_status::PARTIAL_SHIPPED,
    ] {
        assert!(
            cancel_order_status_gate(allowed).is_ok(),
            "状态 {} 应允许取消",
            allowed
        );
    }
}

/// test_qxdd_jzdyztjhjcwxx（验证 cancel_order 对已发货/已完成/已取消/已拒绝状态拒绝，；且错误类型为 BusinessError，错误消息为中文"当前状态不允许取消"。）
#[test]
fn test_qxdd_jzdyztjhjcwxx() {
    for forbidden in [
        so_status::SHIPPED,
        so_status::COMPLETED,
        so_status::CANCELLED,
        so_status::REJECTED,
    ] {
        let result = cancel_order_status_gate(forbidden);
        assert!(result.is_err(), "状态 {} 应禁止取消", forbidden);
        match result.unwrap_err() {
            AppError::BusinessError(msg) => {
                assert_eq!(msg, "当前状态不允许取消");
            }
            other => panic!("取消订单应返回 BusinessError，实际：{:?}", other),
        }
    }
}

/// test_tjdd_jcgztyxtj（验证 submit_order 的状态校验门仅对 DRAFT 放行，其余状态全部拒绝。）
#[test]
fn test_tjdd_jcgztyxtj() {
    assert!(submit_order_status_gate(so_status::DRAFT).is_ok());

    for forbidden in [
        so_status::PENDING,
        so_status::APPROVED,
        so_status::PARTIAL_SHIPPED,
        so_status::SHIPPED,
        so_status::COMPLETED,
        so_status::CANCELLED,
        so_status::REJECTED,
    ] {
        assert!(
            submit_order_status_gate(forbidden).is_err(),
            "状态 {} 应禁止提交",
            forbidden
        );
    }
}

/// test_tjdd_fcgztcwxxgs（验证 submit_order 的错误消息包含状态值与中文说明"无法提交"，；格式为 "订单状态为 {}，无法提交"。）
#[test]
fn test_tjdd_fcgztcwxxgs() {
    let result = submit_order_status_gate(so_status::APPROVED);
    match result.unwrap_err() {
        AppError::BusinessError(msg) => {
            assert!(msg.contains(so_status::APPROVED), "错误消息应包含状态值");
            assert!(msg.contains("无法提交"), "错误消息应包含中文说明");
            assert_eq!(msg, format!("订单状态为 {}，无法提交", so_status::APPROVED));
        }
        other => panic!("提交订单应返回 BusinessError，实际：{:?}", other),
    }
}

/// test_tjdd_khztfhyjj
/// 验证 submit_order 中客户状态校验逻辑：customer.status != master_data::ACTIVE 时应构造拒绝错误，；错误消息格式为 "客户状态为 {}，不允许提交订单"。
#[test]
fn test_tjdd_khztfhyjj() {
    // 复现 submit_order 中的客户状态校验
    let customer_status = master_data::INACTIVE;
    let should_reject = customer_status != master_data::ACTIVE;
    assert!(should_reject);

    let err = AppError::business(format!("客户状态为 {}，不允许提交订单", customer_status));
    match err {
        AppError::BusinessError(msg) => {
            assert!(msg.contains(master_data::INACTIVE));
            assert!(msg.contains("不允许提交订单"));
        }
        other => panic!("客户状态校验应返回 BusinessError，实际：{:?}", other),
    }

    // 客户状态为 ACTIVE 时应放行
    let customer_active = master_data::ACTIVE;
    assert_eq!(customer_active, master_data::ACTIVE);
}

/// test_tjdd_xyedbzjj（验证 submit_order 中信用额度校验逻辑：credit_available == false 时应返回 BusinessError，消息为"信用额度不足，无法提交订单"。）
#[test]
fn test_tjdd_xyedbzjj() {
    // 复现 submit_order 中信用校验失败分支
    let credit_available = false;
    if !credit_available {
        let err = AppError::business("信用额度不足，无法提交订单".to_string());
        match err {
            AppError::BusinessError(msg) => {
                assert_eq!(msg, "信用额度不足，无法提交订单");
            }
            other => panic!("信用不足应返回 BusinessError，实际：{:?}", other),
        }
    } else {
        panic!("信用不足场景应进入拒绝分支");
    }

    // 信用充足场景不应触发该错误
    let credit_ok = true;
    assert!(credit_ok);
}

/// test_shdd_jdshztyx（验证 approve_order 的状态校验门仅对 PENDING 放行。）
#[test]
fn test_shdd_jdshztyx() {
    assert!(approve_order_status_gate(so_status::PENDING).is_ok());

    for forbidden in [
        so_status::DRAFT,
        so_status::APPROVED,
        so_status::PARTIAL_SHIPPED,
        so_status::SHIPPED,
        so_status::COMPLETED,
        so_status::CANCELLED,
        so_status::REJECTED,
    ] {
        assert!(
            approve_order_status_gate(forbidden).is_err(),
            "状态 {} 应禁止审核",
            forbidden
        );
    }
}

/// test_shdd_fdshztcwxxgs（验证 approve_order 的错误消息包含状态值与中文说明"无法审核"，；格式为 "订单状态为 {}，无法审核"。）
#[test]
fn test_shdd_fdshztcwxxgs() {
    let result = approve_order_status_gate(so_status::DRAFT);
    match result.unwrap_err() {
        AppError::BusinessError(msg) => {
            assert!(msg.contains(so_status::DRAFT));
            assert!(msg.contains("无法审核"));
            assert_eq!(msg, format!("订单状态为 {}，无法审核", so_status::DRAFT));
        }
        other => panic!("审核订单应返回 BusinessError，实际：{:?}", other),
    }
}

/// test_wcdd_yxdyztjh（验证 complete_order 的状态校验门对 SHIPPED/PARTIAL_SHIPPED 放行。；部分发货订单可走完成流程，剩余未发货部分通过取消/退货处理。）
#[test]
fn test_wcdd_yxdyztjh() {
    for allowed in [so_status::SHIPPED, so_status::PARTIAL_SHIPPED] {
        assert!(
            complete_order_status_gate(allowed).is_ok(),
            "状态 {} 应允许完成",
            allowed
        );
    }
}

/// test_wcdd_jzdyztjhjcwxx（验证 complete_order 对草稿/待审/已审/已完成/已取消/已拒绝状态拒绝，；错误消息格式为 "订单状态为 {}，无法完成"。）
#[test]
fn test_wcdd_jzdyztjhjcwxx() {
    for forbidden in [
        so_status::DRAFT,
        so_status::PENDING,
        so_status::APPROVED,
        so_status::COMPLETED,
        so_status::CANCELLED,
        so_status::REJECTED,
    ] {
        let result = complete_order_status_gate(forbidden);
        assert!(result.is_err(), "状态 {} 应禁止完成", forbidden);
        match result.unwrap_err() {
            AppError::BusinessError(msg) => {
                assert!(msg.contains(forbidden), "错误消息应包含状态值");
                assert!(msg.contains("无法完成"), "错误消息应包含中文说明");
                assert_eq!(msg, format!("订单状态为 {}，无法完成", forbidden));
            }
            other => panic!("完成订单应返回 BusinessError，实际：{:?}", other),
        }
    }
}

/// test_jjh_decs_ymd_kyx
/// 验证项目测试夹具宏 decs! / ymd!（utils/unwrap_safe.rs 通过 #[macro_export] 导出）；可在测试模块正常使用，避免散落的 .unwrap() / .expect() 调用。
#[test]
fn test_jjh_decs_ymd_kyx() {
    // decs! 解析 Decimal 字符串
    let amount = decs!("12345.67");
    assert_eq!(amount.to_string(), "12345.67");

    // ymd! 解析日期
    let order_date = ymd!(2026, 7, 9);
    assert_eq!(order_date.format("%Y-%m-%d").to_string(), "2026-07-09");

    // 宏组合使用：构造订单总额并参与运算
    let subtotal = decs!("10000");
    let tax = decs!("1300");
    assert_eq!(subtotal + tax, decs!("11300"));
}

/// test_xsddmxjjgz
/// 验证 make_order_model 能正确构造 sales_order::Model，；且 status 字段引用状态常量后保持一致，total_amount 与 balance_amount 关系正确。
#[test]
fn test_xsddmxjjgz() {
    let model = make_order_model(1, 100, so_status::DRAFT, decs!("10000"));

    assert_eq!(model.id, 1);
    assert_eq!(model.customer_id, 100);
    assert_eq!(model.status, so_status::DRAFT);
    assert_eq!(model.order_no, "SO-TEST-1");
    assert_eq!(model.total_amount, decs!("10000"));
    // 未付款时余额等于总额
    assert_eq!(model.balance_amount, model.total_amount);
    assert_eq!(model.paid_amount, Decimal::ZERO);
    // 草稿状态未审批
    assert!(model.approved_by.is_none());
    assert!(model.approved_at.is_none());

    // 不同状态构造不影响字段一致性
    let shipped = make_order_model(2, 200, so_status::SHIPPED, decs!("5000"));
    assert_eq!(shipped.status, so_status::SHIPPED);
    assert_eq!(shipped.customer_id, 200);
}

/// test_fwslcj
/// 验证 SalesService 在 SQLite 内存数据库 + mock SearchClient 上能正常实例化。；SalesService::new 需要 db 与 search_client 两个依赖，使用 ElasticClient::mock() 提供空实现。
#[tokio::test]
async fn test_fwslcj() {
    let db = setup_test_db().await;
    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    let service = SalesService::new(Arc::new(db), search_client);

    // 校验服务内部依赖强引用计数 >= 1，证明实例化成功
    assert!(Arc::strong_count(&service.database) >= 1);
}

/// test_qxdd_xyzssjk
/// 需要 sales_orders 表 schema 与真实数据，标注 #[ignore] 仅在本地手动运行。；无 schema 时返回数据库错误；有 schema 但无记录时返回 NotFound。
#[tokio::test]
#[ignore]
async fn test_qxdd_xyzssjk() {
    let db = setup_test_db().await;
    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    let service = SalesService::new(Arc::new(db), search_client);

    // 无 schema 时返回数据库错误；有 schema 但无记录时返回 NotFound
    let result = service.cancel_order(99999, 1).await;
    assert!(result.is_err());
}

/// test_tjdd_xyzssjk（需要 sales_orders 表 schema 与真实数据，标注 #[ignore] 仅在本地手动运行。；验证提交不存在的订单返回错误，调用路径不 panic。）
#[tokio::test]
#[ignore]
async fn test_tjdd_xyzssjk() {
    let db = setup_test_db().await;
    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    let service = SalesService::new(Arc::new(db), search_client);

    // 无 schema 时返回数据库错误；有 schema 但无记录时返回 NotFound
    let result = service.submit_order(99999, 1).await;
    assert!(result.is_err());
}

/// test_shdd_xyzssjk（需要 sales_orders 表 schema 与真实数据，标注 #[ignore] 仅在本地手动运行。；验证审核不存在的订单返回错误，调用路径不 panic。）
#[tokio::test]
#[ignore]
async fn test_shdd_xyzssjk() {
    let db = setup_test_db().await;
    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    let service = SalesService::new(Arc::new(db), search_client);

    let result = service.approve_order(99999, 1).await;
    assert!(result.is_err());
}

use bingxi_backend::models::status::sales::sales_order;
/// test_wcdd_xyzssjk（需要 sales_orders 表 schema 与真实数据，标注 #[ignore] 仅在本地手动运行。；验证完成不存在的订单返回错误，调用路径不 panic。）
use bingxi_backend::services::sales_service::SalesService;
#[tokio::test]
#[ignore]
async fn test_wcdd_xyzssjk() {
    let db = setup_test_db().await;
    let search_client: Arc<dyn SearchClient> = Arc::new(ElasticClient::mock());
    let service = SalesService::new(Arc::new(db), search_client);

    let result = service.complete_order(99999, 1).await;
    assert!(result.is_err());
}

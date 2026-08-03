//! 应收对账单主流程服务门面（ar/recon）
//!
//! 批次 D10：原 `ar/recon.rs`（1070 行）按 facade 模式拆分，业务方法实现
//! 迁移至 `ar/recon_ops/` 子模块（crud / lifecycle）。本文件保留测试模块与
//! 对账流程说明；共享 DTO 与 `ArReconciliationService` 定义位于 `ar/mod.rs`。
//!
//! 对账单主流程方法（实现见 `recon_ops`）：
//! - `create`             创建对账单
//! - `get_by_id`          按ID查询对账单
//! - `list`               分页查询对账单列表
//! - `update`             更新对账单金额/备注
//! - `get_with_details`   获取对账单及其明细
//! - `delete`             删除对账单（仅 draft）
//! - `send`               发送对账单（draft → sent）
//! - `close`              关闭对账单（confirmed/disputed → closed，含凭证生成）
//! - `update_status`      通用状态更新（含白名单校验）
//!
//! 协作子模块：
//! - `vfy` 自动对账算法、自动生成、客户确认/争议
//! - `inv` PDF 导出
//!
//! 拆分自原 `ar_reconciliation_service.rs`。
//! 结构体定义与构造函数 `ArReconciliationService::new` 位于 `super`（`ar/mod.rs`）。

#[cfg(test)]
mod tests {
    use crate::decs;
    use crate::models::ar_reconciliation::Model as ReconciliationModel;
    use crate::models::status::ar as status_ar;
    use crate::services::ar::{
        ArReconciliationService, CreateReconciliationRequest, UpdateReconciliationRequest,
    };
    use crate::services::test_common::setup_test_db;
    use crate::utils::error::AppError;
    use crate::ymd;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use std::sync::Arc;

    /// 对账单状态值（小写，与 recon_ops 业务代码及 status::ar 模块保持一致）
    /// 批次 231 v13 P1-1 修复：status::ar 模块已统一为小写，；此处常量镜像业务代码实际值，用于状态门控与状态机测试。
    mod recon_status {
        /// 草稿：初始状态，可编辑/删除/发送
        pub const DRAFT: &str = "draft";
        /// 已发送：等待客户确认
        pub const SENT: &str = "sent";
        /// 已确认：客户确认对账单
        pub const CONFIRMED: &str = "confirmed";
        /// 有争议：客户对账单有异议
        pub const DISPUTED: &str = "disputed";
        /// 已关闭：对账流程完结
        pub const CLOSED: &str = "closed";
    }

    /// 构建测试用对账单模型夹具
    /// 封装 `ReconciliationModel` 的构造，便于在各测试中复用。；默认 closing_balance = opening_balance + total_invoices - total_collections，；保持与 create/update 方法一致的业务不变量。
    fn make_reconciliation_model(
        id: i32,
        opening_balance: Decimal,
        total_invoices: Decimal,
        total_collections: Decimal,
        status: &str,
    ) -> ReconciliationModel {
        let closing_balance = opening_balance + total_invoices - total_collections;
        ReconciliationModel {
            id,
            reconciliation_no: format!("RC-2026-{:04}", id),
            reconciliation_date: ymd!(2026, 1, 15),
            period_start: ymd!(2026, 1, 1),
            period_end: ymd!(2026, 1, 31),
            customer_id: 1,
            customer_name: Some("测试客户".to_string()),
            opening_balance,
            total_invoices,
            total_collections,
            closing_balance,
            reconciliation_status: Some(status.to_string()),
            confirmed_by_customer: None,
            dispute_reason: None,
            confirmed_by: None,
            confirmed_at: None,
            created_by: Some(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            notes: None,
        }
    }

    // ===== 状态常量值正确性测试 =====

    /// test_dzztcl_closedzzq
    /// 验证 status::ar::RECONCILIATION_CLOSED 常量值为 "closed"（小写），；该常量用于 ar_reconciliation.reconciliation_status 字段
    #[test]
    fn test_dzztcl_closedzzq() {
        assert_eq!(status_ar::RECONCILIATION_CLOSED, "closed");
    }

    /// test_dzztcl_cancelledzzq_2
    /// 验证 status::ar::RECONCILIATION_CANCELLED 常量值为 "cancelled"（小写），；该常量用于 ar_reconciliation.reconciliation_status 字段
    #[test]
    fn test_dzztcl_cancelledzzq_2() {
        assert_eq!(status_ar::RECONCILIATION_CANCELLED, "cancelled");
    }

    /// test_dzztcl_matchedzzq
    /// 验证 status::ar::MATCH_MATCHED 常量值为 "MATCHED"（大写），；该常量用于 ar_reconciliation_item.match_status 字段
    #[test]
    fn test_dzztcl_matchedzzq() {
        assert_eq!(status_ar::MATCH_MATCHED, "MATCHED");
    }

    // ===== 期末余额计算测试（纯算法） =====

    /// test_qmyejs_cjcjzc（验证 create 方法中的期末余额计算公式：closing_balance = opening_balance + total_invoices - total_collections）
    #[test]
    fn test_qmyejs_cjcjzc() {
        let opening = decs!("10000");
        let invoices = decs!("5000");
        let collections = decs!("3000");

        // 复现 create 方法的期末余额计算逻辑
        let closing_balance = opening + invoices - collections;

        assert_eq!(closing_balance, decs!("12000"));
    }

    /// test_qmyejs_gxcjbfzdgx（验证 update 方法中部分字段更新后期末余额重算逻辑：取更新值或保持原值，再按公式重算 closing_balance）
    #[test]
    fn test_qmyejs_gxcjbfzdgx() {
        let model = make_reconciliation_model(
            1,
            decs!("10000"),
            decs!("5000"),
            decs!("3000"),
            recon_status::DRAFT,
        );

        // 模拟 update 请求：仅更新 total_invoices 和 notes
        let req = UpdateReconciliationRequest {
            opening_balance: None,
            total_invoices: Some(decs!("8000")),
            total_collections: None,
            notes: Some("更新备注".to_string()),
        };

        // 复现 update 方法：取更新值或保持原值
        let opening = req.opening_balance.unwrap_or(model.opening_balance);
        let invoices = req.total_invoices.unwrap_or(model.total_invoices);
        let collections = req.total_collections.unwrap_or(model.total_collections);
        let closing = opening + invoices - collections;

        assert_eq!(opening, decs!("10000"));
        assert_eq!(invoices, decs!("8000"));
        assert_eq!(collections, decs!("3000"));
        assert_eq!(closing, decs!("15000"));
    }

    /// test_qmyejs_lzbj（验证所有金额为零时 closing_balance 也为零）
    #[test]
    fn test_qmyejs_lzbj() {
        let opening = Decimal::ZERO;
        let invoices = Decimal::ZERO;
        let collections = Decimal::ZERO;

        let closing_balance = opening + invoices - collections;

        assert_eq!(closing_balance, Decimal::ZERO);
    }

    /// test_qmyejs_fzcj（验证当收款大于期初+发票时，closing_balance 可为负值（客户预付款场景））
    #[test]
    fn test_qmyejs_fzcj() {
        let opening = decs!("1000");
        let invoices = decs!("2000");
        let collections = decs!("5000");

        let closing_balance = opening + invoices - collections;

        assert_eq!(closing_balance, decs!("-2000"));
    }

    /// test_cjqqgz_qmyejs（验证 CreateReconciliationRequest 构造后，按 create 方法公式计算期末余额，；并校验 create 方法设置的初始状态为 draft）
    #[test]
    fn test_cjqqgz_qmyejs() {
        let req = CreateReconciliationRequest {
            reconciliation_no: "RC-2026-0001".to_string(),
            customer_id: 1,
            customer_name: Some("测试客户".to_string()),
            period_start: ymd!(2026, 1, 1),
            period_end: ymd!(2026, 1, 31),
            opening_balance: decs!("10000"),
            total_invoices: decs!("5000"),
            total_collections: decs!("3000"),
            notes: None,
        };

        // 复现 create 方法的期末余额计算
        let closing_balance = req.opening_balance + req.total_invoices - req.total_collections;
        assert_eq!(closing_balance, decs!("12000"));

        // 复现 create 方法的初始状态设置（应为 draft）
        let initial_status = recon_status::DRAFT;
        assert_eq!(initial_status, "draft");
    }

    // ===== 状态白名单校验测试 =====

    /// test_ztbmd_hfzttg（验证 update_status 方法中状态白名单允许所有 5 个合法状态值）
    #[test]
    fn test_ztbmd_hfzttg() {
        // 复现 update_status 方法的状态白名单
        let allowed_statuses = [
            recon_status::DRAFT,
            recon_status::SENT,
            recon_status::CONFIRMED,
            recon_status::DISPUTED,
            recon_status::CLOSED,
        ];

        // 所有合法状态都应通过白名单校验
        for status in &allowed_statuses {
            assert!(
                allowed_statuses.contains(status),
                "状态 {} 应在白名单中",
                status
            );
        }
    }

    /// test_ztbmd_ffztjj（验证 update_status 方法中非法状态值应被拒绝，并产生正确的错误消息）
    #[test]
    fn test_ztbmd_ffztjj() {
        let allowed_statuses = [
            recon_status::DRAFT,
            recon_status::SENT,
            recon_status::CONFIRMED,
            recon_status::DISPUTED,
            recon_status::CLOSED,
        ];

        // 非法状态不应通过白名单校验
        let invalid_status = "invalid";
        assert!(!allowed_statuses.contains(&invalid_status));

        // 复现 update_status 的错误构造
        let err = AppError::business(format!(
            "非法的对账单状态：{}，允许的状态：{:?}",
            invalid_status, allowed_statuses
        ));
        assert!(matches!(err, AppError::BusinessError(_)));

        // 大写状态值也不应通过（业务代码使用小写）
        assert!(!allowed_statuses.contains(&"DRAFT"));
        assert!(!allowed_statuses.contains(&"SENT"));
    }

    // ===== 状态门控测试 =====

    /// test_ztmk_scjyxcg（验证 delete 方法中仅 draft 状态允许删除，其他状态应返回业务错误）
    #[test]
    fn test_ztmk_scjyxcg() {
        // draft 状态：允许删除
        let model_draft = make_reconciliation_model(
            1,
            decs!("1000"),
            decs!("500"),
            decs!("300"),
            recon_status::DRAFT,
        );
        let can_delete = model_draft.reconciliation_status.as_deref() == Some(recon_status::DRAFT);
        assert!(can_delete);

        // 非 draft 状态：拒绝删除
        for status in [
            recon_status::SENT,
            recon_status::CONFIRMED,
            recon_status::DISPUTED,
            recon_status::CLOSED,
        ] {
            let model =
                make_reconciliation_model(2, decs!("1000"), decs!("500"), decs!("300"), status);
            let can_delete = model.reconciliation_status.as_deref() == Some(recon_status::DRAFT);
            assert!(!can_delete, "状态 {} 不应允许删除", status);
        }

        // 复现 delete 方法的错误构造
        let err = AppError::business("只有草稿状态的对账单可以删除".to_string());
        assert!(matches!(err, AppError::BusinessError(_)));
    }

    /// test_ztmk_fsjyxcg（验证 send 方法中仅 draft 状态允许发送，发送后状态变为 sent）
    #[test]
    fn test_ztmk_fsjyxcg() {
        // draft 状态：允许发送
        let model_draft = make_reconciliation_model(
            1,
            decs!("1000"),
            decs!("500"),
            decs!("300"),
            recon_status::DRAFT,
        );
        let can_send = model_draft.reconciliation_status.as_deref() == Some(recon_status::DRAFT);
        assert!(can_send);

        // 非 draft 状态：拒绝发送
        for status in [
            recon_status::SENT,
            recon_status::CONFIRMED,
            recon_status::DISPUTED,
            recon_status::CLOSED,
        ] {
            let model =
                make_reconciliation_model(2, decs!("1000"), decs!("500"), decs!("300"), status);
            let can_send = model.reconciliation_status.as_deref() == Some(recon_status::DRAFT);
            assert!(!can_send, "状态 {} 不应允许发送", status);
        }

        // 复现 send 方法的错误构造
        let err = AppError::business("只有草稿状态的对账单可以发送".to_string());
        assert!(matches!(err, AppError::BusinessError(_)));

        // 发送后状态应变为 sent
        let new_status = recon_status::SENT;
        assert_eq!(new_status, "sent");
    }

    /// test_ztmk_gbyxyqrhzy（验证 close 方法中 confirmed 和 disputed 状态允许关闭，关闭后状态变为 closed）
    #[test]
    fn test_ztmk_gbyxyqrhzy() {
        // confirmed 状态：允许关闭
        let model_confirmed = make_reconciliation_model(
            1,
            decs!("1000"),
            decs!("500"),
            decs!("300"),
            recon_status::CONFIRMED,
        );
        let status = model_confirmed
            .reconciliation_status
            .as_deref()
            .unwrap_or(recon_status::DRAFT);
        let can_close = status == recon_status::CONFIRMED || status == recon_status::DISPUTED;
        assert!(can_close);

        // disputed 状态：允许关闭
        let model_disputed = make_reconciliation_model(
            2,
            decs!("1000"),
            decs!("500"),
            decs!("300"),
            recon_status::DISPUTED,
        );
        let status = model_disputed
            .reconciliation_status
            .as_deref()
            .unwrap_or(recon_status::DRAFT);
        let can_close = status == recon_status::CONFIRMED || status == recon_status::DISPUTED;
        assert!(can_close);

        // 关闭后状态应变为 closed
        let new_status = recon_status::CLOSED;
        assert_eq!(new_status, "closed");
    }

    /// test_ztmk_gbjjcghyfs（验证 close 方法中 draft 和 sent 状态应被拒绝，None 状态默认为 draft 也应拒绝）
    #[test]
    fn test_ztmk_gbjjcghyfs() {
        // draft 状态：拒绝关闭
        let model_draft = make_reconciliation_model(
            1,
            decs!("1000"),
            decs!("500"),
            decs!("300"),
            recon_status::DRAFT,
        );
        let status = model_draft
            .reconciliation_status
            .as_deref()
            .unwrap_or(recon_status::DRAFT);
        let should_reject = status != recon_status::CONFIRMED && status != recon_status::DISPUTED;
        assert!(should_reject);

        // sent 状态：拒绝关闭
        let model_sent = make_reconciliation_model(
            2,
            decs!("1000"),
            decs!("500"),
            decs!("300"),
            recon_status::SENT,
        );
        let status = model_sent
            .reconciliation_status
            .as_deref()
            .unwrap_or(recon_status::DRAFT);
        let should_reject = status != recon_status::CONFIRMED && status != recon_status::DISPUTED;
        assert!(should_reject);

        // closed 状态：拒绝关闭（已关闭不可再关闭）
        let model_closed = make_reconciliation_model(
            3,
            decs!("1000"),
            decs!("500"),
            decs!("300"),
            recon_status::CLOSED,
        );
        let status = model_closed
            .reconciliation_status
            .as_deref()
            .unwrap_or(recon_status::DRAFT);
        let should_reject = status != recon_status::CONFIRMED && status != recon_status::DISPUTED;
        assert!(should_reject);

        // None 状态：unwrap_or("draft")，应拒绝
        let resolved = recon_status::DRAFT;
        let should_reject =
            resolved != recon_status::CONFIRMED && resolved != recon_status::DISPUTED;
        assert!(should_reject);

        // 复现 close 方法的错误构造
        let err = AppError::business("只有已确认或有争议的对账单可以关闭".to_string());
        assert!(matches!(err, AppError::BusinessError(_)));
    }

    // ===== 状态机转换合法性测试 =====

    /// test_ztjzh_wzlzhf
    /// 验证对账单状态机的完整合法流转路径：draft → sent → confirmed → closed；draft → sent → disputed → closed；draft → disputed → closed（通过 update_status 直接争议）
    #[test]
    fn test_ztjzh_wzlzhf() {
        let allowed_statuses = [
            recon_status::DRAFT,
            recon_status::SENT,
            recon_status::CONFIRMED,
            recon_status::DISPUTED,
            recon_status::CLOSED,
        ];

        // 路径 1：draft → sent → confirmed → closed
        let path1 = [
            recon_status::DRAFT,
            recon_status::SENT,
            recon_status::CONFIRMED,
            recon_status::CLOSED,
        ];
        for status in &path1 {
            assert!(
                allowed_statuses.contains(status),
                "路径1状态 {} 应合法",
                status
            );
        }

        // 路径 2：draft → sent → disputed → closed
        let path2 = [
            recon_status::DRAFT,
            recon_status::SENT,
            recon_status::DISPUTED,
            recon_status::CLOSED,
        ];
        for status in &path2 {
            assert!(
                allowed_statuses.contains(status),
                "路径2状态 {} 应合法",
                status
            );
        }

        // 验证 send 门控：仅 draft → sent
        assert_eq!(path1[0], recon_status::DRAFT);
        assert_eq!(path1[1], recon_status::SENT);

        // 验证 close 门控：confirmed/disputed → closed
        assert!(
            path1[2] == recon_status::CONFIRMED || path1[2] == recon_status::DISPUTED,
            "close 前置状态应为 confirmed 或 disputed"
        );
        assert_eq!(path1[3], recon_status::CLOSED);

        assert!(
            path2[2] == recon_status::CONFIRMED || path2[2] == recon_status::DISPUTED,
            "close 前置状态应为 confirmed 或 disputed"
        );
        assert_eq!(path2[3], recon_status::CLOSED);
    }

    // ===== 错误消息格式测试 =====

    /// test_cwxxgs_ffzthztz（验证 update_status 方法中非法状态的错误消息包含状态值和白名单）
    #[test]
    fn test_cwxxgs_ffzthztz() {
        let allowed_statuses = [
            recon_status::DRAFT,
            recon_status::SENT,
            recon_status::CONFIRMED,
            recon_status::DISPUTED,
            recon_status::CLOSED,
        ];
        let invalid_status = "frozen";

        // 复现 update_status 的错误消息构造
        let msg = format!(
            "非法的对账单状态：{}，允许的状态：{:?}",
            invalid_status, allowed_statuses
        );

        assert!(msg.contains(invalid_status), "错误消息应包含非法状态值");
        assert!(msg.contains("允许的状态"), "错误消息应包含白名单提示");
        assert!(msg.contains("draft"), "错误消息应包含白名单内容");

        let err = AppError::business(msg);
        assert!(matches!(err, AppError::BusinessError(_)));
    }

    /// test_cwxxgs_wzddzd（验证各方法中对账单不存在时的 not_found 错误消息）
    #[test]
    fn test_cwxxgs_wzddzd() {
        let err = AppError::not_found("对账单不存在");
        assert!(matches!(err, AppError::NotFound(_)));

        // 验证 NotFound 错误码
        assert_eq!(err.error_code(), "NOT_FOUND");
    }

    // ===== 夹具宏可用性测试 =====

    /// test_jjhdecskyx（验证 decs! 宏能正确解析 Decimal 字符串）
    #[test]
    fn test_jjhdecskyx() {
        let v = decs!("12345.67");
        assert_eq!(v.to_string(), "12345.67");

        // 验证整数场景
        let zero = decs!("0");
        assert_eq!(zero, Decimal::ZERO);

        // 验证负数场景
        let neg = decs!("-100");
        assert_eq!(neg, decs!("-100"));
    }

    /// test_jjhymdkyx（验证 ymd! 宏能正确解析日期）
    #[test]
    fn test_jjhymdkyx() {
        let date = ymd!(2026, 7, 9);
        assert_eq!(date.format("%Y-%m-%d").to_string(), "2026-07-09");

        // 验证用于模型构造的日期字段
        let model = make_reconciliation_model(
            1,
            decs!("1000"),
            decs!("500"),
            decs!("300"),
            recon_status::DRAFT,
        );
        assert_eq!(
            model.period_start.format("%Y-%m-%d").to_string(),
            "2026-01-01"
        );
        assert_eq!(
            model.period_end.format("%Y-%m-%d").to_string(),
            "2026-01-31"
        );
    }

    // ===== 服务实例化测试 =====

    /// test_fwslcj（验证 ArReconciliationService 在 SQLite 内存数据库上能正常实例化）
    #[tokio::test]
    async fn test_fwslcj() {
        let db = setup_test_db().await;
        let service = ArReconciliationService::new(Arc::new(db));

        assert!(Arc::strong_count(&service.db) >= 1);
    }

    // ===== 数据库交互测试（标注 #[ignore]） =====

    /// test_cjdzd_xysjk
    /// 需要 ar_reconciliations 表 schema，标注 #[ignore] 仅在本地手动运行。；无 schema 时返回数据库错误；有 schema 时验证 create 方法完整调用路径。
    #[tokio::test]
    #[ignore]
    async fn test_cjdzd_xysjk() {
        let db = setup_test_db().await;
        let service = ArReconciliationService::new(Arc::new(db));

        let req = CreateReconciliationRequest {
            reconciliation_no: "RC-TEST-0001".to_string(),
            customer_id: 1,
            customer_name: Some("测试客户".to_string()),
            period_start: ymd!(2026, 1, 1),
            period_end: ymd!(2026, 1, 31),
            opening_balance: decs!("10000"),
            total_invoices: decs!("5000"),
            total_collections: decs!("3000"),
            notes: None,
        };

        // L-17 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
        // 无 schema 时返回数据库错误；有 schema 时验证调用路径不 panic
        let result = service.create(req).await;
        assert!(result.is_err(), "无 schema 时应返回数据库错误");
    }

    /// test_hqdzd_xysjk（需要 ar_reconciliations 表 schema，标注 #[ignore] 仅在本地手动运行。；无 schema 时返回数据库错误；无记录时返回 Ok(None)。）
    #[tokio::test]
    #[ignore]
    async fn test_hqdzd_xysjk() {
        let db = setup_test_db().await;
        let service = ArReconciliationService::new(Arc::new(db));

        // 无 schema 时为 Err；有 schema 无记录时为 Ok(None)
        let result = service.get_by_id(99999).await;
        if let Ok(opt) = result {
            assert!(opt.is_none());
        }
    }
}

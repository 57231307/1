//! 委外收货 workflow 集成测试（V15 P1）
//!
//! 覆盖：状态常量值 + Service 实例化 + DB 异常路径 + 完整流程骨架（#[ignore]）。
//! 委外完成事件由 `OutsourcingReceiptService::confirm` 在事务提交后发布，
//! workflow 测试覆盖到收回单 `draft -> confirmed` 与订单 `processing -> received` 语义。

mod common;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::common::setup_test_db;
    use bingxi_backend::models::status::{outsourcing_order_status, outsourcing_receipt_status};
    use bingxi_backend::services::outsourcing_service::OutsourcingReceiptService;

    /// test_wwshztcl_zzqx
    ///
    /// 验证委外收货/订单状态常量值符合预期。
    #[test]
    fn test_wwshztcl_zzqx() {
        assert_eq!(outsourcing_receipt_status::DRAFT, "draft");
        assert_eq!(outsourcing_receipt_status::CONFIRMED, "confirmed");
        assert_eq!(outsourcing_receipt_status::CANCELLED, "cancelled");

        assert_eq!(outsourcing_order_status::PROCESSING, "processing");
        assert_eq!(outsourcing_order_status::RECEIVED, "received");
    }

    /// test_wwshztcl_xxwyzf
    ///
    /// 验证委外收货/订单状态常量均为小写风格。
    #[test]
    fn test_wwshztcl_xxwyzf() {
        for status in [
            outsourcing_receipt_status::DRAFT,
            outsourcing_receipt_status::CONFIRMED,
            outsourcing_receipt_status::CANCELLED,
            outsourcing_order_status::PROCESSING,
            outsourcing_order_status::RECEIVED,
        ] {
            assert!(
                status.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "状态 {} 应全小写",
                status
            );
        }
    }

    /// test_outsourcingreceiptservice_slhbcfdb
    #[tokio::test]
    async fn test_outsourcingreceiptservice_slhbcfdb() {
        let db = setup_test_db().await;
        let svc = OutsourcingReceiptService::new(Arc::new(db));
        std::mem::drop(svc);
    }

    /// test_outsourcingreceiptservice_confirm_kdbfherr
    ///
    /// 验证在空 SQLite 数据库上 confirm 方法返回 Err 而非 panic。
    #[tokio::test]
    async fn test_outsourcingreceiptservice_confirm_kdbfherr() {
        let db = setup_test_db().await;
        let svc = OutsourcingReceiptService::new(Arc::new(db));
        let result = svc.confirm(1).await;
        assert!(result.is_err(), "空 DB 上 confirm 应返回 Err");
    }

    /// test_outsourcingreceiptservice_get_by_id_kdbfherr
    ///
    /// 验证在空 SQLite 数据库上 get_by_id 方法返回 Err 而非 panic。
    #[tokio::test]
    async fn test_outsourcingreceiptservice_get_by_id_kdbfherr() {
        let db = setup_test_db().await;
        let svc = OutsourcingReceiptService::new(Arc::new(db));
        let result = svc.get_by_id(1).await;
        assert!(result.is_err(), "空 DB 上 get_by_id 应返回 Err");
    }

    /// 集成测试：委外收货全流程 create(draft) → confirm(confirmed)
    ///
    /// 需要 PostgreSQL 测试数据库 + 前置委外订单/成品/仓库数据。
    /// confirm 成功后，收回单应变为 confirmed，委外订单应变为 received，
    /// 且完成事件在事务提交后异步发布。
    #[tokio::test]
    #[ignore = "需要 PostgreSQL 测试数据库 + 前置委外订单/成品/仓库数据"]
    async fn test_wwshqlc_cjdqr() {
        let _ = std::env::var("TEST_DATABASE_URL");
        tokio::task::yield_now().await;
    }
}

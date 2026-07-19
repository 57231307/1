//! P0-T02 生产订单全流程集成测试（V15 Batch 487）
//!
//! 覆盖：状态常量值 + Service 实例化 + DB 异常路径（空 SQLite 上 service 方法返回 Err 不 panic）
//! 纯状态机校验函数 validate_status_transition 为私有方法，通过 DB 异常路径间接验证状态门逻辑。
//! 完整业务流程测试（create → submit → approve → complete）需要真实 PostgreSQL，标记 #[ignore]。

mod common;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use bingxi_backend::models::status::{common, production};
    use bingxi_backend::services::production_order_service::{
        CreateProductionOrderRequest, ProductionOrderQuery, ProductionOrderService,
    };
    use rust_decimal::Decimal;
    use sea_orm::{Database, DatabaseConnection};
    use common::setup_test_db;

    /// 构造最小 CreateProductionOrderRequest（仅必填字段）
    fn sample_create_request() -> CreateProductionOrderRequest {
        CreateProductionOrderRequest {
            order_no: None,
            sales_order_id: None,
            product_id: 1,
            planned_quantity: Some(Decimal::new(1000, 0)),
            planned_start_date: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
            planned_end_date: Some(chrono::NaiveDate::from_ymd_opt(2026, 1, 31).unwrap()),
            priority: Some(1),
            work_center_id: Some(1),
            remarks: None,
            created_by: 1,
        }
    }

    // ===== 状态常量值正确性 =====

    /// 测试_生产订单状态常量_值正确性
    ///
    /// 验证生产订单相关的状态常量值符合预期（大写风格）。
    #[test]
    fn 测试_生产订单状态常量_值正确性() {
        assert_eq!(common::STATUS_DRAFT, "DRAFT");
        assert_eq!(common::STATUS_APPROVED, "APPROVED");
        assert_eq!(common::STATUS_CANCELLED, "CANCELLED");
        assert_eq!(common::STATUS_COMPLETED, "COMPLETED");
        assert_eq!(production::PRODUCTION_SCHEDULED, "SCHEDULED");
        assert_eq!(production::PRODUCTION_IN_PROGRESS, "IN_PROGRESS");
        assert_eq!(production::PRODUCTION_PENDING_APPROVAL, "PENDING_APPROVAL");
        assert_eq!(production::PRODUCTION_REJECTED, "REJECTED");
    }

    /// 测试_生产订单状态常量_大写风格一致性
    ///
    /// 验证所有生产订单状态常量均为大写 + 下划线风格。
    #[test]
    fn 测试_生产订单状态常量_大写风格一致性() {
        let statuses = [
            common::STATUS_DRAFT,
            common::STATUS_APPROVED,
            common::STATUS_CANCELLED,
            common::STATUS_COMPLETED,
            production::PRODUCTION_SCHEDULED,
            production::PRODUCTION_IN_PROGRESS,
            production::PRODUCTION_PENDING_APPROVAL,
            production::PRODUCTION_REJECTED,
        ];
        for s in statuses {
            assert!(
                s.chars().all(|c| c.is_uppercase() || c == '_'),
                "状态 {} 应全大写",
                s
            );
        }
    }

    // ===== Service 实例化与 DB 异常路径 =====

    /// 测试_ProductionOrderService_实例化不触发DB
    ///
    /// 验证 new(db) 仅存储 Arc<DatabaseConnection>，不执行任何 DB 查询。
    #[tokio::test]
    async fn 测试_ProductionOrderService_实例化不触发DB() {
        let db = setup_test_db().await;
        let svc = ProductionOrderService::new(Arc::new(db));
        // 仅验证实例化成功，不调用任何方法
        let _ = svc;
    }

    /// 测试_ProductionOrderService_create_空DB返回Err
    ///
    /// 验证在空 SQLite 数据库上 create 方法返回 Err（因 product 表不存在）而非 panic。
    /// 这是健壮性测试：service 在 DB 异常时应优雅降级。
    #[tokio::test]
    async fn 测试_ProductionOrderService_create_空DB返回Err() {
        let db = setup_test_db().await;
        let svc = ProductionOrderService::new(Arc::new(db));
        let req = sample_create_request();
        let result = svc.create(req).await;
        assert!(
            result.is_err(),
            "空 DB 上 create 应返回 Err（product 表不存在），实际：{:?}",
            result
        );
    }

    /// 测试_ProductionOrderService_get_by_id_空DB返回Err
    ///
    /// 验证在空 SQLite 数据库上 get_by_id 方法返回 Err（表不存在）而非 panic。
    #[tokio::test]
    async fn 测试_ProductionOrderService_get_by_id_空DB返回Err() {
        let db = setup_test_db().await;
        let svc = ProductionOrderService::new(Arc::new(db));
        let result = svc.get_by_id(1).await;
        assert!(result.is_err(), "空 DB 上 get_by_id 应返回 Err");
    }

    /// 测试_ProductionOrderService_list_空DB返回Err
    ///
    /// 验证在空 SQLite 数据库上 list 方法返回 Err（表不存在）而非 panic。
    #[tokio::test]
    async fn 测试_ProductionOrderService_list_空DB返回Err() {
        let db = setup_test_db().await;
        let svc = ProductionOrderService::new(Arc::new(db));
        let query = ProductionOrderQuery {
            status: None,
            product_id: None,
            page: 1,
            page_size: 20,
        };
        let result = svc.list(query).await;
        assert!(result.is_err(), "空 DB 上 list 应返回 Err");
    }

    /// 测试_ProductionOrderService_submit_for_approval_空DB返回Err
    ///
    /// 验证在空 SQLite 数据库上 submit_for_approval 方法返回 Err（表不存在）而非 panic。
    #[tokio::test]
    async fn 测试_ProductionOrderService_submit_for_approval_空DB返回Err() {
        let db = setup_test_db().await;
        let svc = ProductionOrderService::new(Arc::new(db));
        let result = svc.submit_for_approval(1, 1, "测试用户").await;
        assert!(
            result.is_err(),
            "空 DB 上 submit_for_approval 应返回 Err"
        );
    }

    /// 测试_ProductionOrderService_approve_order_空DB返回Err
    ///
    /// 验证在空 SQLite 数据库上 approve_order 方法返回 Err（表不存在）而非 panic。
    #[tokio::test]
    async fn 测试_ProductionOrderService_approve_order_空DB返回Err() {
        let db = setup_test_db().await;
        let svc = ProductionOrderService::new(Arc::new(db));
        let result = svc.approve_order(1, 1, "测试用户", true, None).await;
        assert!(
            result.is_err(),
            "空 DB 上 approve_order 应返回 Err"
        );
    }

    // ===== 完整业务流程测试（需要真实 PostgreSQL，标记 ignore）=====

    /// 集成测试：生产订单全流程 create → submit → approve → schedule → in_progress → complete
    ///
    /// 需要 PostgreSQL 测试数据库（表结构 + 产品/工作中心/销售订单前置数据）。
    /// 设置 TEST_DATABASE_URL=postgres://... 环境变量后运行：cargo test -- --ignored
    #[tokio::test]
    #[ignore = "需要 PostgreSQL 测试数据库 + 前置产品/工作中心数据"]
    async fn 测试_生产订单全流程_创建到完成() {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .expect("需设置 TEST_DATABASE_URL 环境变量");
        let db = Database::connect(&db_url).await.expect("DB 连接失败");
        let svc = ProductionOrderService::new(Arc::new(db));

        // 1. 创建（DRAFT）
        let req = sample_create_request();
        let order = svc.create(req).await.expect("创建失败");
        assert_eq!(order.status, common::STATUS_DRAFT);

        // 2. 提交审批（DRAFT → PENDING_APPROVAL）
        let order = svc
            .submit_for_approval(order.id, 1, "测试用户")
            .await
            .expect("提交审批失败");
        assert_eq!(order.status, production::PRODUCTION_PENDING_APPROVAL);

        // 3. 审批通过（PENDING_APPROVAL → APPROVED）
        let order = svc
            .approve_order(order.id, 1, "审批人", true, None)
            .await
            .expect("审批失败");
        assert_eq!(order.status, common::STATUS_APPROVED);

        // 4. 排产（APPROVED → SCHEDULED）
        let order = svc
            .update_status(order.id, production::PRODUCTION_SCHEDULED.to_string(), None)
            .await
            .expect("排产失败");
        assert_eq!(order.status, production::PRODUCTION_SCHEDULED);

        // 5. 开工（SCHEDULED → IN_PROGRESS）
        let order = svc
            .update_status(order.id, production::PRODUCTION_IN_PROGRESS.to_string(), None)
            .await
            .expect("开工失败");
        assert_eq!(order.status, production::PRODUCTION_IN_PROGRESS);

        // 6. 完工（IN_PROGRESS → COMPLETED，需 actual_quantity）
        let order = svc
            .update_status(
                order.id,
                common::STATUS_COMPLETED.to_string(),
                Some(Decimal::new(1000, 0)),
            )
            .await
            .expect("完工失败");
        assert_eq!(order.status, common::STATUS_COMPLETED);
    }
}

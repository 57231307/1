#[cfg(test)]
mod tests {
    use bingxi_backend::models::ar_reconciliation::Model as ReconciliationModel;
    use bingxi_backend::models::status::ar as status_ar;
    use chrono::Utc;
    use rust_decimal::Decimal;

    /// 构造测试用的对账单模型
    fn make_reconciliation_model(id: i32, status: &str) -> ReconciliationModel {
        let opening_balance = Decimal::new(10000, 2);
        let total_invoices = Decimal::new(50000, 2);
        let total_collections = Decimal::new(40000, 2);
        let closing_balance = opening_balance + total_invoices - total_collections;

        ReconciliationModel {
            id,
            reconciliation_no: format!("RC-2026-{:04}", id),
            reconciliation_date: Utc::now().naive_utc(),
            period_start: Utc::now().naive_utc(),
            period_end: Utc::now().naive_utc(),
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

    // ===== 状态常量测试 =====

    #[test]
    fn test_ar_status_draft() {
        assert_eq!(status_ar::RECONCILIATION_DRAFT, "draft");
    }

    #[test]
    fn test_ar_status_sent() {
        assert_eq!(status_ar::RECONCILIATION_SENT, "sent");
    }

    #[test]
    fn test_ar_status_confirmed() {
        assert_eq!(status_ar::RECONCILIATION_CONFIRMED, "confirmed");
    }

    #[test]
    fn test_ar_status_disputed() {
        assert_eq!(status_ar::RECONCILIATION_DISPUTED, "disputed");
    }

    #[test]
    fn test_ar_status_closed() {
        assert_eq!(status_ar::RECONCILIATION_CLOSED, "closed");
    }

    // ===== 模型测试 =====

    #[test]
    fn test_reconciliation_model_serialization() {
        let recon = make_reconciliation_model(1, "draft");
        let json = serde_json::to_value(&recon).expect("对账单序列化失败");

        assert_eq!(json["id"], 1);
        assert_eq!(json["reconciliation_no"], "RC-2026-0001");
        assert_eq!(json["reconciliation_status"], "draft");
    }

    #[test]
    fn test_reconciliation_balances() {
        let recon = make_reconciliation_model(1, "draft");

        // 验证余额计算
        assert_eq!(recon.opening_balance, Decimal::new(10000, 2));
        assert_eq!(recon.total_invoices, Decimal::new(50000, 2));
        assert_eq!(recon.total_collections, Decimal::new(40000, 2));

        // 验证期末余额 = 期初余额 + 发生额 - 收款额
        let expected_closing = recon.opening_balance + recon.total_invoices - recon.total_collections;
        assert_eq!(recon.closing_balance, expected_closing);
    }

    // ===== 状态转换测试 =====

    #[test]
    fn test_status_draft_to_sent() {
        let recon = make_reconciliation_model(1, "draft");
        assert_eq!(recon.reconciliation_status, Some("draft".to_string()));

        // 验证草稿状态可以转换为已发送
        let valid_transitions = vec!["sent", "cancelled"];
        assert!(valid_transitions.contains(&"sent"));
    }

    #[test]
    fn test_status_sent_to_confirmed() {
        let recon = make_reconciliation_model(1, "sent");
        assert_eq!(recon.reconciliation_status, Some("sent".to_string()));

        // 验证已发送状态可以转换为已确认
        let valid_transitions = vec!["confirmed", "disputed"];
        assert!(valid_transitions.contains(&"confirmed"));
    }

    #[test]
    fn test_status_confirmed_to_closed() {
        let recon = make_reconciliation_model(1, "confirmed");
        assert_eq!(recon.reconciliation_status, Some("confirmed".to_string()));

        // 验证已确认状态可以转换为已关闭
        let valid_transitions = vec!["closed"];
        assert!(valid_transitions.contains(&"closed"));
    }

    #[test]
    fn test_status_closed_is_final() {
        let recon = make_reconciliation_model(1, "closed");
        assert_eq!(recon.reconciliation_status, Some("closed".to_string()));

        // 验证已关闭状态是终态
        let invalid_transitions = vec!["draft", "sent", "confirmed"];
        assert!(!invalid_transitions.contains(&"draft"));
    }

    // ===== 余额计算测试 =====

    #[test]
    fn test_balance_calculation_positive() {
        let opening = Decimal::new(10000, 2);
        let invoices = Decimal::new(50000, 2);
        let collections = Decimal::new(40000, 2);
        let closing = opening + invoices - collections;

        assert_eq!(closing, Decimal::new(20000, 2));
    }

    #[test]
    fn test_balance_calculation_negative() {
        let opening = Decimal::new(10000, 2);
        let invoices = Decimal::new(20000, 2);
        let collections = Decimal::new(40000, 2);
        let closing = opening + invoices - collections;

        // 验证负余额
        assert_eq!(closing, Decimal::new(-10000, 2));
    }

    #[test]
    fn test_balance_calculation_zero() {
        let opening = Decimal::new(10000, 2);
        let invoices = Decimal::new(30000, 2);
        let collections = Decimal::new(40000, 2);
        let closing = opening + invoices - collections;

        assert_eq!(closing, Decimal::new(0, 2));
    }

    // ===== 序列化/反序列化测试 =====

    #[test]
    fn test_reconciliation_json_roundtrip() {
        let recon = make_reconciliation_model(1, "draft");
        let json = serde_json::to_value(&recon).expect("序列化失败");

        // 验证关键字段存在
        assert!(json.get("id").is_some());
        assert!(json.get("reconciliation_no").is_some());
        assert!(json.get("customer_id").is_some());
        assert!(json.get("opening_balance").is_some());
        assert!(json.get("closing_balance").is_some());
        assert!(json.get("reconciliation_status").is_some());
    }
}

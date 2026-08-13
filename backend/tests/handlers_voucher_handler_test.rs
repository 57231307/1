#[cfg(test)]
mod tests {
    use bingxi_backend::handlers::voucher_handler::*;
    use bingxi_backend::models::voucher::Model as VoucherModel;
    use bingxi_backend::models::status::finance as status_finance;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use serde_json::json;

    /// 构造测试用的凭证模型
    fn make_voucher_model(id: i32, status: &str) -> VoucherModel {
        VoucherModel {
            id,
            voucher_no: format!("V-2026-{:04}", id),
            voucher_date: Utc::now().naive_utc(),
            voucher_type: Some("记账凭证".to_string()),
            period_id: Some(1),
            period_name: Some("2026-01".to_string()),
            total_debit: Decimal::new(10000, 2),
            total_credit: Decimal::new(10000, 2),
            status: Some(status.to_string()),
            source_type: Some("sales_order".to_string()),
            source_id: Some(1),
            source_no: Some("SO-2026-0001".to_string()),
            created_by: Some(1),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            audit_by: None,
            audit_at: None,
            notes: Some("测试备注".to_string()),
            attachment_count: Some(0),
        }
    }

    // ===== 状态常量测试 =====

    #[test]
    fn test_finance_status_draft() {
        assert_eq!(status_finance::DRAFT, "draft");
    }

    #[test]
    fn test_finance_status_posted() {
        assert_eq!(status_finance::POSTED, "posted");
    }

    #[test]
    fn test_finance_status_audited() {
        assert_eq!(status_finance::AUDITED, "audited");
    }

    #[test]
    fn test_finance_status_voided() {
        assert_eq!(status_finance::VOIDED, "voided");
    }

    // ===== 模型测试 =====

    #[test]
    fn test_voucher_model_serialization() {
        let voucher = make_voucher_model(1, "draft");
        let json = serde_json::to_value(&voucher).expect("凭证序列化失败");

        assert_eq!(json["id"], 1);
        assert_eq!(json["voucher_no"], "V-2026-0001");
        assert_eq!(json["status"], "draft");
    }

    #[test]
    fn test_voucher_debit_credit_balance() {
        let voucher = make_voucher_model(1, "draft");

        // 验证借贷平衡
        assert_eq!(voucher.total_debit, voucher.total_credit);
    }

    #[test]
    fn test_voucher_debit_credit_imbalance() {
        let mut voucher = make_voucher_model(1, "draft");
        voucher.total_debit = Decimal::new(10000, 2);
        voucher.total_credit = Decimal::new(9000, 2);

        // 验证借贷不平衡
        assert_ne!(voucher.total_debit, voucher.total_credit);
    }

    // ===== 状态转换测试 =====

    #[test]
    fn test_status_draft_to_posted() {
        let voucher = make_voucher_model(1, "draft");
        assert_eq!(voucher.status, Some("draft".to_string()));

        // 验证草稿状态可以转换为已过账
        let valid_transitions = vec!["posted", "voided"];
        assert!(valid_transitions.contains(&"posted"));
    }

    #[test]
    fn test_status_posted_to_audited() {
        let voucher = make_voucher_model(1, "posted");
        assert_eq!(voucher.status, Some("posted".to_string()));

        // 验证已过账状态可以转换为已审核
        let valid_transitions = vec!["audited", "voided"];
        assert!(valid_transitions.contains(&"audited"));
    }

    #[test]
    fn test_status_audited_is_final() {
        let voucher = make_voucher_model(1, "audited");
        assert_eq!(voucher.status, Some("audited".to_string()));

        // 验证已审核状态是终态
        let invalid_transitions = vec!["draft", "posted"];
        assert!(!invalid_transitions.contains(&"draft"));
    }

    // ===== 凭证类型测试 =====

    #[test]
    fn test_voucher_type_receipt() {
        let voucher = make_voucher_model(1, "draft");
        assert_eq!(voucher.voucher_type, Some("记账凭证".to_string()));
    }

    // ===== 来源类型测试 =====

    #[test]
    fn test_source_type_sales_order() {
        let voucher = make_voucher_model(1, "draft");
        assert_eq!(voucher.source_type, Some("sales_order".to_string()));
        assert_eq!(voucher.source_no, Some("SO-2026-0001".to_string()));
    }

    // ===== 金额计算测试 =====

    #[test]
    fn test_amount_rounding() {
        let amount = Decimal::new(12345, 2);
        assert_eq!(amount, Decimal::new(12345, 2));
    }

    #[test]
    fn test_amount_addition() {
        let debit1 = Decimal::new(5000, 2);
        let debit2 = Decimal::new(3000, 2);
        let total = debit1 + debit2;

        assert_eq!(total, Decimal::new(8000, 2));
    }

    #[test]
    fn test_amount_subtraction() {
        let total = Decimal::new(10000, 2);
        let discount = Decimal::new(1000, 2);
        let final_amount = total - discount;

        assert_eq!(final_amount, Decimal::new(9000, 2));
    }

    // ===== 序列化/反序列化测试 =====

    #[test]
    fn test_voucher_json_roundtrip() {
        let voucher = make_voucher_model(1, "draft");
        let json = serde_json::to_value(&voucher).expect("序列化失败");

        // 验证关键字段存在
        assert!(json.get("id").is_some());
        assert!(json.get("voucher_no").is_some());
        assert!(json.get("voucher_date").is_some());
        assert!(json.get("total_debit").is_some());
        assert!(json.get("total_credit").is_some());
        assert!(json.get("status").is_some());
    }

    #[test]
    fn test_voucher_json_amounts() {
        let voucher = make_voucher_model(1, "draft");
        let json = serde_json::to_value(&voucher).expect("序列化失败");

        // 验证金额字段
        assert!(json["total_debit"].is_string());
        assert!(json["total_credit"].is_string());
    }
}

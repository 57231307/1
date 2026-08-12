#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use super::*;

    #[test]
    fn 测试计算损耗率_正常() {
        // 损耗 2 吨，发出 100 吨 → 2%
        let result = compute_loss_rate(Decimal::new(2, 0), Decimal::new(100, 0));
        assert_eq!(result, Decimal::new(2, 2)); // 0.02
    }

    #[test]
    fn 测试计算损耗率_发出为零返回零() {
        let result = compute_loss_rate(Decimal::new(2, 0), Decimal::ZERO);
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn 测试计算总成本_正常() {
        // 材料 500000 + 加工费 100000 + 运费 0 - 非正常损耗 0 = 600000
        let result = compute_total_cost(
            Decimal::new(500000, 0),
            Decimal::new(100000, 0),
            Decimal::ZERO,
            Decimal::ZERO,
        );
        assert_eq!(result, Decimal::new(600000, 0));
    }

    #[test]
    fn 测试计算总成本_扣除非正常损耗() {
        // 材料 500000 + 加工费 100000 + 运费 0 - 非正常损耗 5000 = 595000
        let result = compute_total_cost(
            Decimal::new(500000, 0),
            Decimal::new(100000, 0),
            Decimal::ZERO,
            Decimal::new(5000, 0),
        );
        assert_eq!(result, Decimal::new(595000, 0));
    }

    #[test]
    fn 测试计算单位成本_正常() {
        // 总成本 600000 / 收回 298 = 2013.4228...
        let result = compute_unit_cost(Decimal::new(600000, 0), Decimal::new(298, 0));
        assert!(result > Decimal::ZERO);
    }

    #[test]
    fn 测试计算单位成本_收回为零返回零() {
        let result = compute_unit_cost(Decimal::new(600000, 0), Decimal::ZERO);
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn 测试计算标准损耗率_染色() {
        // dyeing 印染工序中值 5%
        let result = compute_standard_loss_rate(outsourcing_order_type::DYEING);
        assert_eq!(result, Decimal::new(5, 2));
    }

    #[test]
    fn 测试计算标准损耗率_织布() {
        // weaving 织布工序中值 3.5%
        let result = compute_standard_loss_rate(outsourcing_order_type::WEAVING);
        assert_eq!(result, Decimal::new(35, 3));
    }

    #[test]
    fn 测试计算标准损耗率_其他() {
        // other 无标准 0
        let result = compute_standard_loss_rate(outsourcing_order_type::OTHER);
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn 测试损耗分类_正常损耗() {
        // 实际 0.02 ≤ 标准 0.05 → normal
        let result = classify_loss(Decimal::new(2, 2), Decimal::new(5, 2));
        assert_eq!(result, outsourcing_loss_type::NORMAL);
    }

    #[test]
    fn 测试损耗分类_非正常损耗() {
        // 实际 0.08 > 标准 0.05 → abnormal
        let result = classify_loss(Decimal::new(8, 2), Decimal::new(5, 2));
        assert_eq!(result, outsourcing_loss_type::ABNORMAL);
    }

    #[test]
    fn 测试计算非正常损耗金额_正常无超定额() {
        // 发出 300，收回 298，损耗 2，标准 0.05 → 标准损耗 15，超定额 0
        let result = compute_abnormal_loss_amount(
            Decimal::new(300, 0),
            Decimal::new(298, 0),
            Decimal::new(1666, 0), // 单位材料成本
            Decimal::new(5, 2),    // 0.05
        );
        assert_eq!(result, Decimal::ZERO);
    }

    #[test]
    fn 测试计算非正常损耗金额_有超定额() {
        // 发出 100，收回 90，损耗 10，标准 0.05 → 标准损耗 5，超定额 5
        // 单位材料成本 1000 → 非正常损耗金额 5 × 1000 = 5000
        let result = compute_abnormal_loss_amount(
            Decimal::new(100, 0),
            Decimal::new(90, 0),
            Decimal::new(1000, 0),
            Decimal::new(5, 2),
        );
        assert_eq!(result, Decimal::new(5000, 0));
    }

    #[test]
    fn 测试校验委外类型_合法() {
        assert!(validate_order_type("dyeing").is_ok());
        assert!(validate_order_type("printing").is_ok());
        assert!(validate_order_type("weaving").is_ok());
        assert!(validate_order_type("finishing").is_ok());
        assert!(validate_order_type("other").is_ok());
    }

    #[test]
    fn 测试校验委外类型_非法() {
        assert!(validate_order_type("invalid").is_err());
    }

    #[test]
    fn 测试校验委外订单状态_合法() {
        assert!(validate_order_status("draft").is_ok());
        assert!(validate_order_status("issued").is_ok());
        assert!(validate_order_status("processing").is_ok());
        assert!(validate_order_status("received").is_ok());
        assert!(validate_order_status("settled").is_ok());
        assert!(validate_order_status("closed").is_ok());
        assert!(validate_order_status("cancelled").is_ok());
    }

    #[test]
    fn 测试校验委外订单状态_非法() {
        assert!(validate_order_status("invalid").is_err());
    }

    #[test]
    fn 测试校验损耗类型_合法() {
        assert!(validate_loss_type("normal").is_ok());
        assert!(validate_loss_type("abnormal").is_ok());
    }

    #[test]
    fn 测试校验损耗类型_非法() {
        assert!(validate_loss_type("invalid").is_err());
    }

    #[test]
    fn 测试校验凭证类型_合法() {
        assert!(validate_voucher_type("issue").is_ok());
        assert!(validate_voucher_type("fee").is_ok());
        assert!(validate_voucher_type("receipt").is_ok());
        assert!(validate_voucher_type("loss").is_ok());
    }

    #[test]
    fn 测试校验凭证类型_非法() {
        assert!(validate_voucher_type("invalid").is_err());
    }
}
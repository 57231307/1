//! 研发费用加计扣除服务（V15 P2 B08-P2-7）
//!
//! 依据：《财政部 税务总局 科技部关于提高研究开发费用税前加计扣除比例的公告》
//! 业务：识别研发费用（基于科目分类）并计算加计扣除金额（75%/100%）
use std::sync::Arc;

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;

/// 研发费用加计扣除服务
pub struct RndSuperDeductionService {
    #[allow(dead_code)]
    db: Arc<DatabaseConnection>,
}

impl RndSuperDeductionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 计算研发费用加计扣除金额
    ///
    /// 参数：
    /// - `rd_expense`: 研发费用总额
    /// - `deduction_rate`: 加计扣除比例（0.75 或 1.00）
    ///
    /// 返回：加计扣除金额 = 研发费用 × 扣除比例
    pub fn calculate_super_deduction(
        &self,
        rd_expense: Decimal,
        deduction_rate: Decimal,
    ) -> Decimal {
        rd_expense * deduction_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn test_calculate_super_deduction_75_percent() {
        let db = Arc::new(DatabaseConnection::default());
        let service = RndSuperDeductionService::new(db);
        let rd_expense = Decimal::from_str("1000000").unwrap();
        let rate = Decimal::from_str("0.75").unwrap();
        let result = service.calculate_super_deduction(rd_expense, rate);
        assert_eq!(result, Decimal::from_str("750000").unwrap());
    }

    #[test]
    fn test_calculate_super_deduction_100_percent() {
        let db = Arc::new(DatabaseConnection::default());
        let service = RndSuperDeductionService::new(db);
        let rd_expense = Decimal::from_str("1000000").unwrap();
        let rate = Decimal::from_str("1.00").unwrap();
        let result = service.calculate_super_deduction(rd_expense, rate);
        assert_eq!(result, Decimal::from_str("1000000").unwrap());
    }
}

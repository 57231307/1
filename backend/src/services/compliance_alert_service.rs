//! 合规预警服务
//! V15 P2 B08-12：假冒伪劣/虚假宣传/商业贿赂预警
#[allow(dead_code)]
pub struct ComplianceAlertService;

impl ComplianceAlertService {
    /// 检查价格异常（低于市场均价 50% 可能为假冒伪劣）
    pub fn check_price_anomaly(
        price: rust_decimal::Decimal,
        market_avg: rust_decimal::Decimal,
    ) -> bool {
        if market_avg.is_zero() {
            return false;
        }
        price < market_avg * rust_decimal::Decimal::new(5, 1) // 50%
    }

    /// 检查虚假宣传关键词
    pub fn check_false_advertising(text: &str) -> Vec<String> {
        let keywords = ["最", "第一", "唯一", "绝对", "永不褪色", "100% 不退色"];
        keywords
            .iter()
            .filter(|k| text.contains(*k))
            .map(|k| k.to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_type_code() {
        assert_eq!(AlertType::OutOfStock.code(), "out_of_stock");
        assert_eq!(AlertType::LowStock.code(), "low_stock");
        assert_eq!(AlertType::OverStock.code(), "over_stock");
        assert_eq!(AlertType::Expiring.code(), "expiring");
        assert_eq!(AlertType::SlowMoving.code(), "slow_moving");
        assert_eq!(AlertType::Discrepancy.code(), "discrepancy");
    }

    #[test]
    fn test_normal_constant() {
        assert_eq!(ALERT_TYPE_NORMAL, "normal");
    }
}
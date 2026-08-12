    use super::*;
#[cfg(test)]
mod tests {

    #[test]
    fn test_alert_desc() {
        assert_eq!(
            StockAlertNotificationScheduler::alert_desc("out_of_stock"),
            "缺货"
        );
        assert_eq!(
            StockAlertNotificationScheduler::alert_desc("low_stock"),
            "低于下限"
        );
        assert_eq!(
            StockAlertNotificationScheduler::alert_desc("over_stock"),
            "高于上限"
        );
        assert_eq!(
            StockAlertNotificationScheduler::alert_desc("expiring"),
            "即将过期"
        );
        assert_eq!(
            StockAlertNotificationScheduler::alert_desc("slow_moving"),
            "滞销"
        );
        assert_eq!(
            StockAlertNotificationScheduler::alert_desc("discrepancy"),
            "盘点差异"
        );
        assert_eq!(
            StockAlertNotificationScheduler::alert_desc("unknown"),
            "未知告警"
        );
    }

    #[test]
    fn test_default_constants() {
        assert_eq!(DEFAULT_INTERVAL_SECS, 21600);
        assert_eq!(INITIAL_DELAY_SECS, 180);
        assert_eq!(MAX_ALERTS_PER_SCAN, 500);
    }
}
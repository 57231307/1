    use super::*;
#[cfg(test)]
mod tests {

    #[test]
    fn 测试指数退避延迟计算() {
        // retry_count=0: 1 * 2^0 = 1 秒
        assert_eq!(
            EventRetryService::calculate_backoff_delay(0),
            Duration::from_secs(1)
        );
        // retry_count=1: 1 * 2^1 = 2 秒
        assert_eq!(
            EventRetryService::calculate_backoff_delay(1),
            Duration::from_secs(2)
        );
        // retry_count=2: 1 * 2^2 = 4 秒
        assert_eq!(
            EventRetryService::calculate_backoff_delay(2),
            Duration::from_secs(4)
        );
        // retry_count=3: 1 * 2^3 = 8 秒
        assert_eq!(
            EventRetryService::calculate_backoff_delay(3),
            Duration::from_secs(8)
        );
        // retry_count=10: 上限 60 秒
        assert_eq!(
            EventRetryService::calculate_backoff_delay(10),
            Duration::from_secs(60)
        );
        // 负数退化为 0
        assert_eq!(
            EventRetryService::calculate_backoff_delay(-1),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn 测试最大重试次数常量() {
        assert_eq!(MAX_RETRY_COUNT, 5);
    }
}
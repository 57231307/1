#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // P9-1: 测试夹具 helper，统一 build_registry_and_metrics 的 expect
    fn build_metrics() -> (Arc<prometheus::Registry>, BusinessMetrics) {
        build_registry_and_metrics().expect("P9-1: 测试夹具 metrics 注册失败")
    }

    #[test]
    fn test_business_metrics_zc() {
        // 中文测试名：测试 business metrics 全部注册成功
        let (registry, _m) = build_metrics();
        let families = registry.gather();
        // 至少 20+ 个指标家族
        assert!(
            families.len() >= 20,
            "指标家族数应 >= 20，实际: {}",
            families.len()
        );
    }

    #[test]
    fn test_hcmzl() {
        // 中文测试名：测试缓存命中率计算
        let (_r, m) = build_metrics();
        m.record_cache_hit();
        m.record_cache_hit();
        m.record_cache_hit();
        m.record_cache_miss();
        let ratio = m.cache_hit_ratio();
        assert!((ratio - 0.75).abs() < 1e-9);
    }

    #[test]
    fn test_dljl() {
        // 中文测试名：测试登录成功/失败记录
        let (_r, m) = build_metrics();
        m.record_login(true);
        m.record_login(true);
        m.record_login(false);
        // 验证不 panic
    }
}
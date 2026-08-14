
use bingxi_backend::utils::export_concurrency::*;
use std::sync::atomic::Ordering;

/// 测试守卫获取后计数器递增，Drop 后递减
#[test]
fn test_guard_increments_and_decrements() {
    let before = CONCURRENT_EXPORTS.load(Ordering::Acquire);
    {
        let _guard = ExportConcurrencyGuard::acquire().expect("应能获取守卫");
        let during = CONCURRENT_EXPORTS.load(Ordering::Acquire);
        assert_eq!(during, before + 1, "守卫存在时计数器应递增");
    }
    let after = CONCURRENT_EXPORTS.load(Ordering::Acquire);
    assert_eq!(after, before, "守卫 Drop 后计数器应恢复");
}

/// 测试 MAX_CONCURRENT_EXPORTS 常量为 10
#[test]
fn test_max_concurrent_exports_is_10() {
    assert_eq!(MAX_CONCURRENT_EXPORTS, 10, "全局导出并发上限应为 10");
}

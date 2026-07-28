//! V15 P1-9-1：全局导出并发控制工具
//!
//! 设计依据：审计报告 batch-11 P1-9-1（导出无全局并发控制）
//! 计划 13.9.2 要求 MAX_CONCURRENT_EXPORTS = 10 全局并发上限。
//!
//! 实现要点：
//! - 使用进程级 `AtomicUsize` 计数器，限制同时进行的导出操作数量；
//! - 满载时返回 `AppError::too_many_requests`（HTTP 429），引导客户端稍后重试；
//! - RAII 守卫 `ExportConcurrencyGuard` 在 Drop 时自动递减，确保 panic/早返/错误路径不泄漏计数；
//! - 所有导出 handler（sales_order/purchase_order/crm/import_export 等）共享同一计数器，
//!   实现真正的"全局"并发上限，而非单 handler 内的局部上限。

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::utils::error::AppError;

/// 全局导出并发计数器（进程级共享，所有导出 handler 共用）
static CONCURRENT_EXPORTS: AtomicUsize = AtomicUsize::new(0);

/// 全局导出并发上限（计划 13.9.2 要求 10）
pub const MAX_CONCURRENT_EXPORTS: usize = 10;

/// 导出并发计数守卫，Drop 时自动递减（RAII / scopeguard 模式）
///
/// 通过 `ExportConcurrencyGuard::acquire()` 创建，满载时返回 429 错误。
/// 守卫离开作用域时自动递减计数器，确保 panic/早返/错误路径均递减，避免计数器泄漏。
pub struct ExportConcurrencyGuard;

impl ExportConcurrencyGuard {
    /// 尝试递增并发计数器，满载时返回 429 错误
    pub fn acquire() -> Result<Self, AppError> {
        let current = CONCURRENT_EXPORTS.load(Ordering::Acquire);
        if current >= MAX_CONCURRENT_EXPORTS {
            return Err(AppError::too_many_requests(format!(
                "导出并发已满（{}/{}），请稍后重试",
                current, MAX_CONCURRENT_EXPORTS
            )));
        }
        match CONCURRENT_EXPORTS.compare_exchange(
            current,
            current + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                tracing::debug!("导出并发计数器递增: {} -> {}", current, current + 1);
                Ok(ExportConcurrencyGuard)
            }
            Err(actual) => {
                if actual >= MAX_CONCURRENT_EXPORTS {
                    return Err(AppError::too_many_requests(format!(
                        "导出并发已满（{}/{}），请稍后重试",
                        actual, MAX_CONCURRENT_EXPORTS
                    )));
                }
                CONCURRENT_EXPORTS.fetch_add(1, Ordering::AcqRel);
                tracing::debug!("导出并发计数器递增（fetch_add fallback）");
                Ok(ExportConcurrencyGuard)
            }
        }
    }
}

impl Drop for ExportConcurrencyGuard {
    fn drop(&mut self) {
        CONCURRENT_EXPORTS.fetch_sub(1, Ordering::AcqRel);
        tracing::debug!(
            "导出并发计数器递减: {}",
            CONCURRENT_EXPORTS.load(Ordering::Acquire)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

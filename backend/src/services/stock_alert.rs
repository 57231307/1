//! P9-2 库存预警子模块
//!
//! 拆分自原 `services/inventory_stock_service.rs`。
//!
//! ## 模块职责
//! - 库存上下限预警
//! - 过期预警
//! - 滞销库存预警
//! - 预警规则配置
//!
//! ## 批次 126 v8 复审 P2 修复
//! - 移除死代码 AlertLevel 枚举（sensitive_action_alert.rs 有独立的 AlertLevel 实现，
//!   本模块的 AlertLevel 仅测试使用，零业务调用方，按死代码处理规范删除）
//! - AlertType 接入 inventory_stock_query.compute_alert_type 业务，移除 dead_code 标注
//! - 新增 OutOfStock 变体（缺货）+ code() 方法返回前端约定的稳定字符串

/// 预警类型
///
/// 批次 126 v8 复审 P2 修复：接入 inventory_stock_query.get_stock_alerts 业务，
/// compute_alert_type 函数根据库存数量/补货点/过期日期派生 AlertType。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertType {
    /// 缺货（可用量为 0 且设置了补货点）
    OutOfStock,
    /// 低于下限（可用量 < 补货点）
    LowStock,
    /// 高于上限
    OverStock,
    /// 即将过期（expiry_date 距今 ≤ 30 天）
    Expiring,
    /// 滞销
    SlowMoving,
    /// 差异（库存状态非"正常"）
    Discrepancy,
}

impl AlertType {
    /// 中文描述
    pub fn desc(&self) -> &'static str {
        match self {
            Self::OutOfStock => "缺货",
            Self::LowStock => "低于下限",
            Self::OverStock => "高于上限",
            Self::Expiring => "即将过期",
            Self::SlowMoving => "滞销",
            Self::Discrepancy => "盘点差异",
        }
    }

    /// 返回前端约定的稳定字符串代码
    ///
    /// 批次 126 v8 复审 P2 修复：供 inventory_stock_query.compute_alert_type 使用，
    /// 作为 alert_type 字段值返回给前端。
    pub fn code(&self) -> &'static str {
        match self {
            Self::OutOfStock => "out_of_stock",
            Self::LowStock => "low_stock",
            Self::OverStock => "over_stock",
            Self::Expiring => "expiring",
            Self::SlowMoving => "slow_moving",
            Self::Discrepancy => "discrepancy",
        }
    }
}

/// 正常库存的 alert_type 字符串
///
/// 批次 126 v8 复审 P2 修复：当库存状态正常（无告警）时返回此值。
pub const ALERT_TYPE_NORMAL: &str = "normal";

/// 即将过期的阈值天数（默认 30 天）
///
/// 批次 126 v8 复审 P2 修复：expiry_date 距当前时间 ≤ 此天数视为"即将过期"。
/// TODO(tech-debt): 后续可改为从配置读取，支持按产品类别差异化配置。
pub const EXPIRING_THRESHOLD_DAYS: i64 = 30;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_type_desc() {
        assert_eq!(AlertType::OutOfStock.desc(), "缺货");
        assert_eq!(AlertType::LowStock.desc(), "低于下限");
        assert_eq!(AlertType::OverStock.desc(), "高于上限");
        assert_eq!(AlertType::Expiring.desc(), "即将过期");
        assert_eq!(AlertType::SlowMoving.desc(), "滞销");
        assert_eq!(AlertType::Discrepancy.desc(), "盘点差异");
    }

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

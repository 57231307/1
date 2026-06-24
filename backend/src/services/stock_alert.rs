//! P9-2 库存预警子模块（占位）
//!
//! 拆分自原 `services/inventory_stock_service.rs`。
//!
//! ## 模块职责
//! - 库存上下限预警
//! - 过期预警
//! - 滞销库存预警
//! - 预警规则配置

/// 预警级别
#[allow(dead_code)] // TODO(tech-debt): P9-2 业务接入后移除
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertLevel {
    /// 提示
    Info,
    /// 警告
    Warning,
    /// 严重
    Critical,
}

impl AlertLevel {
    /// 中文描述
    #[allow(dead_code)] // TODO(tech-debt): P9-2 业务接入后移除
    pub fn desc(&self) -> &'static str {
        match self {
            Self::Info => "提示",
            Self::Warning => "警告",
            Self::Critical => "严重",
        }
    }
}

/// 预警类型
#[allow(dead_code)] // TODO(tech-debt): P9-2 业务接入后移除
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertType {
    /// 低于下限
    LowStock,
    /// 高于上限
    OverStock,
    /// 即将过期
    Expiring,
    /// 滞销
    SlowMoving,
    /// 差异
    Discrepancy,
}

impl AlertType {
    /// 中文描述
    #[allow(dead_code)] // TODO(tech-debt): P9-2 业务接入后移除
    pub fn desc(&self) -> &'static str {
        match self {
            Self::LowStock => "低于下限",
            Self::OverStock => "高于上限",
            Self::Expiring => "即将过期",
            Self::SlowMoving => "滞销",
            Self::Discrepancy => "盘点差异",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_level_ordering() {
        assert!(AlertLevel::Info < AlertLevel::Warning);
        assert!(AlertLevel::Warning < AlertLevel::Critical);
    }

    #[test]
    fn test_alert_level_desc() {
        assert_eq!(AlertLevel::Info.desc(), "提示");
        assert_eq!(AlertLevel::Warning.desc(), "警告");
        assert_eq!(AlertLevel::Critical.desc(), "严重");
    }

    #[test]
    fn test_alert_type_desc() {
        assert_eq!(AlertType::LowStock.desc(), "低于下限");
        assert_eq!(AlertType::OverStock.desc(), "高于上限");
        assert_eq!(AlertType::Expiring.desc(), "即将过期");
        assert_eq!(AlertType::SlowMoving.desc(), "滞销");
        assert_eq!(AlertType::Discrepancy.desc(), "盘点差异");
    }

}

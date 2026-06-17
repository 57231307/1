//! P9-2 库存台账子模块（占位）
//!
//! 拆分自原 `services/inventory_stock_service.rs`。
//!
//! ## 模块职责
//! - 库存出入库台账
//! - 库存移动记录
//! - 台账查询（按时间/产品/仓库）

/// P9-2 标记：库存台账子模块路径
pub const P92_LEDGER_MODULE: &str = "stock_ledger";

/// 库存移动类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementType {
    /// 入库
    Inbound,
    /// 出库
    Outbound,
    /// 调拨
    Transfer,
    /// 调整
    Adjustment,
    /// 盘点
    Stocktake,
    /// 报损
    Damage,
}

impl MovementType {
    /// 中文描述
    pub fn desc(&self) -> &'static str {
        match self {
            Self::Inbound => "入库",
            Self::Outbound => "出库",
            Self::Transfer => "调拨",
            Self::Adjustment => "调整",
            Self::Stocktake => "盘点",
            Self::Damage => "报损",
        }
    }

    /// 是否为正向（增加库存）
    pub fn is_positive(&self) -> bool {
        matches!(self, Self::Inbound | Self::Adjustment | Self::Stocktake)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movement_type_desc() {
        assert_eq!(MovementType::Inbound.desc(), "入库");
        assert_eq!(MovementType::Outbound.desc(), "出库");
        assert_eq!(MovementType::Transfer.desc(), "调拨");
        assert_eq!(MovementType::Adjustment.desc(), "调整");
        assert_eq!(MovementType::Stocktake.desc(), "盘点");
        assert_eq!(MovementType::Damage.desc(), "报损");
    }

    #[test]
    fn test_movement_type_is_positive() {
        assert!(MovementType::Inbound.is_positive());
        assert!(MovementType::Adjustment.is_positive());
        assert!(MovementType::Stocktake.is_positive());
        assert!(!MovementType::Outbound.is_positive());
        assert!(!MovementType::Transfer.is_positive());
        assert!(!MovementType::Damage.is_positive());
    }

    #[test]
    fn test_module_loaded() {
        assert_eq!(P92_LEDGER_MODULE, "stock_ledger");
    }
}

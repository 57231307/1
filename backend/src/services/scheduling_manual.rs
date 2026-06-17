//! P9-2 排程手动调整子模块（占位）
//!
//! 拆分自原 `services/scheduling_service.rs`。
//!
//! ## 模块职责
//! - 手动调整工单顺序
//! - 锁定/解锁工位
//! - 重新排程

/// P9-2 标记：手动调整子模块路径
pub const P92_MANUAL_MODULE: &str = "scheduling_manual";

/// 调整类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustType {
    /// 上移
    MoveUp,
    /// 下移
    MoveDown,
    /// 置顶
    MoveTop,
    /// 置底
    MoveBottom,
    /// 锁定
    Lock,
    /// 解锁
    Unlock,
}

impl AdjustType {
    /// 中文描述
    pub fn desc(&self) -> &'static str {
        match self {
            Self::MoveUp => "上移",
            Self::MoveDown => "下移",
            Self::MoveTop => "置顶",
            Self::MoveBottom => "置底",
            Self::Lock => "锁定",
            Self::Unlock => "解锁",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adjust_desc() {
        assert_eq!(AdjustType::MoveUp.desc(), "上移");
        assert_eq!(AdjustType::MoveDown.desc(), "下移");
        assert_eq!(AdjustType::MoveTop.desc(), "置顶");
        assert_eq!(AdjustType::MoveBottom.desc(), "置底");
        assert_eq!(AdjustType::Lock.desc(), "锁定");
        assert_eq!(AdjustType::Unlock.desc(), "解锁");
    }

    #[test]
    fn test_module_loaded() {
        assert_eq!(P92_MANUAL_MODULE, "scheduling_manual");
    }
}

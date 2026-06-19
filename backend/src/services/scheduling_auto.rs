//! P9-2 排程自动调度子模块（占位）
//!
//! 拆分自原 `services/scheduling_service.rs`。
//!
//! ## 模块职责
//! - 基于优先级和产能的自动排程
//! - 甘特图数据自动生成
//! - 冲突检测

/// P9-2 标记：自动排程子模块路径
pub const P92_AUTO_MODULE: &str = "scheduling_auto";

/// 排程算法枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingAlgo {
    /// 先进先出
    Fifo,
    /// 优先级优先
    Priority,
    /// 最短加工时间
    Spt,
    /// 最早交货期
    Edd,
}

impl SchedulingAlgo {
    /// 中文描述
    pub fn desc(&self) -> &'static str {
        match self {
            Self::Fifo => "先进先出",
            Self::Priority => "优先级优先",
            Self::Spt => "最短加工时间",
            Self::Edd => "最早交货期",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_algo_desc() {
        assert_eq!(SchedulingAlgo::Fifo.desc(), "先进先出");
        assert_eq!(SchedulingAlgo::Priority.desc(), "优先级优先");
        assert_eq!(SchedulingAlgo::Spt.desc(), "最短加工时间");
        assert_eq!(SchedulingAlgo::Edd.desc(), "最早交货期");
    }

    #[test]
    fn test_module_loaded() {
        assert_eq!(P92_AUTO_MODULE, "scheduling_auto");
    }
}

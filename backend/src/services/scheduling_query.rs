//! P9-2 排程查询与甘特图子模块（占位）
//!
//! 拆分自原 `services/scheduling_service.rs`。
//!
//! ## 模块职责
//! - 排程甘特图数据生成
//! - 排程冲突检测与报告
//! - 排程历史查询

/// P9-2 标记：排程查询子模块路径
pub const P92_QRY_MODULE: &str = "scheduling_query";

/// 甘特图任务项（占位）
#[derive(Debug, Clone)]
pub struct GanttItem {
    /// 任务 ID
    pub task_id: i32,
    /// 任务名称
    pub name: String,
    /// 开始时间
    pub start: chrono::NaiveDate,
    /// 结束时间
    pub end: chrono::NaiveDate,
    /// 进度（0-100）
    pub progress: i32,
}

impl GanttItem {
    /// 计算持续天数
    pub fn duration_days(&self) -> i64 {
        (self.end - self.start).num_days()
    }

    /// 中文描述
    pub fn desc(&self) -> String {
        format!(
            "{}（{} → {}，{} 天，进度 {}%）",
            self.name,
            self.start,
            self.end,
            self.duration_days(),
            self.progress
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gantt_duration() {
        let item = GanttItem {
            task_id: 1,
            name: "工单 A".to_string(),
            start: crate::ymd!(2026, 6, 1),
            end: crate::ymd!(2026, 6, 8),
            progress: 50,
        };
        assert_eq!(item.duration_days(), 7);
        assert!(item.desc().contains("工单 A"));
    }

    #[test]
    fn test_module_loaded() {
        assert_eq!(P92_QRY_MODULE, "scheduling_query");
    }
}

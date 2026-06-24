//! 生产排程 Service 单元测试
//!
//! 适配 scheduling_service.rs 当前的 struct 定义（2026-06-24 重构后）

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    /// 测试排程明细创建
    #[test]
    fn test_schedule_detail_creation() {
        let detail = bingxi_backend::services::scheduling_service::ScheduleDetail {
            order_id: 1,
            order_no: Some("PO-001".to_string()),
            work_center_id: 1,
            work_center_name: Some("工作中心1".to_string()),
            planned_start: NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
            planned_end: NaiveDate::from_ymd_opt(2026, 5, 25).unwrap(),
            start_date: Some(NaiveDate::from_ymd_opt(2026, 5, 20).unwrap()),
            end_date: Some(NaiveDate::from_ymd_opt(2026, 5, 25).unwrap()),
            status: Some("SCHEDULED".to_string()),
        };

        assert_eq!(detail.order_id, 1);
        assert_eq!(detail.order_no.as_deref(), Some("PO-001"));
        assert_eq!(detail.status.as_deref(), Some("SCHEDULED"));
    }

    /// 测试甘特图数据项创建
    #[test]
    fn test_gantt_item_creation() {
        // 注：struct 实际命名为 GanttItemDto（DTO 后缀以与领域模型区分）
        let item = bingxi_backend::services::scheduling_service::GanttItemDto {
            id: "gantt-1".to_string(),
            order_id: 1,
            order_no: "PO-001".to_string(),
            product_id: 100,
            work_center_id: 1,
            work_center_name: "工作中心1".to_string(),
            start_date: NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 5, 25).unwrap(),
            duration_days: 5,
            progress: 0.0,
            status: "SCHEDULED".to_string(),
            priority: 1,
            dependencies: vec![],
        };

        assert_eq!(item.duration_days, 5);
        assert_eq!(item.progress, 0.0);
        assert_eq!(item.status, "SCHEDULED");
        assert_eq!(item.priority, 1);
        assert!(item.dependencies.is_empty());
    }

    /// 测试排程冲突创建
    #[test]
    fn test_schedule_conflict_creation() {
        let conflict = bingxi_backend::services::scheduling_service::ScheduleConflict {
            order_id: 1,
            order_no: Some("PO-001".to_string()),
            work_center_id: 1,
            work_center_name: Some("工作中心1".to_string()),
            conflict_type: "TIME_OVERLAP".to_string(),
            description: "时间重叠".to_string(),
            severity: Some("MEDIUM".to_string()),
            conflicting_order_id: Some(2),
            conflicting_order_no: Some("PO-002".to_string()),
        };

        assert_eq!(conflict.conflict_type, "TIME_OVERLAP");
        assert_eq!(conflict.severity.as_deref(), Some("MEDIUM"));
    }

    /// 测试自动排程请求创建
    #[test]
    fn test_auto_schedule_request_creation() {
        // 注：原 strategy 字段已重命名为 algo
        let request = bingxi_backend::services::scheduling_service::AutoScheduleRequest {
            start_date: NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 5, 25).unwrap(),
            work_center_ids: Some(vec![1, 2, 3]),
            algo: "priority".to_string(),
        };

        assert_eq!(request.work_center_ids.unwrap().len(), 3);
        assert_eq!(request.algo, "priority");
    }

    /// 测试日期范围使用 AutoScheduleRequest 替代已删除的 DateRange
    #[test]
    fn test_date_range_via_auto_request() {
        let request = bingxi_backend::services::scheduling_service::AutoScheduleRequest {
            start_date: NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 5, 25).unwrap(),
            work_center_ids: None,
            algo: "default".to_string(),
        };

        assert!(request.start_date < request.end_date);
    }
}

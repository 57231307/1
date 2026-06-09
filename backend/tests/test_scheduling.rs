//! 生产排程 Service 单元测试

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    #[test]
    fn test_schedule_detail_creation() {
        // 测试排程明细创建
        let detail = bingxi_backend::services::scheduling_service::ScheduleDetail {
            order_id: 1,
            order_no: "PO-001".to_string(),
            work_center_id: 1,
            work_center_name: "工作中心1".to_string(),
            start_date: NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
            end_date: NaiveDate::from_ymd_opt(2026, 5, 25).unwrap(),
            status: "SCHEDULED".to_string(),
        };

        assert_eq!(detail.order_id, 1);
        assert_eq!(detail.order_no, "PO-001");
        assert_eq!(detail.status, "SCHEDULED");
    }

    #[test]
    fn test_gantt_item_creation() {
        // 测试甘特图数据项创建
        let item = bingxi_backend::services::scheduling_service::GanttItem {
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
    }

    #[test]
    fn test_schedule_conflict_creation() {
        // 测试排程冲突创建
        let conflict = bingxi_backend::services::scheduling_service::ScheduleConflict {
            conflict_type: "TIME_OVERLAP".to_string(),
            order_id: 1,
            order_no: "PO-001".to_string(),
            conflicting_order_id: Some(2),
            conflicting_order_no: Some("PO-002".to_string()),
            work_center_id: Some(1),
            description: "时间重叠".to_string(),
            severity: "MEDIUM".to_string(),
        };

        assert_eq!(conflict.conflict_type, "TIME_OVERLAP");
        assert_eq!(conflict.severity, "MEDIUM");
    }

    #[test]
    fn test_auto_schedule_request_creation() {
        // 测试自动排程请求创建
        let request = bingxi_backend::services::scheduling_service::AutoScheduleRequest {
            work_center_ids: Some(vec![1, 2, 3]),
            start_date: Some(NaiveDate::from_ymd_opt(2026, 5, 20).unwrap()),
            strategy: Some("priority".to_string()),
        };

        assert_eq!(request.work_center_ids.unwrap().len(), 3);
        assert_eq!(request.strategy.unwrap(), "priority");
    }

    #[test]
    fn test_date_range_creation() {
        // 测试日期范围创建
        let range = bingxi_backend::services::scheduling_service::DateRange {
            start: NaiveDate::from_ymd_opt(2026, 5, 20).unwrap(),
            end: NaiveDate::from_ymd_opt(2026, 5, 25).unwrap(),
        };

        assert!(range.start < range.end);
    }
}

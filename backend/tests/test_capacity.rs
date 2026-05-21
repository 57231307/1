//! 产能分析 Service 单元测试

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    #[test]
    fn test_work_center_capacity_creation() {
        // 测试工作中心产能信息创建
        let wc = bingxi_backend::services::capacity_service::WorkCenterCapacity {
            id: 1,
            code: "WC-001".to_string(),
            name: "染色车间".to_string(),
            work_center_type: Some("STANDARD".to_string()),
            daily_capacity: Decimal::from(100),
            capacity_unit: Some("米".to_string()),
            status: "ACTIVE".to_string(),
            shifts: vec![],
        };

        assert_eq!(wc.id, 1);
        assert_eq!(wc.code, "WC-001");
        assert_eq!(wc.daily_capacity, Decimal::from(100));
    }

    #[test]
    fn test_shift_info_creation() {
        // 测试班次信息创建
        let shift = bingxi_backend::services::capacity_service::ShiftInfo {
            shift_name: "白班".to_string(),
            start_time: "08:00".to_string(),
            end_time: "17:00".to_string(),
            capacity_ratio: Decimal::from(100),
        };

        assert_eq!(shift.shift_name, "白班");
        assert_eq!(shift.capacity_ratio, Decimal::from(100));
    }

    #[test]
    fn test_capacity_load_item_creation() {
        // 测试产能负荷项创建
        let item = bingxi_backend::services::capacity_service::CapacityLoadItem {
            work_center_id: 1,
            work_center_code: "WC-001".to_string(),
            work_center_name: "染色车间".to_string(),
            daily_capacity: Decimal::from(100),
            capacity_unit: Some("米".to_string()),
            planned_quantity: Decimal::from(80),
            in_progress_quantity: Decimal::from(20),
            total_demand: Decimal::from(100),
            load_rate: Decimal::from(100),
            status: "NORMAL".to_string(),
        };

        assert_eq!(item.load_rate, Decimal::from(100));
        assert_eq!(item.status, "NORMAL");
    }

    #[test]
    fn test_capacity_overview_creation() {
        // 测试产能概览创建
        let overview = bingxi_backend::services::capacity_service::CapacityOverview {
            total_work_centers: 5,
            active_work_centers: 4,
            total_daily_capacity: Decimal::from(500),
            total_planned_demand: Decimal::from(400),
            overall_load_rate: Decimal::from(80),
            bottleneck_work_centers: vec![],
            overloaded_count: 1,
            idle_count: 0,
        };

        assert_eq!(overview.total_work_centers, 5);
        assert_eq!(overview.overall_load_rate, Decimal::from(80));
    }

    #[test]
    fn test_load_analysis_query_creation() {
        // 测试负荷分析查询参数创建
        let query = bingxi_backend::services::capacity_service::LoadAnalysisQuery {
            date_from: Some(NaiveDate::from_ymd_opt(2026, 5, 20).unwrap()),
            date_to: Some(NaiveDate::from_ymd_opt(2026, 5, 25).unwrap()),
            work_center_id: Some(1),
        };

        assert!(query.date_from.is_some());
        assert!(query.date_to.is_some());
        assert_eq!(query.work_center_id.unwrap(), 1);
    }
}

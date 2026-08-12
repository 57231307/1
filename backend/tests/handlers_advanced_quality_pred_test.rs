    use bingxi_backend::services::ai::quality_pred::*;
#[cfg(test)]
mod tests {

    /// 测试：请求体结构正确构造
    #[test]
    fn test_request_struct_construction() {
        let req = QualityPredRequest {
            product_id: Some(1),
            inspection_type: Some("成品".to_string()),
            window_days: Some(90),
            ..Default::default()
        };
        assert_eq!(req.product_id, Some(1));
        assert_eq!(req.inspection_type.as_deref(), Some("成品"));
        assert_eq!(req.window_days, Some(90));
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    /// 校验：请求体结构正确构造
    #[test]
    fn test_request_struct_construction() {
        let req = RecipeOptimizationRequest {
            color_no: "BL-301".to_string(),
            fabric_type: "棉".to_string(),
            dye_type: Some("活性染料".to_string()),
            color_name: Some("宝蓝".to_string()),
            k: Some(5),
        };
        assert!(!req.color_no.is_empty());
        assert!(!req.fabric_type.is_empty());
        assert_eq!(req.k, Some(5));
    }
}
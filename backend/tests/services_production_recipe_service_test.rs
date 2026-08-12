#[cfg(test)]
mod tests {
    use rust_decimal::prelude::FromPrimitive;
    use rust_decimal::Decimal;
    use bingxi_backend::services::production_recipe_service::*;

    /// 测试大货处方单号生成格式：PR-YYYYMMDDHHMMSS-NNN
    #[test]
    fn test_generate_recipe_no() {
        let no = ProductionRecipeService::generate_recipe_no();
        assert!(no.starts_with("PR-"));
        let parts: Vec<&str> = no.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 14); // YYYYMMDDHHMMSS
        assert_eq!(parts[2].len(), 3); // 3 位随机
    }

    /// 测试加料处方单号生成格式：PA-YYYYMMDDHHMMSS-NNN
    #[test]
    fn test_generate_addition_no() {
        let no = ProductionRecipeAdditionService::generate_addition_no();
        assert!(no.starts_with("PA-"));
        let parts: Vec<&str> = no.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 14);
        assert_eq!(parts[2].len(), 3);
    }

    /// 测试浴比解析
    #[test]
    fn test_parse_liquor_ratio() {
        // 标准 "1:8" 格式
        assert_eq!(
            ProductionRecipeService::parse_liquor_ratio("1:8").unwrap(),
            Decimal::from(8)
        );
        // 全角冒号
        assert_eq!(
            ProductionRecipeService::parse_liquor_ratio("1：10").unwrap(),
            Decimal::from(10)
        );
        // 斜杠格式
        assert_eq!(
            ProductionRecipeService::parse_liquor_ratio("1/12").unwrap(),
            Decimal::from(12)
        );
        // 带空格
        assert_eq!(
            ProductionRecipeService::parse_liquor_ratio(" 1:8 ").unwrap(),
            Decimal::from(8)
        );
        // 非法格式
        assert!(ProductionRecipeService::parse_liquor_ratio("").is_err());
        assert!(ProductionRecipeService::parse_liquor_ratio("abc").is_err());
        assert!(ProductionRecipeService::parse_liquor_ratio("1:").is_err());
        assert!(ProductionRecipeService::parse_liquor_ratio("1:0").is_err());
        assert!(ProductionRecipeService::parse_liquor_ratio("1:-5").is_err());
    }

    /// 测试用量计算（真实业务公式：用量 = 浓度% × 布重 × 浴比 / 100 × 加成系数）
    #[test]
    fn test_calculate_amounts() {
        let fabric_weight = Decimal::from(100); // 100 kg
        let liquor_ratio = "1:8".to_string(); // 浴比 8
        let items = vec![RecipeMaterialItem {
            material_code: "D001".to_string(),
            material_name: "活性红".to_string(),
            concentration: Some(Decimal::from(2)), // 2% owf
            unit: "kg".to_string(),
            amount: Decimal::ZERO, // 待计算
            category: "dye".to_string(),
        }];

        let req = CalculateAmountsRequest {
            fabric_weight,
            liquor_ratio,
            adjustment_factor: None,
            items,
        };
        let result = ProductionRecipeService::calculate_amounts(req).unwrap();
        // 用量 = 2 × 100 × 8 / 100 × 1 = 16 kg
        assert_eq!(result[0].amount, Decimal::from(16));
    }

    /// 测试用量计算（带加成系数）
    #[test]
    fn test_calculate_amounts_with_factor() {
        let fabric_weight = Decimal::from(200); // 200 kg
        let liquor_ratio = "1:10".to_string(); // 浴比 10
        let items = vec![RecipeMaterialItem {
            material_code: "D002".to_string(),
            material_name: "分散蓝".to_string(),
            concentration: Some(Decimal::from(3)), // 3% owf
            unit: "kg".to_string(),
            amount: Decimal::ZERO,
            category: "dye".to_string(),
        }];

        let req = CalculateAmountsRequest {
            fabric_weight,
            liquor_ratio,
            adjustment_factor: Some(Decimal::from(150) / Decimal::from(100)), // 1.50 加成
            items,
        };
        let result = ProductionRecipeService::calculate_amounts(req).unwrap();
        // 用量 = 3 × 200 × 10 / 100 × 1.5 = 90 kg
        assert_eq!(result[0].amount, Decimal::from(90));
    }

    /// 测试用量计算（助剂无浓度，保留原用量）
    #[test]
    fn test_calculate_amounts_auxiliary_no_concentration() {
        let fabric_weight = Decimal::from(100);
        let liquor_ratio = "1:8".to_string();
        let original_amount = Decimal::from(5);
        let items = vec![RecipeMaterialItem {
            material_code: "A001".to_string(),
            material_name: "匀染剂".to_string(),
            concentration: None, // 助剂无浓度
            unit: "kg".to_string(),
            amount: original_amount,
            category: "auxiliary".to_string(),
        }];

        let req = CalculateAmountsRequest {
            fabric_weight,
            liquor_ratio,
            adjustment_factor: None,
            items,
        };
        let result = ProductionRecipeService::calculate_amounts(req).unwrap();
        // 无浓度不重算，保留原用量
        assert_eq!(result[0].amount, original_amount);
    }

    /// 测试用量计算非法输入
    #[test]
    fn test_calculate_amounts_invalid() {
        // 备布重量 <= 0
        let req = CalculateAmountsRequest {
            fabric_weight: Decimal::ZERO,
            liquor_ratio: "1:8".to_string(),
            adjustment_factor: None,
            items: vec![],
        };
        assert!(ProductionRecipeService::calculate_amounts(req).is_err());

        // 浴比格式错误
        let req = CalculateAmountsRequest {
            fabric_weight: Decimal::from(100),
            liquor_ratio: "abc".to_string(),
            adjustment_factor: None,
            items: vec![],
        };
        assert!(ProductionRecipeService::calculate_amounts(req).is_err());

        // 加成系数 <= 0
        let req = CalculateAmountsRequest {
            fabric_weight: Decimal::from(100),
            liquor_ratio: "1:8".to_string(),
            adjustment_factor: Some(Decimal::ZERO),
            items: vec![],
        };
        assert!(ProductionRecipeService::calculate_amounts(req).is_err());

        // 浓度为负
        let req = CalculateAmountsRequest {
            fabric_weight: Decimal::from(100),
            liquor_ratio: "1:8".to_string(),
            adjustment_factor: None,
            items: vec![RecipeMaterialItem {
                material_code: "D001".to_string(),
                material_name: "活性红".to_string(),
                concentration: Some(Decimal::from(-1)),
                unit: "kg".to_string(),
                amount: Decimal::ZERO,
                category: "dye".to_string(),
            }],
        };
        assert!(ProductionRecipeService::calculate_amounts(req).is_err());
    }

    /// 测试大货处方状态流转合法性
    #[test]
    fn test_recipe_status_transition_valid() {
        // 合法流转
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::DRAFT,
            recipe_status::APPROVED
        )
        .is_ok());
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::DRAFT,
            recipe_status::CANCELLED
        )
        .is_ok());
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::APPROVED,
            recipe_status::CLOSED
        )
        .is_ok());
    }

    /// 测试大货处方状态流转非法
    #[test]
    fn test_recipe_status_transition_invalid() {
        // 非法流转
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::DRAFT,
            recipe_status::CLOSED
        )
        .is_err());
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::APPROVED,
            recipe_status::DRAFT
        )
        .is_err());
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::APPROVED,
            recipe_status::CANCELLED
        )
        .is_err());
        // 终态不可流转
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::CLOSED,
            recipe_status::APPROVED
        )
        .is_err());
        assert!(ProductionRecipeService::validate_status_transition(
            recipe_status::CANCELLED,
            recipe_status::DRAFT
        )
        .is_err());
    }

    /// 测试大货处方更新/删除状态校验
    #[test]
    fn test_recipe_validate_can_update_and_delete() {
        // 仅 draft 可更新
        assert!(ProductionRecipeService::validate_can_update(recipe_status::DRAFT).is_ok());
        assert!(ProductionRecipeService::validate_can_update(recipe_status::APPROVED).is_err());
        assert!(ProductionRecipeService::validate_can_update(recipe_status::CLOSED).is_err());
        assert!(ProductionRecipeService::validate_can_update(recipe_status::CANCELLED).is_err());

        // 仅 draft 可删除
        assert!(ProductionRecipeService::validate_can_delete(recipe_status::DRAFT).is_ok());
        assert!(ProductionRecipeService::validate_can_delete(recipe_status::APPROVED).is_err());
        assert!(ProductionRecipeService::validate_can_delete(recipe_status::CLOSED).is_err());
    }

    /// 测试加料处方状态流转
    #[test]
    fn test_addition_status_transition() {
        // 合法流转
        assert!(ProductionRecipeAdditionService::validate_status_transition(
            addition_status::DRAFT,
            addition_status::APPROVED
        )
        .is_ok());
        assert!(ProductionRecipeAdditionService::validate_status_transition(
            addition_status::APPROVED,
            addition_status::CLOSED
        )
        .is_ok());

        // 非法流转
        assert!(ProductionRecipeAdditionService::validate_status_transition(
            addition_status::DRAFT,
            addition_status::CLOSED
        )
        .is_err());
        assert!(ProductionRecipeAdditionService::validate_status_transition(
            addition_status::APPROVED,
            addition_status::DRAFT
        )
        .is_err());
        // 终态
        assert!(ProductionRecipeAdditionService::validate_status_transition(
            addition_status::CLOSED,
            addition_status::APPROVED
        )
        .is_err());
    }

    /// 测试 FromPrimitive trait 可用（确保 rust_decimal::prelude::FromPrimitive 引入正确）
    #[test]
    fn test_decimal_from_f64() {
        let d = Decimal::from_f64(1.5).unwrap();
        assert_eq!(d, Decimal::from(15) / Decimal::from(10));
    }
}
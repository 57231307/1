    use bingxi_backend::services::business_mode_service::*;
#[cfg(test)]
mod tests {

    #[test]
    fn 测试校验业务模式代码_合法() {
        assert!(validate_mode_code("grey_trading").is_ok());
        assert!(validate_mode_code("finished_trading").is_ok());
        assert!(validate_mode_code("dyeing_processing").is_ok());
        assert!(validate_mode_code("self_weave_dye").is_ok());
        assert!(validate_mode_code("outsourcing").is_ok());
        assert!(validate_mode_code("toll_processing").is_ok());
    }

    #[test]
    fn 测试校验业务模式代码_非法() {
        assert!(validate_mode_code("invalid").is_err());
        assert!(validate_mode_code("").is_err());
        assert!(validate_mode_code("trading").is_err());
    }

    #[test]
    fn 测试校验物料来源_合法() {
        assert!(validate_material_source("purchase").is_ok());
        assert!(validate_material_source("customer_provided").is_ok());
        assert!(validate_material_source("self_made").is_ok());
        assert!(validate_material_source("toll").is_ok());
    }

    #[test]
    fn 测试校验物料来源_非法() {
        assert!(validate_material_source("invalid").is_err());
        assert!(validate_material_source("").is_err());
    }

    #[test]
    fn 测试校验结算方式_合法() {
        assert!(validate_settlement_method("sale_settlement").is_ok());
        assert!(validate_settlement_method("processing_fee_settlement").is_ok());
    }

    #[test]
    fn 测试校验结算方式_非法() {
        assert!(validate_settlement_method("invalid").is_err());
        assert!(validate_settlement_method("").is_err());
    }

    #[test]
    fn 测试校验库存类型_合法() {
        assert!(validate_inventory_type("grey").is_ok());
        assert!(validate_inventory_type("finished").is_ok());
        assert!(validate_inventory_type("both").is_ok());
        assert!(validate_inventory_type("none").is_ok());
    }

    #[test]
    fn 测试校验库存类型_非法() {
        assert!(validate_inventory_type("invalid").is_err());
        assert!(validate_inventory_type("").is_err());
    }

    #[test]
    fn 测试校验成本核算方法_合法() {
        assert!(validate_cost_method("standard").is_ok());
        assert!(validate_cost_method("actual").is_ok());
        assert!(validate_cost_method("processing_fee").is_ok());
    }

    #[test]
    fn 测试校验成本核算方法_非法() {
        assert!(validate_cost_method("invalid").is_err());
        assert!(validate_cost_method("").is_err());
    }

    #[test]
    fn 测试校验规则类型_合法() {
        assert!(validate_rule_type("required").is_ok());
        assert!(validate_rule_type("optional").is_ok());
        assert!(validate_rule_type("forbidden").is_ok());
    }

    #[test]
    fn 测试校验规则类型_非法() {
        assert!(validate_rule_type("invalid").is_err());
        assert!(validate_rule_type("").is_err());
    }

    #[test]
    fn 测试校验模式分类_合法() {
        assert!(validate_mode_category("trading").is_ok());
        assert!(validate_mode_category("processing").is_ok());
        assert!(validate_mode_category("integrated").is_ok());
    }

    #[test]
    fn 测试校验模式分类_非法() {
        assert!(validate_mode_category("invalid").is_err());
        assert!(validate_mode_category("").is_err());
    }

    #[test]
    fn 测试校验单据类型_合法() {
        assert!(validate_document_type("sales_order").is_ok());
        assert!(validate_document_type("purchase_order").is_ok());
        assert!(validate_document_type("production_order").is_ok());
        assert!(validate_document_type("outsourcing_order").is_ok());
    }

    #[test]
    fn 测试校验单据类型_非法() {
        assert!(validate_document_type("invalid").is_err());
        assert!(validate_document_type("").is_err());
    }

    #[test]
    fn 测试一致性校验_坯布经销模式_合法() {
        // grey_trading: require_purchase=true, require_sales=true, require_production=false, require_outsourcing=false
        // material_source=purchase, settlement_method=sale_settlement
        assert!(check_module_consistency(
            "grey_trading",
            true,
            false,
            false,
            true,
            "purchase",
            "sale_settlement"
        )
        .is_ok());
    }

    #[test]
    fn 测试一致性校验_坯布经销模式_缺少采购模块() {
        // grey_trading 必须 require_purchase=true
        assert!(check_module_consistency(
            "grey_trading",
            false,
            false,
            false,
            true,
            "purchase",
            "sale_settlement"
        )
        .is_err());
    }

    #[test]
    fn 测试一致性校验_染整加工模式_合法() {
        // dyeing_processing: require_production=true, require_sales=false, require_purchase=false, require_outsourcing=false
        // material_source=customer_provided, settlement_method=processing_fee_settlement
        assert!(check_module_consistency(
            "dyeing_processing",
            false,
            true,
            false,
            false,
            "customer_provided",
            "processing_fee_settlement"
        )
        .is_ok());
    }

    #[test]
    fn 测试一致性校验_染整加工模式_误开销售模块() {
        // dyeing_processing 必须 require_sales=false
        assert!(check_module_consistency(
            "dyeing_processing",
            false,
            true,
            false,
            true,
            "customer_provided",
            "processing_fee_settlement"
        )
        .is_err());
    }

    #[test]
    fn 测试一致性校验_染整加工模式_物料来源错误() {
        // dyeing_processing 物料来源必须是 customer_provided
        assert!(check_module_consistency(
            "dyeing_processing",
            false,
            true,
            false,
            false,
            "purchase",
            "processing_fee_settlement"
        )
        .is_err());
    }

    #[test]
    fn 测试一致性校验_委托加工模式_合法() {
        // outsourcing: require_production=true, require_outsourcing=true, require_sales=true, require_purchase=false
        // material_source=self_made, settlement_method=sale_settlement
        assert!(check_module_consistency(
            "outsourcing",
            false,
            true,
            true,
            true,
            "self_made",
            "sale_settlement"
        )
        .is_ok());
    }

    #[test]
    fn 测试一致性校验_委托加工模式_缺少委外模块() {
        // outsourcing 必须 require_outsourcing=true
        assert!(check_module_consistency(
            "outsourcing",
            false,
            true,
            false,
            true,
            "self_made",
            "sale_settlement"
        )
        .is_err());
    }

    #[test]
    fn 测试一致性校验_来料加工模式_合法() {
        // toll_processing: require_production=true, require_sales=false, require_purchase=false, require_outsourcing=false
        // material_source=toll, settlement_method=processing_fee_settlement
        assert!(check_module_consistency(
            "toll_processing",
            false,
            true,
            false,
            false,
            "toll",
            "processing_fee_settlement"
        )
        .is_ok());
    }

    #[test]
    fn 测试一致性校验_来料加工模式_误开采购模块() {
        // toll_processing 必须 require_purchase=false
        assert!(check_module_consistency(
            "toll_processing",
            true,
            true,
            false,
            false,
            "toll",
            "processing_fee_settlement"
        )
        .is_err());
    }

    #[test]
    fn 测试一致性校验_成品经销模式_合法() {
        // finished_trading: require_purchase=true, require_production=true, require_sales=true, require_outsourcing=false
        // material_source=purchase, settlement_method=sale_settlement
        assert!(check_module_consistency(
            "finished_trading",
            true,
            true,
            false,
            true,
            "purchase",
            "sale_settlement"
        )
        .is_ok());
    }

    #[test]
    fn 测试一致性校验_自织自染模式_合法() {
        // self_weave_dye: require_purchase=true, require_production=true, require_sales=true, require_outsourcing=false
        // material_source=purchase, settlement_method=sale_settlement
        assert!(check_module_consistency(
            "self_weave_dye",
            true,
            true,
            false,
            true,
            "purchase",
            "sale_settlement"
        )
        .is_ok());
    }

    #[test]
    fn 测试一致性校验_未知模式代码() {
        // 未知模式代码应报错
        assert!(check_module_consistency(
            "unknown_mode",
            true,
            false,
            false,
            true,
            "purchase",
            "sale_settlement"
        )
        .is_err());
    }
}
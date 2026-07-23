//! 多业务模式支持 Service（facade）
//!
//! v14 批次 431：多业务模式支持
//! 依据：面料行业真实业务调研文档 §6 业务模式 6 种
//!
//! 核心能力：
//! - 业务模式配置 CRUD + 按代码查询 + 默认模式管理 + 模式详情查询（含流程节点+规则）
//! - 业务模式流程节点 CRUD + 按模式查询（按 step_no 排序）
//! - 业务模式规则 CRUD + 按模式查询
//! - 单据-业务模式关联 CRUD + 按单据查询 + 关联单据（含模式快照）
//!
//! 6 种业务模式：
//! - grey_trading 坯布经销：采购坯布 → 库存 → 销售坯布
//! - finished_trading 成品经销：采购坯布 → 染整加工 → 销售成品
//! - dyeing_processing 染整加工：客供坯布 → 染整加工 → 收取加工费
//! - self_weave_dye 自织自染：采购原料 → 纺纱织布染整 → 销售成品
//! - outsourcing 委托加工：自制半成品 → 委外加工 → 收回成品 → 销售
//! - toll_processing 来料加工：客户来料 → 加工 → 收取加工费
//!
//! 批次 489 D10-2b 拆分：本文件作为 facade，保留校验纯函数 + Service struct + new 构造函数 + 测试。
//! 4 个 Service 的 impl 块迁移至 `business_mode_ops` 子模块（config / flow_step / rule / order_link）。
//! DTO struct 迁移至 `business_mode_ops::types`，本 facade 通过 `pub use` 二次 re-export 保持外部引用路径不变。

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::models::status::business_material_source;
use crate::models::status::business_mode_code;
use crate::models::status::business_rule_type;
use crate::models::status::business_settlement_method;
use crate::utils::error::AppError;

// re-export DTOs 与 ops 子模块，保持外部 `use crate::services::business_mode_service::{...}` 路径不变
pub use crate::services::business_mode_ops::{
    BusinessModeConfigQuery, BusinessModeOrderLinkQuery, CreateBusinessModeConfigRequest,
    CreateBusinessModeFlowStepRequest, CreateBusinessModeOrderLinkRequest,
    CreateBusinessModeRuleRequest, UpdateBusinessModeConfigRequest,
    UpdateBusinessModeFlowStepRequest, UpdateBusinessModeOrderLinkRequest,
    UpdateBusinessModeRuleRequest,
};

// ============================================================================
// 业务模式校验纯函数
// ============================================================================

/// 校验业务模式代码是否合法
pub fn validate_mode_code(mode_code: &str) -> Result<(), AppError> {
    let valid = [
        business_mode_code::GREY_TRADING,
        business_mode_code::FINISHED_TRADING,
        business_mode_code::DYEING_PROCESSING,
        business_mode_code::SELF_WEAVE_DYE,
        business_mode_code::OUTSOURCING,
        business_mode_code::TOLL_PROCESSING,
    ];
    if !valid.contains(&mode_code) {
        return Err(AppError::business(format!(
            "业务模式代码必须是 grey_trading / finished_trading / dyeing_processing / self_weave_dye / outsourcing / toll_processing，当前: {}",
            mode_code
        )));
    }
    Ok(())
}

/// 校验物料来源是否合法
pub fn validate_material_source(source: &str) -> Result<(), AppError> {
    let valid = [
        business_material_source::PURCHASE,
        business_material_source::CUSTOMER_PROVIDED,
        business_material_source::SELF_MADE,
        business_material_source::TOLL,
    ];
    if !valid.contains(&source) {
        return Err(AppError::business(format!(
            "物料来源必须是 purchase / customer_provided / self_made / toll，当前: {}",
            source
        )));
    }
    Ok(())
}

/// 校验结算方式是否合法
pub fn validate_settlement_method(method: &str) -> Result<(), AppError> {
    let valid = [
        business_settlement_method::SALE_SETTLEMENT,
        business_settlement_method::PROCESSING_FEE_SETTLEMENT,
    ];
    if !valid.contains(&method) {
        return Err(AppError::business(format!(
            "结算方式必须是 sale_settlement / processing_fee_settlement，当前: {}",
            method
        )));
    }
    Ok(())
}

/// 校验库存类型是否合法
pub fn validate_inventory_type(inv_type: &str) -> Result<(), AppError> {
    let valid = ["grey", "finished", "both", "none"];
    if !valid.contains(&inv_type) {
        return Err(AppError::business(format!(
            "库存类型必须是 grey / finished / both / none，当前: {}",
            inv_type
        )));
    }
    Ok(())
}

/// 校验成本核算方法是否合法
pub fn validate_cost_method(method: &str) -> Result<(), AppError> {
    let valid = ["standard", "actual", "processing_fee"];
    if !valid.contains(&method) {
        return Err(AppError::business(format!(
            "成本核算方法必须是 standard / actual / processing_fee，当前: {}",
            method
        )));
    }
    Ok(())
}

/// 校验规则类型是否合法
pub fn validate_rule_type(rule_type: &str) -> Result<(), AppError> {
    let valid = [
        business_rule_type::REQUIRED,
        business_rule_type::OPTIONAL,
        business_rule_type::FORBIDDEN,
    ];
    if !valid.contains(&rule_type) {
        return Err(AppError::business(format!(
            "规则类型必须是 required / optional / forbidden，当前: {}",
            rule_type
        )));
    }
    Ok(())
}

/// 校验模式分类是否合法
pub fn validate_mode_category(category: &str) -> Result<(), AppError> {
    let valid = ["trading", "processing", "integrated"];
    if !valid.contains(&category) {
        return Err(AppError::business(format!(
            "模式分类必须是 trading / processing / integrated，当前: {}",
            category
        )));
    }
    Ok(())
}

/// 校验单据类型是否合法
pub fn validate_document_type(doc_type: &str) -> Result<(), AppError> {
    let valid = [
        "sales_order",
        "purchase_order",
        "production_order",
        "outsourcing_order",
    ];
    if !valid.contains(&doc_type) {
        return Err(AppError::business(format!(
            "单据类型必须是 sales_order / purchase_order / production_order / outsourcing_order，当前: {}",
            doc_type
        )));
    }
    Ok(())
}

/// 校验业务模式配置一致性
///
/// 业务规则（依据 §6 业务模式 6 种）：
/// - grey_trading：require_purchase=true, require_sales=true, require_production=false, require_outsourcing=false, material_source=purchase, settlement_method=sale_settlement
/// - finished_trading：require_purchase=true, require_production=true, require_sales=true, require_outsourcing=false, material_source=purchase, settlement_method=sale_settlement
/// - dyeing_processing：require_production=true, require_sales=false, require_purchase=false, require_outsourcing=false, material_source=customer_provided, settlement_method=processing_fee_settlement
/// - self_weave_dye：require_purchase=true, require_production=true, require_sales=true, require_outsourcing=false, material_source=purchase, settlement_method=sale_settlement
/// - outsourcing：require_production=true, require_outsourcing=true, require_sales=true, require_purchase=false, material_source=self_made, settlement_method=sale_settlement
/// - toll_processing：require_production=true, require_sales=false, require_purchase=false, require_outsourcing=false, material_source=toll, settlement_method=processing_fee_settlement
pub fn check_module_consistency(
    mode_code: &str,
    require_purchase: bool,
    require_production: bool,
    require_outsourcing: bool,
    require_sales: bool,
    material_source: &str,
    settlement_method: &str,
) -> Result<(), AppError> {
    validate_mode_code(mode_code)?;
    validate_material_source(material_source)?;
    validate_settlement_method(settlement_method)?;

    // P0-D12（Batch 488）：抽取 match 各 arm 校验为独立函数，主函数仅做调度
    // 圈复杂度从 ~35（6 arms × 6 if）降至 7（单一 match 调度）
    let args = ModeConsistencyArgs {
        require_purchase,
        require_production,
        require_outsourcing,
        require_sales,
        material_source,
        settlement_method,
    };

    match mode_code {
        business_mode_code::GREY_TRADING => validate_grey_trading(&args),
        business_mode_code::FINISHED_TRADING => validate_finished_trading(&args),
        business_mode_code::DYEING_PROCESSING => validate_dyeing_processing(&args),
        business_mode_code::SELF_WEAVE_DYE => validate_self_weave_dye(&args),
        business_mode_code::OUTSOURCING => validate_outsourcing(&args),
        business_mode_code::TOLL_PROCESSING => validate_toll_processing(&args),
        _ => Err(AppError::business(format!(
            "未知的业务模式代码: {}",
            mode_code
        ))),
    }
}

/// P0-D12：业务模式一致性校验参数聚合体
///
/// 避免 6 个 validate_xxx 函数重复传递 6 个参数，降低签名复杂度
struct ModeConsistencyArgs<'a> {
    require_purchase: bool,
    require_production: bool,
    require_outsourcing: bool,
    require_sales: bool,
    material_source: &'a str,
    settlement_method: &'a str,
}

/// P0-D12：坯布经销模式校验
fn validate_grey_trading(args: &ModeConsistencyArgs) -> Result<(), AppError> {
    if !args.require_purchase {
        return Err(AppError::business("坯布经销模式必须 require_purchase=true"));
    }
    if !args.require_sales {
        return Err(AppError::business("坯布经销模式必须 require_sales=true"));
    }
    if args.require_production {
        return Err(AppError::business("坯布经销模式必须 require_production=false"));
    }
    if args.require_outsourcing {
        return Err(AppError::business("坯布经销模式必须 require_outsourcing=false"));
    }
    if args.material_source != business_material_source::PURCHASE {
        return Err(AppError::business(
            "坯布经销模式物料来源必须是 purchase 采购",
        ));
    }
    if args.settlement_method != business_settlement_method::SALE_SETTLEMENT {
        return Err(AppError::business(
            "坯布经销模式结算方式必须是 sale_settlement 销售结算",
        ));
    }
    Ok(())
}

/// P0-D12：成品经销模式校验
fn validate_finished_trading(args: &ModeConsistencyArgs) -> Result<(), AppError> {
    if !args.require_purchase {
        return Err(AppError::business("成品经销模式必须 require_purchase=true"));
    }
    if !args.require_production {
        return Err(AppError::business("成品经销模式必须 require_production=true"));
    }
    if !args.require_sales {
        return Err(AppError::business("成品经销模式必须 require_sales=true"));
    }
    if args.require_outsourcing {
        return Err(AppError::business("成品经销模式必须 require_outsourcing=false"));
    }
    if args.material_source != business_material_source::PURCHASE {
        return Err(AppError::business(
            "成品经销模式物料来源必须是 purchase 采购",
        ));
    }
    if args.settlement_method != business_settlement_method::SALE_SETTLEMENT {
        return Err(AppError::business(
            "成品经销模式结算方式必须是 sale_settlement 销售结算",
        ));
    }
    Ok(())
}

/// P0-D12：染整加工模式校验
fn validate_dyeing_processing(args: &ModeConsistencyArgs) -> Result<(), AppError> {
    if args.require_purchase {
        return Err(AppError::business("染整加工模式必须 require_purchase=false"));
    }
    if !args.require_production {
        return Err(AppError::business("染整加工模式必须 require_production=true"));
    }
    if args.require_outsourcing {
        return Err(AppError::business("染整加工模式必须 require_outsourcing=false"));
    }
    if args.require_sales {
        return Err(AppError::business("染整加工模式必须 require_sales=false"));
    }
    if args.material_source != business_material_source::CUSTOMER_PROVIDED {
        return Err(AppError::business(
            "染整加工模式物料来源必须是 customer_provided 客供",
        ));
    }
    if args.settlement_method != business_settlement_method::PROCESSING_FEE_SETTLEMENT {
        return Err(AppError::business(
            "染整加工模式结算方式必须是 processing_fee_settlement 加工费结算",
        ));
    }
    Ok(())
}

/// P0-D12：自织自染模式校验
fn validate_self_weave_dye(args: &ModeConsistencyArgs) -> Result<(), AppError> {
    if !args.require_purchase {
        return Err(AppError::business("自织自染模式必须 require_purchase=true"));
    }
    if !args.require_production {
        return Err(AppError::business("自织自染模式必须 require_production=true"));
    }
    if !args.require_sales {
        return Err(AppError::business("自织自染模式必须 require_sales=true"));
    }
    if args.require_outsourcing {
        return Err(AppError::business("自织自染模式必须 require_outsourcing=false"));
    }
    if args.material_source != business_material_source::PURCHASE {
        return Err(AppError::business(
            "自织自染模式物料来源必须是 purchase 采购",
        ));
    }
    if args.settlement_method != business_settlement_method::SALE_SETTLEMENT {
        return Err(AppError::business(
            "自织自染模式结算方式必须是 sale_settlement 销售结算",
        ));
    }
    Ok(())
}

/// P0-D12：委托加工模式校验
fn validate_outsourcing(args: &ModeConsistencyArgs) -> Result<(), AppError> {
    if args.require_purchase {
        return Err(AppError::business("委托加工模式必须 require_purchase=false"));
    }
    if !args.require_production {
        return Err(AppError::business("委托加工模式必须 require_production=true"));
    }
    if !args.require_outsourcing {
        return Err(AppError::business("委托加工模式必须 require_outsourcing=true"));
    }
    if !args.require_sales {
        return Err(AppError::business("委托加工模式必须 require_sales=true"));
    }
    if args.material_source != business_material_source::SELF_MADE {
        return Err(AppError::business(
            "委托加工模式物料来源必须是 self_made 自制",
        ));
    }
    if args.settlement_method != business_settlement_method::SALE_SETTLEMENT {
        return Err(AppError::business(
            "委托加工模式结算方式必须是 sale_settlement 销售结算",
        ));
    }
    Ok(())
}

/// P0-D12：来料加工模式校验
fn validate_toll_processing(args: &ModeConsistencyArgs) -> Result<(), AppError> {
    if args.require_purchase {
        return Err(AppError::business("来料加工模式必须 require_purchase=false"));
    }
    if !args.require_production {
        return Err(AppError::business("来料加工模式必须 require_production=true"));
    }
    if args.require_outsourcing {
        return Err(AppError::business("来料加工模式必须 require_outsourcing=false"));
    }
    if args.require_sales {
        return Err(AppError::business("来料加工模式必须 require_sales=false"));
    }
    if args.material_source != business_material_source::TOLL {
        return Err(AppError::business("来料加工模式物料来源必须是 toll 来料"));
    }
    if args.settlement_method != business_settlement_method::PROCESSING_FEE_SETTLEMENT {
        return Err(AppError::business(
            "来料加工模式结算方式必须是 processing_fee_settlement 加工费结算",
        ));
    }
    Ok(())
}

// ============================================================================
// 业务模式 Service struct 定义（facade）
// ============================================================================
//
// 4 个 Service struct 与 `new` 构造函数保留在本 facade 中。
// impl 块迁移至 `business_mode_ops` 子模块，Rust 允许同一 crate 多文件多 impl 块。
// `db` 字段使用 `pub(crate)` 可见性，供 ops 子模块的 impl 块访问。

/// 业务模式配置 Service
pub struct BusinessModeConfigService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl BusinessModeConfigService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

/// 业务模式流程节点 Service
pub struct BusinessModeFlowStepService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl BusinessModeFlowStepService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

/// 业务模式规则 Service
pub struct BusinessModeRuleService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl BusinessModeRuleService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

/// 单据-业务模式关联 Service
pub struct BusinessModeOrderLinkService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl BusinessModeOrderLinkService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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

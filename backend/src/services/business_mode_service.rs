//! 多业务模式支持 Service
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

use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::models::business_mode_config::{
    self, ActiveModel as ConfigActiveModel, Entity as ConfigEntity, Model as ConfigModel,
};
use crate::models::business_mode_flow_step::{
    self, ActiveModel as FlowStepActiveModel, Entity as FlowStepEntity, Model as FlowStepModel,
};
use crate::models::business_mode_order_link::{
    self, ActiveModel as OrderLinkActiveModel, Entity as OrderLinkEntity, Model as OrderLinkModel,
};
use crate::models::business_mode_rule::{
    self, ActiveModel as RuleActiveModel, Entity as RuleEntity, Model as RuleModel,
};
use crate::models::status::business_material_source;
use crate::models::status::business_mode_code;
use crate::models::status::business_rule_type;
use crate::models::status::business_settlement_method;
use crate::utils::error::AppError;

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
    if !args.require_production {
        return Err(AppError::business("染整加工模式必须 require_production=true"));
    }
    if args.require_purchase {
        return Err(AppError::business("染整加工模式必须 require_purchase=false"));
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
    if !args.require_production {
        return Err(AppError::business("委托加工模式必须 require_production=true"));
    }
    if !args.require_outsourcing {
        return Err(AppError::business("委托加工模式必须 require_outsourcing=true"));
    }
    if !args.require_sales {
        return Err(AppError::business("委托加工模式必须 require_sales=true"));
    }
    if args.require_purchase {
        return Err(AppError::business("委托加工模式必须 require_purchase=false"));
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
    if !args.require_production {
        return Err(AppError::business("来料加工模式必须 require_production=true"));
    }
    if args.require_purchase {
        return Err(AppError::business("来料加工模式必须 require_purchase=false"));
    }
    if args.require_outsourcing {
        return Err(AppError::business("来料加工模式必须 require_outsourcing=false"));
    }
    if args.require_sales {
        return Err(AppError::business("来料加工模式必须 require_sales=false"));
    }
    if args.material_source != business_material_source::TOLL {
        return Err(AppError::business(
            "来料加工模式物料来源必须是 toll 来料",
        ));
    }
    if args.settlement_method != business_settlement_method::PROCESSING_FEE_SETTLEMENT {
        return Err(AppError::business(
            "来料加工模式结算方式必须是 processing_fee_settlement 加工费结算",
        ));
    }
    Ok(())
}

// ============================================================================
// 业务模式配置 Service
// ============================================================================

/// 创建业务模式配置请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateBusinessModeConfigRequest {
    pub mode_code: String,
    pub mode_name: String,
    pub description: Option<String>,
    pub is_active: Option<bool>,
    pub is_default: Option<bool>,
    pub process_chain: Option<serde_json::Value>,
    pub material_source: String,
    pub settlement_method: String,
    pub inventory_type: String,
    pub cost_method: String,
    pub require_purchase: Option<bool>,
    pub require_production: Option<bool>,
    pub require_outsourcing: Option<bool>,
    pub require_sales: Option<bool>,
    pub mode_category: String,
    pub remarks: Option<String>,
    pub created_by: Option<i32>,
}

/// 更新业务模式配置请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateBusinessModeConfigRequest {
    pub mode_name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
    pub is_default: Option<bool>,
    pub process_chain: Option<serde_json::Value>,
    pub material_source: Option<String>,
    pub settlement_method: Option<String>,
    pub inventory_type: Option<String>,
    pub cost_method: Option<String>,
    pub require_purchase: Option<bool>,
    pub require_production: Option<bool>,
    pub require_outsourcing: Option<bool>,
    pub require_sales: Option<bool>,
    pub mode_category: Option<String>,
    pub remarks: Option<String>,
}

/// 业务模式配置查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct BusinessModeConfigQuery {
    pub mode_code: Option<String>,
    pub mode_category: Option<String>,
    pub is_active: Option<bool>,
    pub material_source: Option<String>,
    pub settlement_method: Option<String>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 业务模式配置 Service
pub struct BusinessModeConfigService {
    db: Arc<DatabaseConnection>,
}

impl BusinessModeConfigService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建业务模式配置
    pub async fn create(
        &self,
        req: CreateBusinessModeConfigRequest,
    ) -> Result<ConfigModel, AppError> {
        validate_mode_code(&req.mode_code)?;
        validate_material_source(&req.material_source)?;
        validate_settlement_method(&req.settlement_method)?;
        validate_inventory_type(&req.inventory_type)?;
        validate_cost_method(&req.cost_method)?;
        validate_mode_category(&req.mode_category)?;

        // 校验业务模式配置一致性
        check_module_consistency(
            &req.mode_code,
            req.require_purchase.unwrap_or(false),
            req.require_production.unwrap_or(false),
            req.require_outsourcing.unwrap_or(false),
            req.require_sales.unwrap_or(false),
            &req.material_source,
            &req.settlement_method,
        )?;

        // 校验模式代码唯一性
        if let Some(_existing) = ConfigEntity::find()
            .filter(business_mode_config::Column::ModeCode.eq(&req.mode_code))
            .filter(business_mode_config::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "业务模式代码 {} 已存在",
                req.mode_code
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let process_chain = req
            .process_chain
            .unwrap_or_else(|| serde_json::Value::Array(vec![]));

        let active = ConfigActiveModel {
            id: Default::default(),
            mode_code: Set(req.mode_code),
            mode_name: Set(req.mode_name),
            description: Set(req.description),
            is_active: Set(req.is_active.unwrap_or(true)),
            is_default: Set(req.is_default.unwrap_or(false)),
            process_chain: Set(process_chain),
            material_source: Set(req.material_source),
            settlement_method: Set(req.settlement_method),
            inventory_type: Set(req.inventory_type),
            cost_method: Set(req.cost_method),
            require_purchase: Set(req.require_purchase.unwrap_or(false)),
            require_production: Set(req.require_production.unwrap_or(false)),
            require_outsourcing: Set(req.require_outsourcing.unwrap_or(false)),
            require_sales: Set(req.require_sales.unwrap_or(false)),
            mode_category: Set(req.mode_category),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(req.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("业务模式配置创建失败: {}", e)))?;

        // 若设置为默认，先清除其他默认
        if result.is_default {
            self.clear_other_defaults(result.id).await?;
        }

        Ok(result)
    }

    /// 更新业务模式配置
    pub async fn update(
        &self,
        id: i32,
        req: UpdateBusinessModeConfigRequest,
    ) -> Result<ConfigModel, AppError> {
        let model = self.get_by_id(id).await?;

        // 在 model.into() 之前记录原值
        let original_mode_code = model.mode_code.clone();
        let original_material_source = model.material_source.clone();
        let original_settlement_method = model.settlement_method.clone();
        let original_require_purchase = model.require_purchase;
        let original_require_production = model.require_production;
        let original_require_outsourcing = model.require_outsourcing;
        let original_require_sales = model.require_sales;

        let mut active: ConfigActiveModel = model.into();

        // 计算最终值用于一致性校验
        let mut final_material_source = original_material_source.clone();
        let mut final_settlement_method = original_settlement_method.clone();
        let mut final_require_purchase = original_require_purchase;
        let mut final_require_production = original_require_production;
        let mut final_require_outsourcing = original_require_outsourcing;
        let mut final_require_sales = original_require_sales;

        if let Some(v) = req.mode_name {
            active.mode_name = Set(v);
        }
        if let Some(v) = req.description {
            active.description = Set(Some(v));
        }
        if let Some(v) = req.is_active {
            active.is_active = Set(v);
        }
        if let Some(v) = req.is_default {
            active.is_default = Set(v);
        }
        if let Some(v) = req.process_chain {
            active.process_chain = Set(v);
        }
        if let Some(v) = req.material_source {
            validate_material_source(&v)?;
            final_material_source = v.clone();
            active.material_source = Set(v);
        }
        if let Some(v) = req.settlement_method {
            validate_settlement_method(&v)?;
            final_settlement_method = v.clone();
            active.settlement_method = Set(v);
        }
        if let Some(v) = req.inventory_type {
            validate_inventory_type(&v)?;
            active.inventory_type = Set(v);
        }
        if let Some(v) = req.cost_method {
            validate_cost_method(&v)?;
            active.cost_method = Set(v);
        }
        if let Some(v) = req.require_purchase {
            final_require_purchase = v;
            active.require_purchase = Set(v);
        }
        if let Some(v) = req.require_production {
            final_require_production = v;
            active.require_production = Set(v);
        }
        if let Some(v) = req.require_outsourcing {
            final_require_outsourcing = v;
            active.require_outsourcing = Set(v);
        }
        if let Some(v) = req.require_sales {
            final_require_sales = v;
            active.require_sales = Set(v);
        }
        if let Some(v) = req.mode_category {
            validate_mode_category(&v)?;
            active.mode_category = Set(v);
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        // 校验业务模式配置一致性
        check_module_consistency(
            &original_mode_code,
            final_require_purchase,
            final_require_production,
            final_require_outsourcing,
            final_require_sales,
            &final_material_source,
            &final_settlement_method,
        )?;

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;

        // 若设置为默认，先清除其他默认
        if updated.is_default {
            self.clear_other_defaults(updated.id).await?;
        }

        Ok(updated)
    }

    /// 软删除业务模式配置
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;

        // 校验是否被单据引用（被引用的模式不可删除）
        let linked_count = OrderLinkEntity::find()
            .filter(business_mode_order_link::Column::ModeId.eq(id))
            .count(&*self.db)
            .await?;
        if linked_count > 0 {
            return Err(AppError::business(format!(
                "业务模式 {} 已被 {} 个单据引用，不可删除",
                model.mode_code, linked_count
            )));
        }

        let mut active: ConfigActiveModel = model.into();
        active.is_deleted = Set(true);
        active.is_active = Set(false);
        active.is_default = Set(false);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<ConfigModel, AppError> {
        ConfigEntity::find_by_id(id)
            .filter(business_mode_config::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("业务模式配置 {} 不存在", id)))
    }

    /// 按模式代码查询
    pub async fn get_by_code(&self, mode_code: &str) -> Result<ConfigModel, AppError> {
        ConfigEntity::find()
            .filter(business_mode_config::Column::ModeCode.eq(mode_code))
            .filter(business_mode_config::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("业务模式代码 {} 不存在", mode_code)))
    }

    /// 查询默认业务模式
    pub async fn get_default_mode(&self) -> Result<Option<ConfigModel>, AppError> {
        let model = ConfigEntity::find()
            .filter(business_mode_config::Column::IsDefault.eq(true))
            .filter(business_mode_config::Column::IsDeleted.eq(false))
            .filter(business_mode_config::Column::IsActive.eq(true))
            .one(&*self.db)
            .await?;
        Ok(model)
    }

    /// 设置默认业务模式（先清除其他默认，再设置当前）
    pub async fn set_default(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if !model.is_active {
            return Err(AppError::business("未启用的业务模式不可设置为默认"));
        }

        self.clear_other_defaults(id).await?;

        let mut active: ConfigActiveModel = model.into();
        active.is_default = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 获取业务模式详情（含流程节点）
    pub async fn get_with_flow_steps(
        &self,
        id: i32,
    ) -> Result<(ConfigModel, Vec<FlowStepModel>), AppError> {
        let model = self.get_by_id(id).await?;
        let steps = FlowStepEntity::find()
            .filter(business_mode_flow_step::Column::ModeId.eq(id))
            .order_by_asc(business_mode_flow_step::Column::StepNo)
            .all(&*self.db)
            .await?;
        Ok((model, steps))
    }

    /// 获取业务模式详情（含规则）
    pub async fn get_with_rules(
        &self,
        id: i32,
    ) -> Result<(ConfigModel, Vec<RuleModel>), AppError> {
        let model = self.get_by_id(id).await?;
        let rules = RuleEntity::find()
            .filter(business_mode_rule::Column::ModeId.eq(id))
            .filter(business_mode_rule::Column::IsActive.eq(true))
            .order_by_asc(business_mode_rule::Column::Id)
            .all(&*self.db)
            .await?;
        Ok((model, rules))
    }

    /// 获取业务模式完整详情（含流程节点+规则）
    pub async fn get_full_detail(
        &self,
        id: i32,
    ) -> Result<(ConfigModel, Vec<FlowStepModel>, Vec<RuleModel>), AppError> {
        let (model, steps) = self.get_with_flow_steps(id).await?;
        let rules = RuleEntity::find()
            .filter(business_mode_rule::Column::ModeId.eq(id))
            .filter(business_mode_rule::Column::IsActive.eq(true))
            .order_by_asc(business_mode_rule::Column::Id)
            .all(&*self.db)
            .await?;
        Ok((model, steps, rules))
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: BusinessModeConfigQuery,
    ) -> Result<(Vec<ConfigModel>, u64), AppError> {
        let mut q = ConfigEntity::find()
            .filter(business_mode_config::Column::IsDeleted.eq(false));
        if let Some(v) = query.mode_code {
            q = q.filter(business_mode_config::Column::ModeCode.eq(v));
        }
        if let Some(v) = query.mode_category {
            q = q.filter(business_mode_config::Column::ModeCategory.eq(v));
        }
        if let Some(v) = query.is_active {
            q = q.filter(business_mode_config::Column::IsActive.eq(v));
        }
        if let Some(v) = query.material_source {
            q = q.filter(business_mode_config::Column::MaterialSource.eq(v));
        }
        if let Some(v) = query.settlement_method {
            q = q.filter(business_mode_config::Column::SettlementMethod.eq(v));
        }
        if let Some(kw) = query.keyword {
            q = q.filter(
                Condition::any()
                    .add(business_mode_config::Column::ModeCode.contains(&kw))
                    .add(business_mode_config::Column::ModeName.contains(&kw))
                    .add(business_mode_config::Column::Description.contains(&kw)),
            );
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(business_mode_config::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }

    /// 清除其他默认模式（内部方法）
    async fn clear_other_defaults(&self, exclude_id: i32) -> Result<(), AppError> {
        let others = ConfigEntity::find()
            .filter(business_mode_config::Column::IsDefault.eq(true))
            .filter(business_mode_config::Column::IsDeleted.eq(false))
            .filter(business_mode_config::Column::Id.ne(exclude_id))
            .all(&*self.db)
            .await?;

        let now = crate::utils::date_utils::utc_now_fixed();
        for m in others {
            let mut active: ConfigActiveModel = m.into();
            active.is_default = Set(false);
            active.updated_at = Set(now);
            active.update(&*self.db).await?;
        }
        Ok(())
    }
}

// ============================================================================
// 业务模式流程节点 Service
// ============================================================================

/// 创建业务模式流程节点请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateBusinessModeFlowStepRequest {
    pub mode_id: i32,
    pub step_no: i32,
    pub step_code: String,
    pub step_name: String,
    pub module_name: String,
    pub is_required: Option<bool>,
    pub description: Option<String>,
}

/// 更新业务模式流程节点请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateBusinessModeFlowStepRequest {
    pub step_no: Option<i32>,
    pub step_code: Option<String>,
    pub step_name: Option<String>,
    pub module_name: Option<String>,
    pub is_required: Option<bool>,
    pub description: Option<String>,
}

/// 业务模式流程节点 Service
pub struct BusinessModeFlowStepService {
    db: Arc<DatabaseConnection>,
}

impl BusinessModeFlowStepService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建业务模式流程节点
    pub async fn create(
        &self,
        req: CreateBusinessModeFlowStepRequest,
    ) -> Result<FlowStepModel, AppError> {
        // 校验步骤序号非负
        if req.step_no < 1 {
            return Err(AppError::business("步骤序号必须从 1 开始"));
        }

        // 校验业务模式存在
        if ConfigEntity::find_by_id(req.mode_id)
            .filter(business_mode_config::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .is_none()
        {
            return Err(AppError::business(format!(
                "业务模式 {} 不存在",
                req.mode_id
            )));
        }

        // 校验同模式内步骤序号唯一
        if let Some(_existing) = FlowStepEntity::find()
            .filter(business_mode_flow_step::Column::ModeId.eq(req.mode_id))
            .filter(business_mode_flow_step::Column::StepNo.eq(req.step_no))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "业务模式 {} 已存在步骤序号 {}",
                req.mode_id, req.step_no
            )));
        }

        // 校验同模式内步骤代码唯一
        if let Some(_existing) = FlowStepEntity::find()
            .filter(business_mode_flow_step::Column::ModeId.eq(req.mode_id))
            .filter(business_mode_flow_step::Column::StepCode.eq(&req.step_code))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "业务模式 {} 已存在步骤代码 {}",
                req.mode_id, req.step_code
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = FlowStepActiveModel {
            id: Default::default(),
            mode_id: Set(req.mode_id),
            step_no: Set(req.step_no),
            step_code: Set(req.step_code),
            step_name: Set(req.step_name),
            module_name: Set(req.module_name),
            is_required: Set(req.is_required.unwrap_or(true)),
            description: Set(req.description),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("业务模式流程节点创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新业务模式流程节点
    pub async fn update(
        &self,
        id: i32,
        req: UpdateBusinessModeFlowStepRequest,
    ) -> Result<FlowStepModel, AppError> {
        let model = self.get_by_id(id).await?;
        let original_step_no = model.step_no;
        let original_step_code = model.step_code.clone();
        let mode_id = model.mode_id;

        let mut active: FlowStepActiveModel = model.into();

        if let Some(v) = req.step_no {
            if v < 1 {
                return Err(AppError::business("步骤序号必须从 1 开始"));
            }
            if v != original_step_no {
                // 校验同模式内步骤序号唯一
                if let Some(_existing) = FlowStepEntity::find()
                    .filter(business_mode_flow_step::Column::ModeId.eq(mode_id))
                    .filter(business_mode_flow_step::Column::StepNo.eq(v))
                    .one(&*self.db)
                    .await?
                {
                    return Err(AppError::business(format!(
                        "业务模式 {} 已存在步骤序号 {}",
                        mode_id, v
                    )));
                }
            }
            active.step_no = Set(v);
        }
        if let Some(v) = req.step_code {
            if v != original_step_code {
                // 校验同模式内步骤代码唯一
                if let Some(_existing) = FlowStepEntity::find()
                    .filter(business_mode_flow_step::Column::ModeId.eq(mode_id))
                    .filter(business_mode_flow_step::Column::StepCode.eq(&v))
                    .one(&*self.db)
                    .await?
                {
                    return Err(AppError::business(format!(
                        "业务模式 {} 已存在步骤代码 {}",
                        mode_id, v
                    )));
                }
            }
            active.step_code = Set(v);
        }
        if let Some(v) = req.step_name {
            active.step_name = Set(v);
        }
        if let Some(v) = req.module_name {
            active.module_name = Set(v);
        }
        if let Some(v) = req.is_required {
            active.is_required = Set(v);
        }
        if let Some(v) = req.description {
            active.description = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除业务模式流程节点（物理删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        let _ = model;
        FlowStepEntity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("业务模式流程节点删除失败: {}", e)))?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<FlowStepModel, AppError> {
        FlowStepEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("业务模式流程节点 {} 不存在", id)))
    }

    /// 按业务模式查询流程节点列表（按 step_no 排序）
    pub async fn list_by_mode(&self, mode_id: i32) -> Result<Vec<FlowStepModel>, AppError> {
        let items = FlowStepEntity::find()
            .filter(business_mode_flow_step::Column::ModeId.eq(mode_id))
            .order_by_asc(business_mode_flow_step::Column::StepNo)
            .all(&*self.db)
            .await?;
        Ok(items)
    }
}

// ============================================================================
// 业务模式规则 Service
// ============================================================================

/// 创建业务模式规则请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateBusinessModeRuleRequest {
    pub mode_id: i32,
    pub rule_code: String,
    pub rule_name: String,
    pub rule_type: String,
    pub module_name: String,
    pub validation_logic: Option<serde_json::Value>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

/// 更新业务模式规则请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateBusinessModeRuleRequest {
    pub rule_code: Option<String>,
    pub rule_name: Option<String>,
    pub rule_type: Option<String>,
    pub module_name: Option<String>,
    pub validation_logic: Option<serde_json::Value>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

/// 业务模式规则 Service
pub struct BusinessModeRuleService {
    db: Arc<DatabaseConnection>,
}

impl BusinessModeRuleService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建业务模式规则
    pub async fn create(&self, req: CreateBusinessModeRuleRequest) -> Result<RuleModel, AppError> {
        validate_rule_type(&req.rule_type)?;

        // 校验业务模式存在
        if ConfigEntity::find_by_id(req.mode_id)
            .filter(business_mode_config::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .is_none()
        {
            return Err(AppError::business(format!(
                "业务模式 {} 不存在",
                req.mode_id
            )));
        }

        // 校验同模式内规则代码唯一
        if let Some(_existing) = RuleEntity::find()
            .filter(business_mode_rule::Column::ModeId.eq(req.mode_id))
            .filter(business_mode_rule::Column::RuleCode.eq(&req.rule_code))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "业务模式 {} 已存在规则代码 {}",
                req.mode_id, req.rule_code
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = RuleActiveModel {
            id: Default::default(),
            mode_id: Set(req.mode_id),
            rule_code: Set(req.rule_code),
            rule_name: Set(req.rule_name),
            rule_type: Set(req.rule_type),
            module_name: Set(req.module_name),
            validation_logic: Set(req.validation_logic),
            description: Set(req.description),
            is_active: Set(req.is_active.unwrap_or(true)),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("业务模式规则创建失败: {}", e)))?;
        Ok(result)
    }

    /// 更新业务模式规则
    pub async fn update(
        &self,
        id: i32,
        req: UpdateBusinessModeRuleRequest,
    ) -> Result<RuleModel, AppError> {
        let model = self.get_by_id(id).await?;
        let original_rule_code = model.rule_code.clone();
        let mode_id = model.mode_id;

        let mut active: RuleActiveModel = model.into();

        if let Some(v) = req.rule_code {
            if v != original_rule_code {
                // 校验同模式内规则代码唯一
                if let Some(_existing) = RuleEntity::find()
                    .filter(business_mode_rule::Column::ModeId.eq(mode_id))
                    .filter(business_mode_rule::Column::RuleCode.eq(&v))
                    .one(&*self.db)
                    .await?
                {
                    return Err(AppError::business(format!(
                        "业务模式 {} 已存在规则代码 {}",
                        mode_id, v
                    )));
                }
            }
            active.rule_code = Set(v);
        }
        if let Some(v) = req.rule_name {
            active.rule_name = Set(v);
        }
        if let Some(v) = req.rule_type {
            validate_rule_type(&v)?;
            active.rule_type = Set(v);
        }
        if let Some(v) = req.module_name {
            active.module_name = Set(v);
        }
        if let Some(v) = req.validation_logic {
            active.validation_logic = Set(Some(v));
        }
        if let Some(v) = req.description {
            active.description = Set(Some(v));
        }
        if let Some(v) = req.is_active {
            active.is_active = Set(v);
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除业务模式规则（物理删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        let _ = model;
        RuleEntity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("业务模式规则删除失败: {}", e)))?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<RuleModel, AppError> {
        RuleEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("业务模式规则 {} 不存在", id)))
    }

    /// 按业务模式查询规则列表
    pub async fn list_by_mode(&self, mode_id: i32) -> Result<Vec<RuleModel>, AppError> {
        let items = RuleEntity::find()
            .filter(business_mode_rule::Column::ModeId.eq(mode_id))
            .filter(business_mode_rule::Column::IsActive.eq(true))
            .order_by_asc(business_mode_rule::Column::Id)
            .all(&*self.db)
            .await?;
        Ok(items)
    }
}

// ============================================================================
// 单据-业务模式关联 Service
// ============================================================================

/// 创建单据-业务模式关联请求
#[derive(Debug, Clone, Deserialize)]
pub struct CreateBusinessModeOrderLinkRequest {
    pub mode_id: i32,
    pub document_type: String,
    pub document_id: i32,
    pub document_no: String,
    pub mode_snapshot: Option<serde_json::Value>,
    pub remarks: Option<String>,
}

/// 更新单据-业务模式关联请求
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateBusinessModeOrderLinkRequest {
    pub mode_id: Option<i32>,
    pub mode_snapshot: Option<serde_json::Value>,
    pub remarks: Option<String>,
}

/// 单据-业务模式关联查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct BusinessModeOrderLinkQuery {
    pub mode_id: Option<i32>,
    pub document_type: Option<String>,
    pub document_no: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 单据-业务模式关联 Service
pub struct BusinessModeOrderLinkService {
    db: Arc<DatabaseConnection>,
}

impl BusinessModeOrderLinkService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 关联单据到业务模式
    pub async fn link_order(
        &self,
        mode_id: i32,
        document_type: &str,
        document_id: i32,
        document_no: &str,
        mode_snapshot: Option<serde_json::Value>,
    ) -> Result<OrderLinkModel, AppError> {
        validate_document_type(document_type)?;

        // 校验业务模式存在
        if ConfigEntity::find_by_id(mode_id)
            .filter(business_mode_config::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .is_none()
        {
            return Err(AppError::business(format!(
                "业务模式 {} 不存在",
                mode_id
            )));
        }

        // 校验单据是否已关联其他业务模式
        if let Some(_existing) = OrderLinkEntity::find()
            .filter(business_mode_order_link::Column::DocumentType.eq(document_type))
            .filter(business_mode_order_link::Column::DocumentId.eq(document_id))
            .one(&*self.db)
            .await?
        {
            return Err(AppError::business(format!(
                "单据 {}:{} 已关联业务模式，请先解除原关联",
                document_type, document_id
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();
        let active = OrderLinkActiveModel {
            id: Default::default(),
            mode_id: Set(mode_id),
            document_type: Set(document_type.to_string()),
            document_id: Set(document_id),
            document_no: Set(document_no.to_string()),
            mode_snapshot: Set(mode_snapshot),
            remarks: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let result = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("单据业务模式关联创建失败: {}", e)))?;
        Ok(result)
    }

    /// 创建单据-业务模式关联（兼容标准 CRUD 入口）
    pub async fn create(
        &self,
        req: CreateBusinessModeOrderLinkRequest,
    ) -> Result<OrderLinkModel, AppError> {
        self.link_order(
            req.mode_id,
            &req.document_type,
            req.document_id,
            &req.document_no,
            req.mode_snapshot,
        )
        .await
    }

    /// 更新单据-业务模式关联
    pub async fn update(
        &self,
        id: i32,
        req: UpdateBusinessModeOrderLinkRequest,
    ) -> Result<OrderLinkModel, AppError> {
        let model = self.get_by_id(id).await?;

        let mut active: OrderLinkActiveModel = model.into();

        if let Some(v) = req.mode_id {
            // 校验业务模式存在
            if ConfigEntity::find_by_id(v)
                .filter(business_mode_config::Column::IsDeleted.eq(false))
                .one(&*self.db)
                .await?
                .is_none()
            {
                return Err(AppError::business(format!("业务模式 {} 不存在", v)));
            }
            active.mode_id = Set(v);
        }
        if let Some(v) = req.mode_snapshot {
            active.mode_snapshot = Set(Some(v));
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(Some(v));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 删除单据-业务模式关联（物理删除，解除关联）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        let _ = model;
        OrderLinkEntity::delete_by_id(id)
            .exec(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("单据业务模式关联删除失败: {}", e)))?;
        Ok(())
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<OrderLinkModel, AppError> {
        OrderLinkEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("单据业务模式关联 {} 不存在", id)))
    }

    /// 按单据查询关联
    pub async fn get_by_document(
        &self,
        document_type: &str,
        document_id: i32,
    ) -> Result<Option<OrderLinkModel>, AppError> {
        validate_document_type(document_type)?;
        let model = OrderLinkEntity::find()
            .filter(business_mode_order_link::Column::DocumentType.eq(document_type))
            .filter(business_mode_order_link::Column::DocumentId.eq(document_id))
            .one(&*self.db)
            .await?;
        Ok(model)
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: BusinessModeOrderLinkQuery,
    ) -> Result<(Vec<OrderLinkModel>, u64), AppError> {
        let mut q = OrderLinkEntity::find();
        if let Some(v) = query.mode_id {
            q = q.filter(business_mode_order_link::Column::ModeId.eq(v));
        }
        if let Some(v) = query.document_type {
            q = q.filter(business_mode_order_link::Column::DocumentType.eq(v));
        }
        if let Some(v) = query.document_no {
            q = q.filter(business_mode_order_link::Column::DocumentNo.contains(&v));
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(business_mode_order_link::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
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

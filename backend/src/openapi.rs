//! OpenAPI 文档配置
//!
//! 使用 utoipa 生成 OpenAPI/Swagger 文档

use utoipa::OpenApi;

/// OpenAPI 文档配置
#[derive(OpenApi)]
#[openapi(
    paths(
        // 认证相关
        crate::handlers::auth_handler::login,
        crate::handlers::auth_handler::verify_token,
        
        // 用户管理
        crate::handlers::user_handler::get_users,
        crate::handlers::user_handler::create_user,
        
        // 采购合同
        crate::handlers::purchase_contract_handler::get_purchase_contracts,
        crate::handlers::purchase_contract_handler::get_purchase_contract_by_id,
        crate::handlers::purchase_contract_handler::create_purchase_contract,
        crate::handlers::purchase_contract_handler::approve_purchase_contract,
        crate::handlers::purchase_contract_handler::execute_purchase_contract,
        crate::handlers::purchase_contract_handler::cancel_purchase_contract,
        crate::handlers::purchase_contract_handler::delete_purchase_contract,
        
        // 销售合同
        crate::handlers::sales_contract_handler::get_sales_contracts,
        crate::handlers::sales_contract_handler::get_sales_contract_by_id,
        crate::handlers::sales_contract_handler::create_sales_contract,
        crate::handlers::sales_contract_handler::approve_sales_contract,
        crate::handlers::sales_contract_handler::execute_sales_contract,
        crate::handlers::sales_contract_handler::cancel_sales_contract,
        crate::handlers::sales_contract_handler::delete_sales_contract,
        
        // 固定资产
        crate::handlers::fixed_asset_handler::get_fixed_assets,
        crate::handlers::fixed_asset_handler::get_fixed_asset_by_id,
        crate::handlers::fixed_asset_handler::create_fixed_asset,
        crate::handlers::fixed_asset_handler::depreciate_fixed_asset,
        crate::handlers::fixed_asset_handler::dispose_fixed_asset,
        crate::handlers::fixed_asset_handler::delete_fixed_asset,
        
        // 预算管理
      crate::handlers::budget_management_handler::get_budget_items,
      crate::handlers::budget_management_handler::get_budget_item_by_id,
      crate::handlers::budget_management_handler::create_budget_item,
      crate::handlers::budget_management_handler::update_budget_item,
      crate::handlers::budget_management_handler::delete_budget_item,
      
      // 质量标准
      crate::handlers::quality_standard_handler::list_standards,
      crate::handlers::quality_standard_handler::get_standard,
      crate::handlers::quality_standard_handler::create_standard,
      crate::handlers::quality_standard_handler::update_standard,
      crate::handlers::quality_standard_handler::approve_standard,
      crate::handlers::quality_standard_handler::publish_standard,
      crate::handlers::quality_standard_handler::list_versions,
      crate::handlers::quality_standard_handler::create_version_history,
      
      // 资金管理
      crate::handlers::fund_management_handler::list_accounts,
      crate::handlers::fund_management_handler::get_account,
      crate::handlers::fund_management_handler::create_account,
      crate::handlers::fund_management_handler::deposit,
      crate::handlers::fund_management_handler::withdraw,
      crate::handlers::fund_management_handler::freeze_funds,
      crate::handlers::fund_management_handler::unfreeze_funds,
      crate::handlers::fund_management_handler::delete_account,
      crate::handlers::fund_management_handler::transfer,
      crate::handlers::fund_management_handler::list_transfer_records,
      crate::handlers::fund_management_handler::get_transfer_record
    ),
    components(
        schemas(
            // 认证相关
            crate::handlers::auth_handler::LoginRequest,
            crate::handlers::auth_handler::LoginResponse,
            
            // 通用响应
            crate::utils::response::ApiResponse<String>,
            
            // 采购合同
            crate::models::purchase_contract::Model,
            
            // 销售合同
            crate::models::sales_contract::Model,
            
            // 固定资产
            crate::models::fixed_asset::Model,
            
            // 预算管理
            crate::models::budget_management::Model
        )
    ),
    tags(
        (name = "认证", description = "用户认证和授权"),
      (name = "用户管理", description = "用户 CRUD 操作"),
      (name = "采购合同", description = "采购合同管理"),
      (name = "销售合同", description = "销售合同管理"),
      (name = "固定资产", description = "固定资产管理"),
      (name = "预算管理", description = "预算管理"),
      (name = "质量标准", description = "质量标准管理及审批流程"),
      (name = "资金管理", description = "资金账户管理、转账、存取款")
    ),
    info(
        title = "秉羲面料管理 API",
        description = "秉羲面料管理的 RESTful API 文档\n\n主要功能模块：\n- 用户认证与授权\n- 采购合同管理\n- 销售合同管理\n- 固定资产管理\n- 预算管理\n- 质量标准与审批流程\n- 资金账户与转账",
        version = "1.0.0",
        contact(
            name = "秉羲团队",
            email = "support@bingxi.com"
        )
    ),
    servers(
        (url = "/api/v1/erp", description = "生产环境"),
        (url = "http://localhost:8080/api/v1/erp", description = "本地开发")
    )
)]
pub struct ApiDoc;

impl ApiDoc {
    /// 创建 OpenAPI 文档
    pub fn new() -> Self {
        Self
    }
}

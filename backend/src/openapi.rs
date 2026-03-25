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
    ),
    components(
        schemas(
            // 认证相关
            crate::handlers::auth_handler::LoginRequest,
            crate::handlers::auth_handler::LoginResponse,
            
            // 通用响应
            crate::handlers::ApiResponse<String>,
            
            // 采购合同
            crate::models::purchase_contract::Model,
            
            // 销售合同
            crate::models::sales_contract::Model,
            
            // 固定资产
            crate::models::fixed_asset::Model,
            
            // 预算管理
            crate::models::budget_management::Model,
        )
    ),
    tags(
        (name = "认证", description = "用户认证和授权"),
        (name = "用户管理", description = "用户 CRUD 操作"),
        (name = "采购合同", description = "采购合同管理"),
        (name = "销售合同", description = "销售合同管理"),
        (name = "固定资产", description = "固定资产管理"),
        (name = "预算管理", description = "预算管理"),
    ),
    info(
        title = "秉羲管理系统 API",
        description = "秉羲管理系统的 RESTful API 文档",
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

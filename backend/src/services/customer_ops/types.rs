//! 客户服务类型与查询辅助函数子模块（customer_ops/types）
//!
//! 拆分：从原 `customer_service.rs` 迁移：
//! - 模块级 helper 函数（select_customer_column / build_select_only_query）
//! - 客户创建/更新参数对象（CreateCustomerArgs / UpdateCustomerArgs）
//! - 客户联系人请求 DTO（CreateCustomerContactRequest / UpdateCustomerContactRequest）
//!
//! facade 通过 `pub use` 重新导出 DTOs，保持外部引用路径不变。

use sea_orm::QuerySelect;

use crate::models::customer::{self, Entity as CustomerEntity};
use crate::utils::data_permission::{DataPermissionFilter, CUSTOMER_ALL_FIELDS};

/// 将字段名映射到客户实体的列枚举（数据库层面字段选择）
pub(super) fn select_customer_column(
    query: sea_orm::Select<CustomerEntity>,
    field: &str,
) -> sea_orm::Select<CustomerEntity> {
    use customer::Column;
    match field {
        "id" => query.column(Column::Id),
        "customer_code" => query.column(Column::CustomerCode),
        "customer_name" => query.column(Column::CustomerName),
        "contact_person" => query.column(Column::ContactPerson),
        "contact_phone" => query.column(Column::ContactPhone),
        "contact_email" => query.column(Column::ContactEmail),
        "address" => query.column(Column::Address),
        "city" => query.column(Column::City),
        "province" => query.column(Column::Province),
        "country" => query.column(Column::Country),
        "postal_code" => query.column(Column::PostalCode),
        "credit_limit" => query.column(Column::CreditLimit),
        "payment_terms" => query.column(Column::PaymentTerms),
        "tax_id" => query.column(Column::TaxId),
        "bank_name" => query.column(Column::BankName),
        "bank_account" => query.column(Column::BankAccount),
        "status" => query.column(Column::Status),
        "customer_type" => query.column(Column::CustomerType),
        "notes" => query.column(Column::Notes),
        "created_by" => query.column(Column::CreatedBy),
        "created_at" => query.column(Column::CreatedAt),
        "updated_at" => query.column(Column::UpdatedAt),
        "customer_industry" => query.column(Column::CustomerIndustry),
        "main_products" => query.column(Column::MainProducts),
        "annual_purchase" => query.column(Column::AnnualPurchase),
        "quality_requirement" => query.column(Column::QualityRequirement),
        "inspection_standard" => query.column(Column::InspectionStandard),
        _ => query,
    }
}

/// 根据数据权限过滤器构建只选择指定字段的查询
pub(super) fn build_select_only_query(
    query: sea_orm::Select<CustomerEntity>,
    filter: &DataPermissionFilter,
) -> sea_orm::Select<CustomerEntity> {
    let select_fields = filter.get_select_fields(CUSTOMER_ALL_FIELDS);
    let mut select_query = query.select_only();
    for field in &select_fields {
        select_query = select_customer_column(select_query, field);
    }
    select_query
}

/// 创建客户参数对象（消除 create_customer 的 too_many_arguments 警告）
#[derive(Debug, Clone)]
pub struct CreateCustomerArgs {
    /// 客户编码
    pub customer_code: String,
    /// 客户名称
    pub customer_name: String,
    /// 联系人
    pub contact_person: Option<String>,
    /// 联系电话
    pub contact_phone: Option<String>,
    /// 联系邮箱
    pub contact_email: Option<String>,
    /// 地址
    pub address: Option<String>,
    /// 城市
    pub city: Option<String>,
    /// 省份
    pub province: Option<String>,
    /// 国家
    pub country: Option<String>,
    /// 邮编
    pub postal_code: Option<String>,
    /// 信用额度
    pub credit_limit: rust_decimal::Decimal,
    /// 付款条件（天数）
    pub payment_terms: i32,
    /// 税号
    pub tax_id: Option<String>,
    /// 开户行
    pub bank_name: Option<String>,
    /// 银行账号
    pub bank_account: Option<String>,
    /// 客户类型
    pub customer_type: String,
    /// 备注
    pub notes: Option<String>,
    /// 创建人 ID
    pub created_by: Option<i32>,
}

/// 更新客户参数对象（消除 update_customer 的 too_many_arguments 警告）
#[derive(Debug, Clone)]
pub struct UpdateCustomerArgs {
    /// 客户 ID
    pub customer_id: i32,
    /// 客户名称
    pub customer_name: Option<String>,
    /// 联系人
    pub contact_person: Option<String>,
    /// 联系电话
    pub contact_phone: Option<String>,
    /// 联系邮箱
    pub contact_email: Option<String>,
    /// 地址
    pub address: Option<String>,
    /// 城市
    pub city: Option<String>,
    /// 省份
    pub province: Option<String>,
    /// 邮编
    pub postal_code: Option<String>,
    /// 信用额度
    pub credit_limit: Option<rust_decimal::Decimal>,
    /// 付款条件（天数）
    pub payment_terms: Option<i32>,
    /// 税号
    pub tax_id: Option<String>,
    /// 开户行
    pub bank_name: Option<String>,
    /// 银行账号
    pub bank_account: Option<String>,
    /// 客户类型
    pub customer_type: Option<String>,
    /// 状态
    pub status: Option<String>,
    /// 备注
    pub notes: Option<String>,
    /// 操作人 ID
    pub user_id: i32,
}

/// 创建客户联系人请求 DTO（批次 90b P2-12）
#[derive(Debug, serde::Deserialize, validator::Validate)]
pub struct CreateCustomerContactRequest {
    /// 联系人姓名：必填，长度 1-50
    #[validate(length(min = 1, max = 50, message = "联系人姓名长度必须在1到50个字符之间"))]
    pub name: String,
    /// 职务：可选，长度上限 100
    #[validate(length(max = 100, message = "职务长度不能超过100个字符"))]
    pub title: Option<String>,
    /// 联系电话：必填，长度 1-50（兼容手机/座机/国际号码，宽松校验）
    #[validate(length(min = 1, max = 50, message = "联系电话长度必须在1到50个字符之间"))]
    pub phone: String,
    /// 联系邮箱：可选，需符合邮箱格式
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: Option<String>,
    /// 是否主要联系人：默认 false
    #[serde(default)]
    pub is_primary: bool,
    /// 备注：可选，长度上限 500
    #[validate(length(max = 500, message = "备注长度不能超过500个字符"))]
    pub remarks: Option<String>,
}

/// 更新客户联系人请求 DTO（批次 90b P2-12）
#[derive(Debug, serde::Deserialize, validator::Validate)]
pub struct UpdateCustomerContactRequest {
    /// 联系人姓名：可选
    #[validate(length(min = 1, max = 50, message = "联系人姓名长度必须在1到50个字符之间"))]
    pub name: Option<String>,
    /// 职务：可选
    #[validate(length(max = 100, message = "职务长度不能超过100个字符"))]
    pub title: Option<String>,
    /// 联系电话：可选
    #[validate(length(min = 1, max = 50, message = "联系电话长度必须在1到50个字符之间"))]
    pub phone: Option<String>,
    /// 联系邮箱：可选
    #[validate(email(message = "邮箱格式不正确"))]
    pub email: Option<String>,
    /// 是否主要联系人：可选
    pub is_primary: Option<bool>,
    /// 备注：可选
    #[validate(length(max = 500, message = "备注长度不能超过500个字符"))]
    pub remarks: Option<String>,
}

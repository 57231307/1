#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// TODO(tech-debt): 业务接入后逐项移除此标注；rustc 1.94+ 编译时由编译器报告具体死代码位置。

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
};
use std::sync::Arc;

use crate::models::customer::{self, Entity as CustomerEntity};
use crate::models::dto::PageRequest;
use crate::utils::data_permission::{CUSTOMER_ALL_FIELDS, DataPermissionFilter};
use crate::utils::error::AppError;
use crate::utils::PaginatedResponse;

/// 将字段名映射到客户实体的列枚举
///
/// 用于数据库层面的字段选择，将字符串字段名转换为 SeaORM 的 Column 枚举
fn select_customer_column(
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
fn build_select_only_query(
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

/// 客户服务
pub struct CustomerService {
    db: Arc<DatabaseConnection>,
}

impl CustomerService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 生成客户编码
    pub async fn generate_customer_code(&self) -> Result<String, crate::utils::error::AppError> {
        use crate::utils::number_generator::DocumentNumberGenerator;
        DocumentNumberGenerator::generate_no(
            &*self.db,
            "CUS",
            customer::Entity,
            customer::Column::CustomerCode,
        )
        .await
    }

    /// 创建客户
    #[allow(clippy::too_many_arguments)]
    pub async fn create_customer(
        &self,
        customer_code: String,
        customer_name: String,
        contact_person: Option<String>,
        contact_phone: Option<String>,
        contact_email: Option<String>,
        address: Option<String>,
        city: Option<String>,
        province: Option<String>,
        country: Option<String>,
        postal_code: Option<String>,
        credit_limit: rust_decimal::Decimal,
        payment_terms: i32,
        tax_id: Option<String>,
        bank_name: Option<String>,
        bank_account: Option<String>,
        customer_type: String,
        notes: Option<String>,
        created_by: Option<i32>,
    ) -> Result<customer::Model, AppError> {
        // 检查客户编码是否已存在
        let existing = CustomerEntity::find()
            .filter(customer::Column::CustomerCode.eq(&customer_code))
            .one(&*self.db)
            .await?;

        if existing.is_some() {
            return Err(AppError::business("客户编码已存在"));
        }

        let customer = customer::ActiveModel {
            id: Default::default(),
            customer_code: sea_orm::ActiveValue::Set(customer_code),
            customer_name: sea_orm::ActiveValue::Set(customer_name),
            contact_person: sea_orm::ActiveValue::Set(contact_person),
            contact_phone: sea_orm::ActiveValue::Set(contact_phone),
            contact_email: sea_orm::ActiveValue::Set(contact_email),
            address: sea_orm::ActiveValue::Set(address),
            city: sea_orm::ActiveValue::Set(city),
            province: sea_orm::ActiveValue::Set(province),
            country: sea_orm::ActiveValue::Set(Some(country.unwrap_or_else(|| "中国".to_string()))),
            postal_code: sea_orm::ActiveValue::Set(postal_code),
            credit_limit: sea_orm::ActiveValue::Set(credit_limit),
            payment_terms: sea_orm::ActiveValue::Set(payment_terms),
            tax_id: sea_orm::ActiveValue::Set(tax_id),
            bank_name: sea_orm::ActiveValue::Set(bank_name),
            bank_account: sea_orm::ActiveValue::Set(bank_account),
            status: sea_orm::ActiveValue::Set("active".to_string()),
            customer_type: sea_orm::ActiveValue::Set(customer_type),
            notes: sea_orm::ActiveValue::Set(notes),
            created_by: sea_orm::ActiveValue::Set(created_by),
            created_at: sea_orm::ActiveValue::Set(Utc::now()),
            updated_at: sea_orm::ActiveValue::Set(Utc::now()),
            customer_industry: sea_orm::ActiveValue::NotSet,
            main_products: sea_orm::ActiveValue::NotSet,
            annual_purchase: sea_orm::ActiveValue::NotSet,
            quality_requirement: sea_orm::ActiveValue::NotSet,
            inspection_standard: sea_orm::ActiveValue::NotSet,
        };

        customer.insert(&*self.db).await.map_err(AppError::from)
    }

    /// 获取客户详情
    pub async fn get_customer(&self, customer_id: i32) -> Result<customer::Model, AppError> {
        CustomerEntity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 未找到", customer_id)))
    }

    /// 获取客户列表
    pub async fn list_customers(
        &self,
        page_req: PageRequest,
        status: Option<String>,
        customer_type: Option<String>,
        keyword: Option<String>,
    ) -> Result<PaginatedResponse<customer::Model>, AppError> {
        let mut query = CustomerEntity::find();

        // 状态筛选
        if let Some(status) = status {
            query = query.filter(customer::Column::Status.eq(status));
        }

        // 客户类型筛选
        if let Some(customer_type) = customer_type {
            query = query.filter(customer::Column::CustomerType.eq(customer_type));
        }

        // 关键词搜索
        if let Some(keyword) = keyword {
            query = query.filter(
                customer::Column::CustomerName
                    .contains(&keyword)
                    .or(customer::Column::CustomerCode.contains(&keyword)),
            );
        }

        // 总数
        let total = query.clone().count(&*self.db).await?;

        // 分页排序
        let offset = (page_req.page - 1) * page_req.page_size;
        let customers = query
            .order_by(customer::Column::CreatedAt, Order::Desc)
            .offset(offset)
            .limit(page_req.page_size)
            .all(&*self.db)
            .await?;

        Ok(PaginatedResponse::new(
            customers,
            total,
            page_req.page,
            page_req.page_size,
        ))
    }

    /// 获取客户列表（带数据权限过滤）
    ///
    /// # 参数
    /// - `page_req`: 分页参数
    /// - `status`: 状态筛选
    /// - `customer_type`: 客户类型筛选
    /// - `keyword`: 关键词搜索
    /// - `permission_filter`: 数据权限过滤器，用于在数据库层面过滤字段
    pub async fn list_customers_with_filter(
        &self,
        page_req: PageRequest,
        status: Option<String>,
        customer_type: Option<String>,
        keyword: Option<String>,
        permission_filter: Option<DataPermissionFilter>,
    ) -> Result<PaginatedResponse<serde_json::Value>, AppError> {
        let mut query = CustomerEntity::find();

        // 状态筛选
        if let Some(status) = status {
            query = query.filter(customer::Column::Status.eq(status));
        }

        // 客户类型筛选
        if let Some(customer_type) = customer_type {
            query = query.filter(customer::Column::CustomerType.eq(customer_type));
        }

        // 关键词搜索
        if let Some(keyword) = keyword {
            query = query.filter(
                customer::Column::CustomerName
                    .contains(&keyword)
                    .or(customer::Column::CustomerCode.contains(&keyword)),
            );
        }

        // 总数
        let total = query.clone().count(&*self.db).await?;

        // 分页排序
        let offset = (page_req.page - 1) * page_req.page_size;

        // 根据数据权限过滤字段
        let customers = if let Some(filter) = permission_filter {
            // 使用辅助函数构建只选择指定字段的查询
            let select_query = build_select_only_query(query, &filter);

            let rows = select_query
                .order_by(customer::Column::CreatedAt, Order::Desc)
                .offset(offset)
                .limit(page_req.page_size)
                .into_json()
                .all(&*self.db)
                .await?;

            rows
        } else {
            // 没有过滤器，查询所有字段
            let rows = query
                .order_by(customer::Column::CreatedAt, Order::Desc)
                .offset(offset)
                .limit(page_req.page_size)
                .all(&*self.db)
                .await?;

            // 转换为 JSON 值
            rows.into_iter()
                .map(|c| {
                    serde_json::to_value(c)
                        .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(PaginatedResponse::new(
            customers,
            total,
            page_req.page,
            page_req.page_size,
        ))
    }

    /// 获取客户详情（带数据权限过滤）
    ///
    /// # 参数
    /// - `customer_id`: 客户ID
    /// - `permission_filter`: 数据权限过滤器，用于在数据库层面过滤字段
    pub async fn get_customer_with_filter(
        &self,
        customer_id: i32,
        permission_filter: Option<DataPermissionFilter>,
    ) -> Result<serde_json::Value, AppError> {
        let query = CustomerEntity::find_by_id(customer_id);

        let customer = if let Some(filter) = permission_filter {
            // 使用辅助函数构建只选择指定字段的查询
            let select_query = build_select_only_query(query, &filter);

            select_query
                .into_json()
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::not_found(format!("客户 {} 未找到", customer_id)))?
        } else {
            // 没有过滤器，查询所有字段
            let model = query
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::not_found(format!("客户 {} 未找到", customer_id)))?;

            serde_json::to_value(model)
                .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?
        };

        Ok(customer)
    }

    /// 更新客户信息
    #[allow(clippy::too_many_arguments)]
    pub async fn update_customer(
        &self,
        customer_id: i32,
        customer_name: Option<String>,
        contact_person: Option<String>,
        contact_phone: Option<String>,
        contact_email: Option<String>,
        address: Option<String>,
        city: Option<String>,
        province: Option<String>,
        postal_code: Option<String>,
        credit_limit: Option<rust_decimal::Decimal>,
        payment_terms: Option<i32>,
        tax_id: Option<String>,
        bank_name: Option<String>,
        bank_account: Option<String>,
        customer_type: Option<String>,
        status: Option<String>,
        notes: Option<String>,
    ) -> Result<customer::Model, AppError> {
        let customer = CustomerEntity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 未找到", customer_id)))?;

        let mut customer_update: customer::ActiveModel = customer.into();

        if let Some(customer_name) = customer_name {
            customer_update.customer_name = sea_orm::ActiveValue::Set(customer_name);
        }
        if let Some(contact_person) = contact_person {
            customer_update.contact_person = sea_orm::ActiveValue::Set(Some(contact_person));
        }
        if let Some(contact_phone) = contact_phone {
            customer_update.contact_phone = sea_orm::ActiveValue::Set(Some(contact_phone));
        }
        if let Some(contact_email) = contact_email {
            customer_update.contact_email = sea_orm::ActiveValue::Set(Some(contact_email));
        }
        if let Some(address) = address {
            customer_update.address = sea_orm::ActiveValue::Set(Some(address));
        }
        if let Some(city) = city {
            customer_update.city = sea_orm::ActiveValue::Set(Some(city));
        }
        if let Some(province) = province {
            customer_update.province = sea_orm::ActiveValue::Set(Some(province));
        }
        if let Some(postal_code) = postal_code {
            customer_update.postal_code = sea_orm::ActiveValue::Set(Some(postal_code));
        }
        if let Some(credit_limit) = credit_limit {
            customer_update.credit_limit = sea_orm::ActiveValue::Set(credit_limit);
        }
        if let Some(payment_terms) = payment_terms {
            customer_update.payment_terms = sea_orm::ActiveValue::Set(payment_terms);
        }
        if let Some(tax_id) = tax_id {
            customer_update.tax_id = sea_orm::ActiveValue::Set(Some(tax_id));
        }
        if let Some(bank_name) = bank_name {
            customer_update.bank_name = sea_orm::ActiveValue::Set(Some(bank_name));
        }
        if let Some(bank_account) = bank_account {
            customer_update.bank_account = sea_orm::ActiveValue::Set(Some(bank_account));
        }
        if let Some(customer_type) = customer_type {
            customer_update.customer_type = sea_orm::ActiveValue::Set(customer_type);
        }
        if let Some(status) = status {
            customer_update.status = sea_orm::ActiveValue::Set(status);
        }
        if let Some(notes) = notes {
            customer_update.notes = sea_orm::ActiveValue::Set(Some(notes));
        }

        customer_update.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        customer_update
            .update(&*self.db)
            .await
            .map_err(AppError::from)
    }

    /// 删除客户（软删除，将状态改为 inactive）
    pub async fn delete_customer(&self, customer_id: i32) -> Result<customer::Model, AppError> {
        let customer = CustomerEntity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 未找到", customer_id)))?;

        let mut customer_update: customer::ActiveModel = customer.into();
        customer_update.status = sea_orm::ActiveValue::Set("inactive".to_string());
        customer_update.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        customer_update
            .update(&*self.db)
            .await
            .map_err(AppError::from)
    }

    /// 检查客户编码是否已存在
    pub async fn check_customer_code_exists(
        &self,
        customer_code: &str,
        exclude_id: Option<i32>,
    ) -> Result<bool, AppError> {
        let mut query =
            CustomerEntity::find().filter(customer::Column::CustomerCode.eq(customer_code));

        if let Some(exclude_id) = exclude_id {
            query = query.filter(customer::Column::Id.ne(exclude_id));
        }

        let count = query.count(&*self.db).await?;
        Ok(count > 0)
    }
}

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, 
    QueryFilter, QuerySelect, QueryOrder, PaginatorTrait, Order,
};
use std::sync::Arc;
use chrono::Utc;

use crate::models::dto::PageRequest;
use crate::models::customer::{self, Entity as CustomerEntity};
use crate::utils::PaginatedResponse;

/// 客户服务
pub struct CustomerService {
    db: Arc<DatabaseConnection>,
}

impl CustomerService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建客户
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
    ) -> Result<customer::Model, sea_orm::DbErr> {
        // 检查客户编码是否已存在
        let existing = CustomerEntity::find()
            .filter(customer::Column::CustomerCode.eq(&customer_code))
            .one(&*self.db)
            .await?;

        if existing.is_some() {
            return Err(sea_orm::DbErr::Custom("客户编码已存在".to_string()));
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
        };

        customer.insert(&*self.db).await
    }

    /// 获取客户详情
    pub async fn get_customer(&self, customer_id: i32) -> Result<customer::Model, sea_orm::DbErr> {
        CustomerEntity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("客户 {} 未找到", customer_id)))
    }

    /// 获取客户列表
    pub async fn list_customers(
        &self,
        page_req: PageRequest,
        status: Option<String>,
        customer_type: Option<String>,
        keyword: Option<String>,
    ) -> Result<PaginatedResponse<customer::Model>, sea_orm::DbErr> {
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
                customer::Column::CustomerName.contains(&keyword)
                    .or(customer::Column::CustomerCode.contains(&keyword))
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

        Ok(PaginatedResponse::new(customers, total, page_req.page, page_req.page_size))
    }

    /// 更新客户信息
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
    ) -> Result<customer::Model, sea_orm::DbErr> {
        let customer = CustomerEntity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("客户 {} 未找到", customer_id)))?;

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

        customer_update.update(&*self.db).await
    }

    /// 删除客户（软删除，将状态改为 inactive）
    pub async fn delete_customer(&self, customer_id: i32) -> Result<customer::Model, sea_orm::DbErr> {
        let customer = CustomerEntity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound(format!("客户 {} 未找到", customer_id)))?;

        let mut customer_update: customer::ActiveModel = customer.into();
        customer_update.status = sea_orm::ActiveValue::Set("inactive".to_string());
        customer_update.updated_at = sea_orm::ActiveValue::Set(Utc::now());

        customer_update.update(&*self.db).await
    }

    /// 检查客户编码是否已存在
    #[allow(dead_code)]
    pub async fn check_customer_code_exists(
        &self,
        customer_code: &str,
        exclude_id: Option<i32>,
    ) -> Result<bool, sea_orm::DbErr> {
        let mut query = CustomerEntity::find()
            .filter(customer::Column::CustomerCode.eq(customer_code));

        if let Some(exclude_id) = exclude_id {
            query = query.filter(customer::Column::Id.ne(exclude_id));
        }

        let count = query.count(&*self.db).await?;
        Ok(count > 0)
    }
}

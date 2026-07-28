//! 客户更新/ES 同步/编码生成 impl 子模块（customer_ops/update）
//!
//! 拆分：从原 `customer_service.rs` 迁移 CustomerService 的更新相关方法：
//! - build_customer_doc / sync_customer_to_es（ES 同步，最终一致性）
//! - generate_customer_code（客户编码生成）
//! - build_customer_active_model（create 复用的 ActiveModel 构建器）
//! - apply_customer_field_updates（update 复用的字段更新器）
//! - publish_customer_updated_side_effects（缓存失效 + 事件 + ES 同步）
//! - update_customer（事务 + lock_exclusive + 审计 + 副作用）
//!
//! redis_cache 调用（update 路径上的缓存失效）保留在 publish_customer_updated_side_effects 内。

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{EntityTrait, QuerySelect, Set, TransactionTrait};

use crate::models::customer::{self, Entity as CustomerEntity};
use crate::models::status::master_data;
use crate::search::CustomerDoc;
use crate::services::customer_ops::types::{CreateCustomerArgs, UpdateCustomerArgs};
use crate::services::customer_service::CustomerService;
use crate::services::event_bus::{BusinessEvent, EVENT_BUS};
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;
use crate::utils::redis_cache::{cache_key, redis_cache_del};

impl CustomerService {
    /// 将 customer::Model 转换为 CustomerDoc 用于 ES 索引
    pub(crate) fn build_customer_doc(model: &customer::Model) -> CustomerDoc {
        CustomerDoc {
            id: model.id,
            code: model.customer_code.clone(),
            name: model.customer_name.clone(),
            contact_person: model.contact_person.clone(),
            phone: model.contact_phone.clone(),
            email: model.contact_email.clone(),
            address: model.address.clone(),
            tier: model.customer_type.clone(),
        }
    }

    /// 同步客户到 ES（最终一致性，ES 失败仅记日志，不回滚 PG 事务）
    pub(crate) async fn sync_customer_to_es(&self, model: &customer::Model, operation: &str) {
        let doc = Self::build_customer_doc(model);
        if let Err(e) = self.search_syncer.sync_customer(&doc).await {
            tracing::warn!(
                error = %e,
                customer_id = model.id,
                customer_code = %model.customer_code,
                operation = operation,
                "ES 客户同步失败（PG 已提交，最终一致性靠补偿任务修复）"
            );
        }
    }

    /// 生成客户编码
    pub async fn generate_customer_code(&self) -> Result<String, AppError> {
        DocumentNumberGenerator::generate_no(
            &*self.db,
            "CUS",
            customer::Entity,
            customer::Column::CustomerCode,
        )
        .await
    }

    /// 构建客户 ActiveModel（create_customer 复用，默认 owner_id=created_by）
    pub(crate) fn build_customer_active_model(args: CreateCustomerArgs) -> customer::ActiveModel {
        let CreateCustomerArgs {
            customer_code,
            customer_name,
            contact_person,
            contact_phone,
            contact_email,
            address,
            city,
            province,
            country,
            postal_code,
            credit_limit,
            payment_terms,
            tax_id,
            bank_name,
            bank_account,
            customer_type,
            notes,
            created_by,
        } = args;
        customer::ActiveModel {
            id: Default::default(),
            customer_code: Set(customer_code),
            customer_name: Set(customer_name),
            contact_person: Set(contact_person),
            contact_phone: Set(contact_phone),
            contact_email: Set(contact_email),
            address: Set(address),
            ..Self::build_customer_active_model_rest(
                city,
                province,
                country,
                postal_code,
                credit_limit,
                payment_terms,
                tax_id,
                bank_name,
                bank_account,
                customer_type,
                notes,
                created_by,
            )
        }
    }

    fn build_customer_active_model_rest(
        city: String,
        province: String,
        country: Option<String>,
        postal_code: String,
        credit_limit: Decimal,
        payment_terms: String,
        tax_id: Option<String>,
        bank_name: Option<String>,
        bank_account: Option<String>,
        customer_type: String,
        notes: Option<String>,
        created_by: Option<i32>,
    ) -> customer::ActiveModel {
        customer::ActiveModel {
            city: Set(city),
            province: Set(province),
            country: Set(Some(country.unwrap_or_else(|| "中国".to_string()))),
            postal_code: Set(postal_code),
            credit_limit: Set(credit_limit),
            payment_terms: Set(payment_terms),
            tax_id: Set(tax_id),
            bank_name: Set(bank_name),
            bank_account: Set(bank_account),
            status: Set(master_data::ACTIVE.to_string()),
            customer_type: Set(customer_type),
            notes: Set(notes),
            created_by: Set(created_by),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            customer_industry: sea_orm::ActiveValue::NotSet,
            main_products: sea_orm::ActiveValue::NotSet,
            annual_purchase: sea_orm::ActiveValue::NotSet,
            quality_requirement: sea_orm::ActiveValue::NotSet,
            inspection_standard: sea_orm::ActiveValue::NotSet,
            owner_id: Set(created_by.unwrap_or(0)),
            owner_assigned_at: Set(Some(Utc::now())),
            ..Default::default()
        }
    }

    /// 应用客户字段更新（消费 UpdateCustomerArgs 构建 ActiveModel）
    pub(crate) fn apply_customer_field_updates(
        customer: customer::Model,
        args: UpdateCustomerArgs,
    ) -> customer::ActiveModel {
        let UpdateCustomerArgs {
            customer_name,
            contact_person,
            contact_phone,
            contact_email,
            address,
            city,
            province,
            postal_code,
            credit_limit,
            payment_terms,
            tax_id,
            bank_name,
            bank_account,
            customer_type,
            status,
            notes,
            ..
        } = args;
        let mut m: customer::ActiveModel = customer.into();
        Self::apply_customer_core_updates(
            &mut m,
            customer_name,
            contact_person,
            contact_phone,
            contact_email,
            address,
        );
        Self::apply_customer_extended_updates(
            &mut m,
            city,
            province,
            postal_code,
            credit_limit,
            payment_terms,
        );
        Self::apply_customer_financial_updates(
            &mut m,
            tax_id,
            bank_name,
            bank_account,
            customer_type,
            status,
            notes,
        );
        m
    }

    fn apply_customer_core_updates(
        m: &mut customer::ActiveModel,
        customer_name: Option<String>,
        contact_person: Option<String>,
        contact_phone: Option<String>,
        contact_email: Option<String>,
        address: Option<String>,
    ) {
        if let Some(v) = customer_name {
            m.customer_name = Set(v);
        }
        if let Some(v) = contact_person {
            m.contact_person = Set(Some(v));
        }
        if let Some(v) = contact_phone {
            m.contact_phone = Set(Some(v));
        }
        if let Some(v) = contact_email {
            m.contact_email = Set(Some(v));
        }
        if let Some(v) = address {
            m.address = Set(Some(v));
        }
    }

    fn apply_customer_extended_updates(
        m: &mut customer::ActiveModel,
        city: Option<String>,
        province: Option<String>,
        postal_code: Option<String>,
        credit_limit: Option<Decimal>,
        payment_terms: Option<String>,
    ) {
        if let Some(v) = city {
            m.city = Set(Some(v));
        }
        if let Some(v) = province {
            m.province = Set(Some(v));
        }
        if let Some(v) = postal_code {
            m.postal_code = Set(Some(v));
        }
        if let Some(v) = credit_limit {
            m.credit_limit = Set(v);
        }
        if let Some(v) = payment_terms {
            m.payment_terms = Set(v);
        }
    }

    fn apply_customer_financial_updates(
        m: &mut customer::ActiveModel,
        tax_id: Option<String>,
        bank_name: Option<String>,
        bank_account: Option<String>,
        customer_type: Option<String>,
        status: Option<String>,
        notes: Option<String>,
    ) {
        if let Some(v) = tax_id {
            m.tax_id = Set(Some(v));
        }
        if let Some(v) = bank_name {
            m.bank_name = Set(Some(v));
        }
        if let Some(v) = bank_account {
            m.bank_account = Set(Some(v));
        }
        if let Some(v) = customer_type {
            m.customer_type = Set(v);
        }
        if let Some(v) = status {
            m.status = Set(v);
        }
        if let Some(v) = notes {
            m.notes = Set(Some(v));
        }
    }

    /// 发布客户更新副作用（缓存失效 / 事件发布 / ES 同步）
    pub(crate) async fn publish_customer_updated_side_effects(
        &self,
        updated: &customer::Model,
        customer_id: i32,
        user_id: i32,
    ) {
        redis_cache_del(&cache_key("customer", customer_id)).await;
        EVENT_BUS.publish(BusinessEvent::CustomerUpdated {
            customer_id: updated.id,
            customer_name: updated.customer_name.clone(),
            user_id,
        });
        self.sync_customer_to_es(updated, "update").await;
    }

    /// 更新客户（事务 + lock_exclusive + 审计 + 缓存失效 + 事件 + ES 同步）
    pub async fn update_customer(
        &self,
        args: UpdateCustomerArgs,
    ) -> Result<customer::Model, AppError> {
        let customer_id = args.customer_id;
        let user_id = args.user_id;
        let txn = (*self.db).begin().await?;
        let customer = CustomerEntity::find_by_id(customer_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 未找到", customer_id)))?;
        let mut customer_update = Self::apply_customer_field_updates(customer, args);
        customer_update.updated_at = Set(Utc::now());
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "customer",
            customer_update,
            Some(user_id),
        )
        .await?;
        txn.commit().await?;
        self.publish_customer_updated_side_effects(&updated, customer_id, user_id)
            .await;
        Ok(updated)
    }
}

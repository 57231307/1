//! CRM 线索服务（crm/lead）
//!
//! 包含线索 CRUD、状态更新、线索转客户等。
//! 拆分自原 `crm_service.rs`。

use crate::models::{crm_lead, crm_opportunity, customer};
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use std::sync::Arc;

use super::cust::CrmService;

impl CrmService {
    /// 创建线索
    pub async fn create_lead(
        &self,
        req: crate::models::dto::crm_dto::CreateLeadRequest,
        user_id: i32,
    ) -> Result<crm_lead::Model, AppError> {
        // 生成线索编号（如果未提供）
        let lead_no = req.lead_no.unwrap_or_else(|| {
            format!(
                "LD{}",
                chrono::Utc::now().format("%Y%m%d%H%M%S")
            )
        });
        let lead_source = req.lead_source.unwrap_or_else(|| "OTHER".to_string());
        let owner_id = user_id;
        let owner_name = format!("用户{}", user_id);
        let contact_name = req
            .contact_name
            .unwrap_or_else(|| req.company_name.clone().unwrap_or_else(|| "未知".to_string()));
        let lead_status = req.lead_status.clone();
        let now = chrono::Utc::now();

        let lead = crm_lead::ActiveModel {
            id: Default::default(),
            lead_no: Set(lead_no),
            lead_source: Set(lead_source),
            lead_status: Set(lead_status),
            company_name: Set(req.company_name),
            contact_name: Set(contact_name),
            contact_title: Set(req.contact_title),
            mobile_phone: Set(req.mobile_phone),
            tel_phone: Set(req.tel_phone),
            email: Set(req.email),
            wechat: Set(req.wechat),
            qq: Set(req.qq),
            address: Set(req.address),
            product_interest: Set(req.product_interest),
            estimated_quantity: Set(req.estimated_quantity),
            estimated_amount: Set(req.estimated_amount),
            expected_delivery_date: Set(req.expected_delivery_date),
            requirement_desc: Set(req.requirement_desc),
            owner_id: Set(owner_id),
            owner_name: Set(owner_name),
            priority: Set(req.priority),
            rating: Set(req.rating),
            tags: Set(req.tags),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(&*self.db)
        .await?;

        Ok(lead)
    }

    /// 列出线索（返回分页结果）
    pub async fn list_leads(
        &self,
        query: crate::models::dto::crm_dto::LeadQuery,
    ) -> Result<serde_json::Value, AppError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).max(1);

        let mut q = crm_lead::Entity::find();

        if let Some(s) = query.lead_status {
            q = q.filter(crm_lead::Column::LeadStatus.eq(s));
        }

        let paginator = q
            .order_by(crm_lead::Column::CreatedAt, sea_orm::Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items: Vec<crm_lead::Model> = paginator.fetch_page(page - 1).await?;

        Ok(serde_json::json!({
            "data": items,
            "total": total,
            "page": page,
            "page_size": page_size,
        }))
    }

    /// 获取线索详情
    pub async fn get_lead(&self, lead_id: i32) -> Result<crm_lead::Model, AppError> {
        let lead = crm_lead::Entity::find_by_id(lead_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("线索 {} 不存在", lead_id)))?;
        Ok(lead)
    }

    /// 更新线索
    pub async fn update_lead(
        &self,
        lead_id: i32,
        req: crate::models::dto::crm_dto::UpdateLeadRequest,
    ) -> Result<crm_lead::Model, AppError> {
        let lead = self.get_lead(lead_id).await?;
        let mut lead_active: crm_lead::ActiveModel = lead.into();

        if let Some(v) = req.lead_source {
            lead_active.lead_source = Set(v);
        }
        if let Some(v) = req.lead_status {
            lead_active.lead_status = Set(Some(v));
        }
        if let Some(v) = req.company_name {
            lead_active.company_name = Set(Some(v));
        }
        if let Some(v) = req.contact_name {
            lead_active.contact_name = Set(v);
        }
        if let Some(v) = req.contact_title {
            lead_active.contact_title = Set(Some(v));
        }
        if let Some(v) = req.mobile_phone {
            lead_active.mobile_phone = Set(Some(v));
        }
        if let Some(v) = req.tel_phone {
            lead_active.tel_phone = Set(Some(v));
        }
        if let Some(v) = req.email {
            lead_active.email = Set(Some(v));
        }
        if let Some(v) = req.wechat {
            lead_active.wechat = Set(Some(v));
        }
        if let Some(v) = req.qq {
            lead_active.qq = Set(Some(v));
        }
        if let Some(v) = req.address {
            lead_active.address = Set(Some(v));
        }
        if let Some(v) = req.product_interest {
            lead_active.product_interest = Set(Some(v));
        }
        if let Some(v) = req.estimated_quantity {
            lead_active.estimated_quantity = Set(Some(v));
        }
        if let Some(v) = req.estimated_amount {
            lead_active.estimated_amount = Set(Some(v));
        }
        if let Some(v) = req.expected_delivery_date {
            lead_active.expected_delivery_date = Set(Some(v));
        }
        if let Some(v) = req.requirement_desc {
            lead_active.requirement_desc = Set(Some(v));
        }
        if let Some(v) = req.priority {
            lead_active.priority = Set(Some(v));
        }
        if let Some(v) = req.rating {
            lead_active.rating = Set(Some(v));
        }
        if let Some(v) = req.tags {
            lead_active.tags = Set(Some(v));
        }

        lead_active.updated_at = Set(Some(chrono::Utc::now()));

        let lead = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            lead_active,
            Some(0),
        )
        .await?;

        Ok(lead)
    }

    /// 删除线索
    pub async fn delete_lead(&self, lead_id: i32) -> Result<(), AppError> {
        crm_lead::Entity::delete_by_id(lead_id)
            .exec(&*self.db)
            .await?;
        Ok(())
    }

    /// 更新线索状态
    pub async fn update_lead_status(
        &self,
        lead_id: i32,
        status: &str,
    ) -> Result<(), AppError> {
        let lead = self.get_lead(lead_id).await?;
        let mut lead_active: crm_lead::ActiveModel = lead.into();
        lead_active.lead_status = Set(Some(status.to_string()));
        lead_active.updated_at = Set(Some(chrono::Utc::now()));

        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            lead_active,
            Some(0),
        )
        .await?;

        Ok(())
    }

    /// 将线索转换为客户（同时创建一条对应的"初步接洽"商机）
    pub async fn convert_lead_to_customer(
        &self,
        lead_id: i32,
        req: crate::models::dto::crm_dto::ConvertLeadRequest,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        // 1. 查询线索
        let lead = self.get_lead(lead_id).await?;

        if lead.lead_status.as_deref() == Some("converted") {
            return Err(AppError::business("线索已转换为客户".to_string()));
        }

        let txn = self.db.begin().await?;

        // 2. 创建客户
        let customer_code = format!("C{}", chrono::Utc::now().timestamp());
        let customer_name = lead
            .company_name
            .clone()
            .unwrap_or_else(|| lead.contact_name.clone());
        let contact_person = Some(lead.contact_name.clone());
        let contact_phone = lead.mobile_phone.clone().or(lead.tel_phone.clone());
        let customer_industry: Option<String> = None;
        let customer_type = req.customer_type.unwrap_or_else(|| "POTENTIAL".to_string());

        let new_customer = customer::ActiveModel {
            id: Default::default(),
            customer_code: Set(customer_code.clone()),
            customer_name: Set(customer_name.clone()),
            contact_person: Set(contact_person),
            contact_phone: Set(contact_phone),
            contact_email: Set(lead.email.clone()),
            address: Set(lead.address.clone()),
            city: Set(None),
            province: Set(None),
            country: Set(None),
            postal_code: Set(None),
            credit_limit: Set(rust_decimal::Decimal::ZERO),
            payment_terms: Set(30),
            tax_id: Set(None),
            bank_name: Set(None),
            bank_account: Set(None),
            status: Set("active".to_string()),
            customer_type: Set(customer_type),
            notes: Set(req.notes.clone().or(lead.requirement_desc.clone())),
            created_by: Set(Some(user_id)),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            customer_industry: Set(customer_industry),
            main_products: Set(None),
            annual_purchase: Set(None),
            quality_requirement: Set(None),
            inspection_standard: Set(None),
        }
        .insert(&txn)
        .await?;

        // 3. 更新线索状态
        let mut lead_active: crm_lead::ActiveModel = lead.clone().into();
        lead_active.lead_status = Set(Some("converted".to_string()));
        lead_active.converted_customer_id = Set(Some(new_customer.id));
        lead_active.converted_at = Set(Some(chrono::Utc::now()));
        lead_active.updated_at = Set(Some(chrono::Utc::now()));
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            lead_active,
            Some(0),
        )
        .await?;

        // 4. 创建初步商机
        let opportunity_no = format!(
            "OPP{}",
            chrono::Utc::now().format("%Y%m%d%H%M%S")
        );
        let opportunity_name = format!("{} - 初步接洽", customer_name);
        let _opportunity = crm_opportunity::ActiveModel {
            id: Default::default(),
            opportunity_no: Set(opportunity_no),
            opportunity_name: Set(opportunity_name),
            customer_id: Set(new_customer.id),
            lead_id: Set(Some(lead_id)),
            opportunity_type: Set(Some("NEW".to_string())),
            opportunity_stage: Set(Some("QUALIFICATION".to_string())),
            win_probability: Set(Some(rust_decimal::Decimal::new(20, 0))),
            estimated_amount: Set(lead.estimated_amount),
            actual_amount: Set(None),
            currency: Set(Some("CNY".to_string())),
            expected_close_date: Set(lead.expected_delivery_date),
            actual_close_date: Set(None),
            product_ids: Set(None),
            product_names: Set(None),
            product_desc: Set(lead.product_interest),
            owner_id: Set(lead.owner_id),
            owner_name: Set(lead.owner_name.clone()),
            opportunity_status: Set(Some("OPEN".to_string())),
            created_by: Set(Some(user_id)),
            created_at: Set(Some(chrono::Utc::now())),
            updated_at: Set(Some(chrono::Utc::now())),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 5. 提交事务
        txn.commit().await?;

        Ok(serde_json::json!({
            "customer_id": new_customer.id,
            "customer_code": new_customer.customer_code,
            "customer_name": new_customer.customer_name,
        }))
    }
}

/// 引用 Arc 别名
#[allow(dead_code)]
pub(crate) type DbArc = Arc<DatabaseConnection>;

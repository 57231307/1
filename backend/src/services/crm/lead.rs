//! CRM 线索服务（crm/lead）
//!
//! 包含线索 CRUD、状态更新、线索转客户等。
//! 拆分自原 `crm_service.rs`。

use crate::models::{customer, crm_lead, crm_opportunity};
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;
use uuid::Uuid;

use super::cust::CrmService;

impl CrmService {
    /// 创建线索
    pub async fn create_lead(
        &self,
        req: super::CreateLeadRequest,
        user_id: i32,
    ) -> Result<crm_lead::Model, AppError> {
        let lead = crm_lead::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            name: Set(req.name),
            contact_name: Set(req.contact_name),
            contact_phone: Set(req.contact_phone),
            contact_email: Set(req.contact_email),
            company: Set(req.company),
            source: Set(req.source),
            industry: Set(req.industry),
            expected_amount: Set(req.expected_amount),
            notes: Set(req.notes),
            status: Set("NEW".to_string()),
            owner_id: Set(req.owner_id.or(Some(user_id))),
            customer_id: Set(None),
            created_by: Set(Some(user_id)),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        }
        .insert(&*self.db)
        .await?;

        Ok(lead)
    }

    /// 列出线索（分页 + 过滤）
    pub async fn list_leads(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        owner_id: Option<i32>,
    ) -> Result<(Vec<crm_lead::Model>, u64), AppError> {
        let mut query = crm_lead::Entity::find();

        if let Some(s) = status {
            query = query.filter(crm_lead::Column::Status.eq(s));
        }
        if let Some(oid) = owner_id {
            query = query.filter(crm_lead::Column::OwnerId.eq(oid));
        }

        let paginator = query
            .order_by(crm_lead::Column::CreatedAt, sea_orm::Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page - 1).await?;
        Ok((items, total))
    }

    /// 获取线索详情
    pub async fn get_lead(&self, lead_id: &str) -> Result<crm_lead::Model, AppError> {
        let lead = crm_lead::Entity::find_by_id(lead_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("线索 {} 不存在", lead_id)))?;
        Ok(lead)
    }

    /// 更新线索
    pub async fn update_lead(
        &self,
        lead_id: &str,
        req: super::UpdateLeadRequest,
        user_id: i32,
    ) -> Result<crm_lead::Model, AppError> {
        let lead = self.get_lead(lead_id).await?;

        // 权限检查
        if let Some(owner) = lead.owner_id {
            if owner != user_id {
                return Err(AppError::permission_denied(
                    "只能修改自己负责的线索".to_string(),
                ));
            }
        }

        let mut lead_active: crm_lead::ActiveModel = lead.into();

        if let Some(name) = req.name {
            lead_active.name = Set(name);
        }
        if let Some(contact_name) = req.contact_name {
            lead_active.contact_name = Set(Some(contact_name));
        }
        if let Some(contact_phone) = req.contact_phone {
            lead_active.contact_phone = Set(Some(contact_phone));
        }
        if let Some(contact_email) = req.contact_email {
            lead_active.contact_email = Set(Some(contact_email));
        }
        if let Some(company) = req.company {
            lead_active.company = Set(Some(company));
        }
        if let Some(source) = req.source {
            lead_active.source = Set(Some(source));
        }
        if let Some(industry) = req.industry {
            lead_active.industry = Set(Some(industry));
        }
        if let Some(expected_amount) = req.expected_amount {
            lead_active.expected_amount = Set(Some(expected_amount));
        }
        if let Some(notes) = req.notes {
            lead_active.notes = Set(Some(notes));
        }
        if let Some(owner_id) = req.owner_id {
            lead_active.owner_id = Set(Some(owner_id));
        }

        lead_active.updated_at = Set(chrono::Utc::now());

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
    pub async fn delete_lead(&self, lead_id: &str, user_id: i32) -> Result<(), AppError> {
        let lead = self.get_lead(lead_id).await?;

        if let Some(owner) = lead.owner_id {
            if owner != user_id {
                return Err(AppError::permission_denied(
                    "只能删除自己负责的线索".to_string(),
                ));
            }
        }

        // 已转换的线索不能删除
        if lead.status == "CONVERTED" {
            return Err(AppError::business("已转换的线索不能删除".to_string()));
        }

        crm_lead::Entity::delete_by_id(lead_id)
            .exec(&*self.db)
            .await?;

        Ok(())
    }

    /// 更新线索状态
    pub async fn update_lead_status(
        &self,
        lead_id: &str,
        req: super::UpdateLeadStatusRequest,
        user_id: i32,
    ) -> Result<crm_lead::Model, AppError> {
        let lead = self.get_lead(lead_id).await?;

        if let Some(owner) = lead.owner_id {
            if owner != user_id {
                return Err(AppError::permission_denied(
                    "只能修改自己负责的线索".to_string(),
                ));
            }
        }

        let mut lead_active: crm_lead::ActiveModel = lead.into();
        lead_active.status = Set(req.status);
        lead_active.notes = Set(req.reason.or(lead_active.notes.take().unwrap_or(None)));
        lead_active.updated_at = Set(chrono::Utc::now());

        let lead = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            lead_active,
            Some(0),
        )
        .await?;

        Ok(lead)
    }

    /// 将线索转换为客户（同时创建一条对应的"初步接洽"商机）
    pub async fn convert_lead_to_customer(
        &self,
        lead_id: &str,
        user_id: i32,
    ) -> Result<i32, AppError> {
        // 1. 查询线索
        let lead = self.get_lead(lead_id).await?;

        if lead.status == "CONVERTED" {
            return Err(AppError::business("线索已转换为客户".to_string()));
        }

        let txn = (*self.db).begin().await?;

        // 2. 创建客户
        let new_customer = customer::ActiveModel {
            customer_code: Set(format!("C{}", chrono::Utc::now().timestamp())),
            customer_name: Set(lead.company.clone().unwrap_or_else(|| lead.name.clone())),
            contact_person: Set(lead.contact_name.clone()),
            contact_phone: Set(lead.contact_phone.clone()),
            contact_email: Set(lead.contact_email.clone()),
            industry: Set(lead.industry.clone()),
            source: Set(lead.source.clone()),
            customer_type: Set("POTENTIAL".to_string()),
            status: Set("ACTIVE".to_string()),
            owner_id: Set(lead.owner_id),
            notes: Set(lead.notes.clone()),
            created_by: Set(Some(user_id)),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 3. 更新线索状态
        let mut lead_active: crm_lead::ActiveModel = lead.clone().into();
        lead_active.status = Set("CONVERTED".to_string());
        lead_active.customer_id = Set(Some(new_customer.id));
        lead_active.updated_at = Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            lead_active,
            Some(0),
        )
        .await?;

        // 4. 创建初步商机
        let _opportunity = crm_opportunity::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            customer_id: Set(Some(new_customer.id)),
            lead_id: Set(Some(lead_id.to_string())),
            name: Set(format!("{} - 初步接洽", lead.name)),
            amount: Set(lead.expected_amount),
            stage: Set("QUALIFICATION".to_string()),
            probability: Set(Some(rust_decimal::Decimal::new(20, 0))),
            expected_close_date: Set(None),
            owner_id: Set(lead.owner_id),
            opportunity_status: Set(Some("OPEN".to_string())),
            created_by: Set(Some(user_id)),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 5. 提交事务
        txn.commit().await?;

        Ok(new_customer.id)
    }
}

/// 引用 Arc 别名
#[allow(dead_code)]
pub(crate) type DbArc = Arc<DatabaseConnection>;

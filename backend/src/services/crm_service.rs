use crate::models::dto::crm_dto::{
    CreateLeadRequest, CreateOpportunityRequest, LeadQuery, OpportunityQuery,
};
use crate::models::dto::PageResponse;
use crate::models::{crm_lead, crm_opportunity};
use crate::utils::error::AppError;
use sea_orm::*;
use std::sync::Arc;

pub struct CrmService {
    db: Arc<DatabaseConnection>,
}

impl CrmService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // --- Lead Methods ---
    pub async fn create_lead(
        &self,
        req: CreateLeadRequest,
        user_id: i32,
    ) -> Result<crm_lead::Model, AppError> {
        let lead_no = format!("LD{}", chrono::Local::now().format("%Y%m%d%H%M%S"));

        let model = crm_lead::ActiveModel {
            lead_no: Set(lead_no),
            name: Set(req.name),
            customer_name: Set(req.customer_name),
            contact_person: Set(req.contact_person),
            contact_phone: Set(req.contact_phone),
            email: Set(req.email),
            address: Set(req.address),
            source: Set(req.source),
            status: Set("NEW".to_string()),
            remarks: Set(req.remarks),
            created_by: Set(user_id),
            ..Default::default()
        };

        model
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))
    }

    pub async fn list_leads(
        &self,
        query: LeadQuery,
    ) -> Result<PageResponse<crm_lead::Model>, AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10);

        let mut stmt = crm_lead::Entity::find().order_by_desc(crm_lead::Column::CreatedAt);

        if let Some(status) = query.status {
            stmt = stmt.filter(crm_lead::Column::Status.eq(status));
        }

        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator
            .num_items()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let items = paginator
            .fetch_page(page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(PageResponse::new(items, total, page, page_size))
    }

    pub async fn update_lead_status(&self, id: i32, status: &str) -> Result<(), AppError> {
        let lead = crm_lead::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Lead not found".to_string()))?;

        let mut active: crm_lead::ActiveModel = lead.into();
        active.status = Set(status.to_string());
        active.updated_at = Set(chrono::Utc::now());
        active
            .update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    // --- Opportunity Methods ---
    pub async fn create_opportunity(
        &self,
        req: CreateOpportunityRequest,
        user_id: i32,
    ) -> Result<crm_opportunity::Model, AppError> {
        let opp_no = format!("OPP{}", chrono::Local::now().format("%Y%m%d%H%M%S"));

        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let model = crm_opportunity::ActiveModel {
            opportunity_no: Set(opp_no),
            name: Set(req.name),
            customer_id: Set(req.customer_id),
            lead_id: Set(req.lead_id),
            amount: Set(req.amount),
            expected_close_date: Set(req.expected_close_date),
            stage: Set(req.stage),
            source: Set(req.source),
            remarks: Set(req.remarks),
            created_by: Set(user_id),
            ..Default::default()
        };

        let opp = model
            .insert(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 如果是从线索转化的，更新线索状态
        if let Some(lead_id) = req.lead_id {
            if let Some(lead) = crm_lead::Entity::find_by_id(lead_id)
                .one(&txn)
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?
            {
                let mut active_lead: crm_lead::ActiveModel = lead.into();
                active_lead.status = Set("CONVERTED".to_string());
                active_lead
                    .update(&txn)
                    .await
                    .map_err(|e| AppError::DatabaseError(e.to_string()))?;
            }
        }

        txn.commit()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(opp)
    }

    pub async fn list_opportunities(
        &self,
        query: OpportunityQuery,
    ) -> Result<PageResponse<crm_opportunity::Model>, AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10);

        let mut stmt =
            crm_opportunity::Entity::find().order_by_desc(crm_opportunity::Column::CreatedAt);

        if let Some(stage) = query.stage {
            stmt = stmt.filter(crm_opportunity::Column::Stage.eq(stage));
        }

        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator
            .num_items()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let items = paginator
            .fetch_page(page - 1)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(PageResponse::new(items, total, page, page_size))
    }
}

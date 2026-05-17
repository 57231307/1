use sea_orm::*;
use std::sync::Arc;
use crate::models::{crm_lead, crm_opportunity, customer};
use crate::models::dto::crm_dto::{ConvertLeadRequest, CreateLeadRequest, CreateOpportunityRequest, LeadQuery, OpportunityQuery, UpdateLeadRequest, UpdateOpportunityRequest};
use crate::models::dto::PageResponse;
use crate::utils::error::AppError;

pub struct CrmService {
    db: Arc<DatabaseConnection>,
}

impl CrmService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // --- Lead Methods ---
    pub async fn create_lead(&self, req: CreateLeadRequest, user_id: i32) -> Result<crm_lead::Model, AppError> {
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

        model.insert(&*self.db).await.map_err(|e| AppError::DatabaseError(e.to_string()))
    }

    pub async fn list_leads(&self, query: LeadQuery) -> Result<PageResponse<crm_lead::Model>, AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10);
        
        let mut stmt = crm_lead::Entity::find().order_by_desc(crm_lead::Column::CreatedAt);
        
        if let Some(status) = query.status {
            stmt = stmt.filter(crm_lead::Column::Status.eq(status));
        }
        
        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator.num_items().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let items = paginator.fetch_page(page - 1).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(PageResponse::new(items, total, page, page_size))
    }

    pub async fn get_lead(&self, id: i32) -> Result<crm_lead::Model, AppError> {
        crm_lead::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("线索不存在".to_string()))
    }

    pub async fn update_lead(&self, id: i32, req: UpdateLeadRequest) -> Result<crm_lead::Model, AppError> {
        let lead = crm_lead::Entity::find_by_id(id).one(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("线索不存在".to_string()))?;

        let mut active: crm_lead::ActiveModel = lead.into();

        if let Some(name) = req.name {
            active.name = Set(name);
        }
        if let Some(customer_name) = req.customer_name {
            active.customer_name = Set(Some(customer_name));
        }
        if let Some(contact_person) = req.contact_person {
            active.contact_person = Set(Some(contact_person));
        }
        if let Some(contact_phone) = req.contact_phone {
            active.contact_phone = Set(Some(contact_phone));
        }
        if let Some(email) = req.email {
            active.email = Set(Some(email));
        }
        if let Some(address) = req.address {
            active.address = Set(Some(address));
        }
        if let Some(source) = req.source {
            active.source = Set(source);
        }
        if let Some(remarks) = req.remarks {
            active.remarks = Set(Some(remarks));
        }

        active.updated_at = Set(chrono::Utc::now());
        active.update(&*self.db).await.map_err(|e| AppError::DatabaseError(e.to_string()))
    }

    pub async fn delete_lead(&self, id: i32) -> Result<(), AppError> {
        let lead = crm_lead::Entity::find_by_id(id).one(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("线索不存在".to_string()))?;

        let mut active: crm_lead::ActiveModel = lead.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(chrono::Utc::now());
        active.update(&*self.db).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub async fn update_lead_status(&self, id: i32, status: &str) -> Result<(), AppError> {
        let lead = crm_lead::Entity::find_by_id(id).one(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("线索不存在".to_string()))?;

        let mut active: crm_lead::ActiveModel = lead.into();
        active.status = Set(status.to_string());
        active.updated_at = Set(chrono::Utc::now());
        active.update(&*self.db).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub async fn convert_lead_to_customer(&self, lead_id: i32, req: ConvertLeadRequest, user_id: i32) -> Result<customer::Model, AppError> {
        let txn = self.db.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let lead = crm_lead::Entity::find_by_id(lead_id)
            .one(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("线索不存在".to_string()))?;

        if lead.status == "CONVERTED" {
            return Err(AppError::BusinessError("该线索已转化".to_string()));
        }

        let customer_code = format!("CUST{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        let customer_name = lead.customer_name.clone().unwrap_or_else(|| lead.name.clone());
        let customer_type = req.customer_type.unwrap_or_else(|| "retail".to_string());

        let customer_model = customer::ActiveModel {
            id: Default::default(),
            customer_code: Set(customer_code),
            customer_name: Set(customer_name),
            contact_person: Set(lead.contact_person.clone()),
            contact_phone: Set(lead.contact_phone.clone()),
            contact_email: Set(lead.email.clone()),
            address: Set(lead.address.clone()),
            city: Default::default(),
            province: Default::default(),
            country: Default::default(),
            postal_code: Default::default(),
            credit_limit: Set(rust_decimal::Decimal::ZERO),
            payment_terms: Set(30),
            tax_id: Default::default(),
            bank_name: Default::default(),
            bank_account: Default::default(),
            status: Set("active".to_string()),
            customer_type: Set(customer_type),
            notes: Set(req.notes),
            created_by: Set(Some(user_id)),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            customer_industry: Default::default(),
            main_products: Default::default(),
            annual_purchase: Default::default(),
            quality_requirement: Default::default(),
            inspection_standard: Default::default(),
        };

        let customer = customer_model.insert(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut lead_active: crm_lead::ActiveModel = lead.into();
        lead_active.customer_id = Set(Some(customer.id));
        lead_active.status = Set("CONVERTED".to_string());
        lead_active.updated_at = Set(chrono::Utc::now());
        lead_active.update(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        txn.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(customer)
    }

    // --- Opportunity Methods ---
    pub async fn create_opportunity(&self, req: CreateOpportunityRequest, user_id: i32) -> Result<crm_opportunity::Model, AppError> {
        let opp_no = format!("OPP{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        
        let txn = self.db.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

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

        let opp = model.insert(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 如果是从线索转化的，更新线索状态
        if let Some(lead_id) = req.lead_id {
            if let Some(lead) = crm_lead::Entity::find_by_id(lead_id).one(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))? {
                let mut active_lead: crm_lead::ActiveModel = lead.into();
                active_lead.status = Set("CONVERTED".to_string());
                active_lead.update(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
            }
        }

        txn.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(opp)
    }

    pub async fn list_opportunities(&self, query: OpportunityQuery) -> Result<PageResponse<crm_opportunity::Model>, AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10);
        
        let mut stmt = crm_opportunity::Entity::find().order_by_desc(crm_opportunity::Column::CreatedAt);
        
        if let Some(stage) = query.stage {
            stmt = stmt.filter(crm_opportunity::Column::Stage.eq(stage));
        }
        
        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator.num_items().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let items = paginator.fetch_page(page - 1).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(PageResponse::new(items, total, page, page_size))
    }

    pub async fn get_opportunity(&self, id: i32) -> Result<crm_opportunity::Model, AppError> {
        crm_opportunity::Entity::find_by_id(id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("商机不存在".to_string()))
    }

    pub async fn update_opportunity(&self, id: i32, req: UpdateOpportunityRequest) -> Result<crm_opportunity::Model, AppError> {
        let opp = crm_opportunity::Entity::find_by_id(id).one(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("商机不存在".to_string()))?;

        let mut active: crm_opportunity::ActiveModel = opp.into();

        if let Some(name) = req.name {
            active.name = Set(name);
        }
        if let Some(customer_id) = req.customer_id {
            active.customer_id = Set(Some(customer_id));
        }
        if let Some(lead_id) = req.lead_id {
            active.lead_id = Set(Some(lead_id));
        }
        if let Some(amount) = req.amount {
            active.amount = Set(amount);
        }
        if let Some(expected_close_date) = req.expected_close_date {
            active.expected_close_date = Set(Some(expected_close_date));
        }
        if let Some(stage) = req.stage {
            active.stage = Set(stage);
        }
        if let Some(source) = req.source {
            active.source = Set(Some(source));
        }
        if let Some(remarks) = req.remarks {
            active.remarks = Set(Some(remarks));
        }

        active.updated_at = Set(chrono::Utc::now());
        active.update(&*self.db).await.map_err(|e| AppError::DatabaseError(e.to_string()))
    }

    pub async fn delete_opportunity(&self, id: i32) -> Result<(), AppError> {
        let opp = crm_opportunity::Entity::find_by_id(id).one(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("商机不存在".to_string()))?;

        let mut active: crm_opportunity::ActiveModel = opp.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(chrono::Utc::now());
        active.update(&*self.db).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    /// Get lead relation info with opportunities
    pub async fn get_lead_relation(&self, lead_id: i32) -> Result<LeadRelationInfo, AppError> {
        let lead = crm_lead::Entity::find_by_id(lead_id)
            .one(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Lead not found".to_string()))?;

        let opportunities = crm_opportunity::Entity::find()
            .filter(crm_opportunity::Column::LeadId.eq(lead_id))
            .all(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let total_amount: rust_decimal::Decimal = opportunities.iter()
            .map(|o| o.amount)
            .sum();

        Ok(LeadRelationInfo {
            lead_id: lead.id,
            lead_no: lead.lead_no,
            lead_name: lead.name,
            lead_status: lead.status,
            opportunity_count: opportunities.len() as i32,
            total_opportunity_amount: total_amount,
            opportunities: opportunities.into_iter().map(|o| OpportunityBrief {
                id: o.id,
                opportunity_no: o.opportunity_no,
                name: o.name,
                amount: Some(o.amount),
                stage: Some(o.stage),
                expected_close_date: o.expected_close_date,
            }).collect(),
        })
    }

    /// Get customer relation summary
    pub async fn get_customer_relation_summary(&self, customer_id: i32) -> Result<CustomerRelationSummary, AppError> {
        let opportunities = crm_opportunity::Entity::find()
            .filter(crm_opportunity::Column::CustomerId.eq(customer_id))
            .all(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let total_amount: rust_decimal::Decimal = opportunities.iter()
            .map(|o| o.amount)
            .sum();

        let won_amount: rust_decimal::Decimal = opportunities.iter()
            .filter(|o| o.stage == "WON")
            .map(|o| o.amount)
            .sum();

        Ok(CustomerRelationSummary {
            customer_id,
            opportunity_count: opportunities.len() as i32,
            total_amount,
            won_amount,
            won_count: opportunities.iter().filter(|o| o.stage == "WON").count() as i32,
            lost_count: opportunities.iter().filter(|o| o.stage == "LOST").count() as i32,
            open_count: opportunities.iter().filter(|o| {
                o.stage != "WON" && o.stage != "LOST"
            }).count() as i32,
        })
    }
}

/// Lead relation info
#[derive(Debug, serde::Serialize)]
pub struct LeadRelationInfo {
    pub lead_id: i32,
    pub lead_no: String,
    pub lead_name: String,
    pub lead_status: String,
    pub opportunity_count: i32,
    pub total_opportunity_amount: rust_decimal::Decimal,
    pub opportunities: Vec<OpportunityBrief>,
}

/// Opportunity brief info
#[derive(Debug, serde::Serialize)]
pub struct OpportunityBrief {
    pub id: i32,
    pub opportunity_no: String,
    pub name: String,
    pub amount: Option<rust_decimal::Decimal>,
    pub stage: Option<String>,
    pub expected_close_date: Option<chrono::NaiveDate>,
}

/// Customer relation summary
#[derive(Debug, serde::Serialize)]
pub struct CustomerRelationSummary {
    pub customer_id: i32,
    pub opportunity_count: i32,
    pub total_amount: rust_decimal::Decimal,
    pub won_amount: rust_decimal::Decimal,
    pub won_count: i32,
    pub lost_count: i32,
    pub open_count: i32,
}
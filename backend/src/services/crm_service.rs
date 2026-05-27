use sea_orm::*;
use std::sync::Arc;
use crate::models::{crm_lead, crm_opportunity, customer, sales_order};
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
        let lead_no = req.lead_no.unwrap_or_else(|| format!("LD{}", chrono::Local::now().format("%Y%m%d%H%M%S")));
        
        let model = crm_lead::ActiveModel {
            lead_no: Set(lead_no),
            lead_source: Set(req.lead_source.unwrap_or_else(|| "未知来源".to_string())),
            lead_status: Set(req.lead_status.or_else(|| Some("new".to_string()))),
            company_name: Set(req.company_name),
            contact_name: Set(req.contact_name.unwrap_or_default()),
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
            owner_id: Set(user_id),
            owner_name: Set("admin".to_string()),
            priority: Set(req.priority.or_else(|| Some("medium".to_string()))),
            rating: Set(req.rating.or_else(|| Some(0))),
            tags: Set(req.tags),
            created_by: Set(Some(user_id)),
            ..Default::default()
        };

        model.insert(&*self.db).await.map_err(|e| AppError::DatabaseError(e.to_string()))
    }

    pub async fn list_leads(&self, query: LeadQuery) -> Result<PageResponse<crm_lead::Model>, AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10);
        
        let mut stmt = crm_lead::Entity::find().order_by_desc(crm_lead::Column::CreatedAt);
        
        if let Some(status) = query.lead_status {
            stmt = stmt.filter(crm_lead::Column::LeadStatus.eq(status));
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

        if let Some(lead_source) = req.lead_source {
            active.lead_source = Set(lead_source);
        }
        if let Some(lead_status) = req.lead_status {
            active.lead_status = Set(Some(lead_status));
        }
        if let Some(company_name) = req.company_name {
            active.company_name = Set(Some(company_name));
        }
        if let Some(contact_name) = req.contact_name {
            active.contact_name = Set(contact_name);
        }
        if let Some(contact_title) = req.contact_title {
            active.contact_title = Set(Some(contact_title));
        }
        if let Some(mobile_phone) = req.mobile_phone {
            active.mobile_phone = Set(Some(mobile_phone));
        }
        if let Some(tel_phone) = req.tel_phone {
            active.tel_phone = Set(Some(tel_phone));
        }
        if let Some(email) = req.email {
            active.email = Set(Some(email));
        }
        if let Some(wechat) = req.wechat {
            active.wechat = Set(Some(wechat));
        }
        if let Some(qq) = req.qq {
            active.qq = Set(Some(qq));
        }
        if let Some(address) = req.address {
            active.address = Set(Some(address));
        }
        if let Some(product_interest) = req.product_interest {
            active.product_interest = Set(Some(product_interest));
        }
        if let Some(estimated_quantity) = req.estimated_quantity {
            active.estimated_quantity = Set(Some(estimated_quantity));
        }
        if let Some(estimated_amount) = req.estimated_amount {
            active.estimated_amount = Set(Some(estimated_amount));
        }
        if let Some(expected_delivery_date) = req.expected_delivery_date {
            active.expected_delivery_date = Set(Some(expected_delivery_date));
        }
        if let Some(requirement_desc) = req.requirement_desc {
            active.requirement_desc = Set(Some(requirement_desc));
        }
        if let Some(priority) = req.priority {
            active.priority = Set(Some(priority));
        }
        if let Some(rating) = req.rating {
            active.rating = Set(Some(rating));
        }
        if let Some(tags) = req.tags {
            active.tags = Set(Some(tags));
        }

        active.updated_at = Set(Some(chrono::Utc::now()));
        active.update(&*self.db).await.map_err(|e| AppError::DatabaseError(e.to_string()))
    }

    pub async fn delete_lead(&self, id: i32) -> Result<(), AppError> {
        let lead = crm_lead::Entity::find_by_id(id).one(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("线索不存在".to_string()))?;

        let active: crm_lead::ActiveModel = lead.into();
        active.delete(&*self.db).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub async fn update_lead_status(&self, id: i32, status: &str) -> Result<(), AppError> {
        let lead = crm_lead::Entity::find_by_id(id).one(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("线索不存在".to_string()))?;

        let mut active: crm_lead::ActiveModel = lead.into();
        active.lead_status = Set(Some(status.to_string()));
        active.updated_at = Set(Some(chrono::Utc::now()));
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

        if lead.lead_status.as_deref() == Some("converted") {
            return Err(AppError::BusinessError("该线索已转化".to_string()));
        }

        let customer_code = format!("CUST{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        let customer_name = lead.company_name.clone().unwrap_or_else(|| lead.contact_name.clone());
        let customer_type = req.customer_type.unwrap_or_else(|| "retail".to_string());

        let customer_model = customer::ActiveModel {
            id: Default::default(),
            customer_code: Set(customer_code),
            customer_name: Set(customer_name),
            contact_person: Set(Some(lead.contact_name.clone())),
            contact_phone: Set(lead.mobile_phone.clone()),
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
        lead_active.converted_customer_id = Set(Some(customer.id));
        lead_active.lead_status = Set(Some("converted".to_string()));
        lead_active.converted_at = Set(Some(chrono::Utc::now()));
        lead_active.updated_at = Set(Some(chrono::Utc::now()));
        lead_active.update(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        txn.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(customer)
    }

    // --- Opportunity Methods ---
    pub async fn create_opportunity(&self, req: CreateOpportunityRequest, user_id: i32) -> Result<crm_opportunity::Model, AppError> {
        let opp_no = req.opportunity_no.unwrap_or_else(|| format!("OPP{}", chrono::Local::now().format("%Y%m%d%H%M%S")));
        
        let txn = self.db.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let model = crm_opportunity::ActiveModel {
            opportunity_no: Set(opp_no),
            opportunity_name: Set(req.opportunity_name),
            customer_id: Set(req.customer_id),
            lead_id: Set(req.lead_id),
            opportunity_type: Set(req.opportunity_type),
            opportunity_stage: Set(req.opportunity_stage.or_else(|| Some("prospecting".to_string()))),
            win_probability: Set(req.win_probability),
            estimated_amount: Set(req.estimated_amount),
            actual_amount: Set(req.actual_amount),
            currency: Set(req.currency.or_else(|| Some("CNY".to_string()))),
            expected_close_date: Set(req.expected_close_date),
            actual_close_date: Set(req.actual_close_date),
            product_ids: Set(req.product_ids),
            product_names: Set(req.product_names),
            product_desc: Set(req.product_desc),
            owner_id: Set(user_id),
            owner_name: Set("admin".to_string()),
            opportunity_status: Set(Some("open".to_string())),
            priority: Set(req.priority.or_else(|| Some("medium".to_string()))),
            rating: Set(req.rating.or_else(|| Some(0))),
            tags: Set(req.tags),
            created_by: Set(Some(user_id)),
            ..Default::default()
        };

        let opp = model.insert(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 如果是从线索转化的，更新线索状态
        if let Some(lead_id) = req.lead_id {
            if let Some(lead) = crm_lead::Entity::find_by_id(lead_id).one(&txn).await.map_err(|e| AppError::DatabaseError(e.to_string()))? {
                let mut active_lead: crm_lead::ActiveModel = lead.into();
                active_lead.lead_status = Set(Some("converted".to_string()));
                active_lead.converted_opportunity_id = Set(Some(opp.id));
                active_lead.updated_at = Set(Some(chrono::Utc::now()));
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
        
        if let Some(stage) = query.opportunity_stage {
            stmt = stmt.filter(crm_opportunity::Column::OpportunityStage.eq(stage));
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

        if let Some(opportunity_name) = req.opportunity_name {
            active.opportunity_name = Set(opportunity_name);
        }
        if let Some(customer_id) = req.customer_id {
            active.customer_id = Set(customer_id);
        }
        if let Some(lead_id) = req.lead_id {
            active.lead_id = Set(Some(lead_id));
        }
        if let Some(opportunity_type) = req.opportunity_type {
            active.opportunity_type = Set(Some(opportunity_type));
        }
        if let Some(opportunity_stage) = req.opportunity_stage {
            active.opportunity_stage = Set(Some(opportunity_stage));
        }
        if let Some(win_probability) = req.win_probability {
            active.win_probability = Set(Some(win_probability));
        }
        if let Some(estimated_amount) = req.estimated_amount {
            active.estimated_amount = Set(Some(estimated_amount));
        }
        if let Some(actual_amount) = req.actual_amount {
            active.actual_amount = Set(Some(actual_amount));
        }
        if let Some(currency) = req.currency {
            active.currency = Set(Some(currency));
        }
        if let Some(expected_close_date) = req.expected_close_date {
            active.expected_close_date = Set(Some(expected_close_date));
        }
        if let Some(actual_close_date) = req.actual_close_date {
            active.actual_close_date = Set(Some(actual_close_date));
        }
        if let Some(product_ids) = req.product_ids {
            active.product_ids = Set(Some(product_ids));
        }
        if let Some(product_names) = req.product_names {
            active.product_names = Set(Some(product_names));
        }
        if let Some(product_desc) = req.product_desc {
            active.product_desc = Set(Some(product_desc));
        }
        if let Some(priority) = req.priority {
            active.priority = Set(Some(priority));
        }
        if let Some(rating) = req.rating {
            active.rating = Set(Some(rating));
        }
        if let Some(tags) = req.tags {
            active.tags = Set(Some(tags));
        }

        active.updated_at = Set(Some(chrono::Utc::now()));
        active.update(&*self.db).await.map_err(|e| AppError::DatabaseError(e.to_string()))
    }

    pub async fn delete_opportunity(&self, id: i32) -> Result<(), AppError> {
        let opp = crm_opportunity::Entity::find_by_id(id).one(&*self.db).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("商机不存在".to_string()))?;

        let active: crm_opportunity::ActiveModel = opp.into();
        active.delete(&*self.db).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    /// 将商机转化为销售订单
    pub async fn convert_opportunity_to_order(
        &self,
        opportunity_id: i32,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        let txn = self.db.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 1. 获取商机信息
        let opportunity = crm_opportunity::Entity::find_by_id(opportunity_id)
            .one(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("商机不存在".to_string()))?;

        // 检查商机状态
        if opportunity.opportunity_stage.as_deref() == Some("closed_won") ||
           opportunity.opportunity_stage.as_deref() == Some("closed_lost") {
            return Err(AppError::BusinessError("商机已关闭，无法转化".to_string()));
        }

        // 2. 创建销售订单
        let order_no = format!("SO{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        let total_amount = opportunity.estimated_amount.unwrap_or(rust_decimal::Decimal::ZERO);

        let order = sales_order::ActiveModel {
            id: Default::default(),
            order_no: Set(order_no),
            customer_id: Set(opportunity.customer_id),
            opportunity_id: Set(Some(opportunity_id)),
            order_date: Set(chrono::Utc::now()),
            required_date: Set(chrono::Utc::now() + chrono::Duration::days(30)),
            ship_date: Set(None),
            status: Set("draft".to_string()),
            subtotal: Set(total_amount),
            tax_amount: Set(rust_decimal::Decimal::ZERO),
            discount_amount: Set(rust_decimal::Decimal::ZERO),
            shipping_cost: Set(rust_decimal::Decimal::ZERO),
            total_amount: Set(total_amount),
            paid_amount: Set(rust_decimal::Decimal::ZERO),
            balance_amount: Set(total_amount),
            shipping_address: Set(None),
            billing_address: Set(None),
            notes: Set(Some(format!("从商机 {} 转化", opportunity.opportunity_no))),
            created_by: Set(Some(user_id)),
            approved_by: Set(None),
            approved_at: Set(None),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };

        let order_entity = order.insert(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // 3. 更新商机状态
        let mut opp_active: crm_opportunity::ActiveModel = opportunity.into();
        opp_active.opportunity_stage = Set(Some("closed_won".to_string()));
        opp_active.opportunity_status = Set(Some("won".to_string()));
        opp_active.actual_amount = Set(Some(total_amount));
        opp_active.actual_close_date = Set(Some(chrono::Utc::now().date_naive()));
        opp_active.updated_at = Set(Some(chrono::Utc::now()));
        
        opp_active.update(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        txn.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        tracing::info!("商机 {} 已成功转化为订单 {}", opportunity_id, order_entity.id);

        Ok(order_entity)
    }

    /// 订单完成后更新商机状态
    pub async fn update_opportunity_on_order_complete(
        &self,
        opportunity_id: i32,
        order_total_amount: rust_decimal::Decimal,
    ) -> Result<(), AppError> {
        let opportunity = crm_opportunity::Entity::find_by_id(opportunity_id)
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("商机不存在".to_string()))?;

        let mut opp_active: crm_opportunity::ActiveModel = opportunity.into();
        opp_active.opportunity_stage = Set(Some("closed_won".to_string()));
        opp_active.opportunity_status = Set(Some("won".to_string()));
        opp_active.actual_amount = Set(Some(order_total_amount));
        opp_active.actual_close_date = Set(Some(chrono::Utc::now().date_naive()));
        opp_active.won_reason = Set(Some("订单完成".to_string()));
        opp_active.updated_at = Set(Some(chrono::Utc::now()));

        opp_active.update(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        tracing::info!("商机 {} 已标记为成交，实际金额: {}", opportunity_id, order_total_amount);

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
            .filter_map(|o| o.estimated_amount)
            .sum();

        Ok(LeadRelationInfo {
            lead_id: lead.id,
            lead_no: lead.lead_no,
            lead_name: lead.contact_name,
            lead_status: lead.lead_status.unwrap_or_default(),
            opportunity_count: opportunities.len() as i32,
            total_opportunity_amount: total_amount,
            opportunities: opportunities.into_iter().map(|o| OpportunityBrief {
                id: o.id,
                opportunity_no: o.opportunity_no,
                name: o.opportunity_name,
                amount: o.estimated_amount,
                stage: o.opportunity_stage,
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
            .filter_map(|o| o.estimated_amount)
            .sum();

        let won_amount: rust_decimal::Decimal = opportunities.iter()
            .filter(|o| o.opportunity_stage.as_deref() == Some("closed_won"))
            .filter_map(|o| o.estimated_amount)
            .sum();

        Ok(CustomerRelationSummary {
            customer_id,
            opportunity_count: opportunities.len() as i32,
            total_amount,
            won_amount,
            won_count: opportunities.iter().filter(|o| o.opportunity_stage.as_deref() == Some("closed_won")).count() as i32,
            lost_count: opportunities.iter().filter(|o| o.opportunity_stage.as_deref() == Some("closed_lost")).count() as i32,
            open_count: opportunities.iter().filter(|o| {
                o.opportunity_stage.as_deref() != Some("closed_won") && o.opportunity_stage.as_deref() != Some("closed_lost")
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
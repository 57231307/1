use crate::models::dto::crm_dto::{
    ConvertLeadRequest, CreateLeadRequest, CreateOpportunityRequest, FollowUpRequest,
    LeadQuery, OpportunityQuery, RfmScoreResponse, UpdateCustomerEnhancedRequest, UpdateLeadRequest,
    UpdateOpportunityRequest,
};
use crate::models::dto::PageResponse;
use crate::models::{
    crm_lead, crm_opportunity, customer, product, sales_order, sales_order_item, user,
};
use crate::utils::error::AppError;
use chrono::TimeZone;
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
        let lead_no = req
            .lead_no
            .unwrap_or_else(|| format!("LD{}", chrono::Local::now().format("%Y%m%d%H%M%S")));

        // 查询用户真实姓名
        let owner_name = user::Entity::find_by_id(user_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("用户不存在"))?
            .username;

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
            owner_name: Set(owner_name),
            priority: Set(req.priority.or_else(|| Some("medium".to_string()))),
            rating: Set(req.rating.or(Some(0))),
            tags: Set(req.tags),
            created_by: Set(Some(user_id)),
            ..Default::default()
        };

        model
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(e.to_string()))
    }

    pub async fn list_leads(
        &self,
        query: LeadQuery,
    ) -> Result<PageResponse<crm_lead::Model>, AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(10);

        let mut stmt = crm_lead::Entity::find().order_by_desc(crm_lead::Column::CreatedAt);

        if let Some(status) = query.lead_status {
            stmt = stmt.filter(crm_lead::Column::LeadStatus.eq(status));
        }

        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page - 1).await?;

        Ok(PageResponse::new(items, total, page, page_size))
    }

    pub async fn get_lead(&self, id: i32) -> Result<crm_lead::Model, AppError> {
        crm_lead::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("线索不存在"))
    }

    pub async fn update_lead(
        &self,
        id: i32,
        req: UpdateLeadRequest,
    ) -> Result<crm_lead::Model, AppError> {
        let lead = crm_lead::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("线索不存在"))?;

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
        active
            .update(&*self.db)
            .await
            .map_err(|e| AppError::database(e.to_string()))
    }

    pub async fn delete_lead(&self, id: i32) -> Result<(), AppError> {
        let lead = crm_lead::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("线索不存在"))?;

        // 已转化的线索不允许删除，防止破坏关联数据
        if lead.lead_status.as_deref() == Some("converted") {
            return Err(AppError::business("已转化的线索不允许删除".to_string()));
        }

        // 软删除：标记为已删除状态
        let mut active: crm_lead::ActiveModel = lead.into();
        active.lead_status = Set(Some("deleted".to_string()));
        active.updated_at = Set(Some(chrono::Utc::now()));
        active.update(&*self.db).await?;
        Ok(())
    }

    pub async fn update_lead_status(&self, id: i32, status: &str) -> Result<(), AppError> {
        let lead = crm_lead::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("线索不存在"))?;

        let mut active: crm_lead::ActiveModel = lead.into();
        active.lead_status = Set(Some(status.to_string()));
        active.updated_at = Set(Some(chrono::Utc::now()));
        active.update(&*self.db).await?;
        Ok(())
    }

    pub async fn convert_lead_to_customer(
        &self,
        lead_id: i32,
        req: ConvertLeadRequest,
        user_id: i32,
    ) -> Result<customer::Model, AppError> {
        let txn = self.db.begin().await?;

        // 使用FOR UPDATE锁定行，防止并发转化
        let lead = crm_lead::Entity::find_by_id(lead_id)
            .lock(sea_orm::sea_query::LockType::Update)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("线索不存在"))?;

        if lead.lead_status.as_deref() == Some("converted") {
            return Err(AppError::business("该线索已转化"));
        }

        let lead_no = lead.lead_no.clone();
        let lead_owner_id = lead.owner_id;

        let customer_code = format!("CUST{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        let customer_name = lead
            .company_name
            .clone()
            .unwrap_or_else(|| lead.contact_name.clone());
        let customer_type = req.customer_type.unwrap_or_else(|| "retail".to_string());

        // 合并线索备注信息
        let mut notes_parts = Vec::new();
        if let Some(req_notes) = &req.notes {
            notes_parts.push(req_notes.clone());
        }
        if let Some(interest) = &lead.product_interest {
            notes_parts.push(format!("产品兴趣: {}", interest));
        }
        if let Some(desc) = &lead.requirement_desc {
            notes_parts.push(format!("需求描述: {}", desc));
        }
        if let Some(amount) = &lead.estimated_amount {
            notes_parts.push(format!("预估金额: {}", amount));
        }
        if let Some(quantity) = &lead.estimated_quantity {
            notes_parts.push(format!("预估数量: {}", quantity));
        }
        let merged_notes = if notes_parts.is_empty() {
            req.notes
        } else {
            Some(notes_parts.join("; "))
        };

        let customer_model = customer::ActiveModel {
            id: Default::default(),
            customer_code: Set(customer_code),
            customer_name: Set(customer_name),
            contact_person: Set(Some(lead.contact_name.clone())),
            contact_phone: Set(lead.mobile_phone.clone().or_else(|| lead.tel_phone.clone())),
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
            notes: Set(merged_notes),
            created_by: Set(Some(user_id)),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            customer_industry: Default::default(),
            main_products: Set(lead.product_interest.clone()),
            annual_purchase: Set(lead.estimated_amount),
            quality_requirement: Default::default(),
            inspection_standard: Default::default(),
        };

        let customer = customer_model.insert(&txn).await?;

        // 自动创建初始商机
        let opp_no = format!("OPP{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        let opportunity_model = crm_opportunity::ActiveModel {
            id: Default::default(),
            opportunity_no: Set(opp_no),
            opportunity_name: Set(format!("线索转化商机-{}", lead.contact_name)),
            customer_id: Set(customer.id),
            lead_id: Set(Some(lead_id)),
            opportunity_type: Set(Some("new_business".to_string())),
            opportunity_stage: Set(Some("prospecting".to_string())),
            win_probability: Set(Some(rust_decimal::Decimal::new(20, 0))),
            estimated_amount: Set(lead.estimated_amount),
            actual_amount: Set(None),
            currency: Set(Some("CNY".to_string())),
            expected_close_date: Set(lead.expected_delivery_date),
            actual_close_date: Set(None),
            product_ids: Set(None),
            product_names: Set(lead.product_interest.clone().map(|p| vec![p])),
            product_desc: Set(lead.requirement_desc.clone()),
            owner_id: Set(lead.owner_id),
            owner_name: Set(lead.owner_name.clone()),
            last_follow_up_date: Set(None),
            next_follow_up_date: Set(None),
            follow_up_plan: Set(None),
            competitor_names: Set(None),
            competitive_advantage: Set(None),
            opportunity_status: Set(Some("open".to_string())),
            won_reason: Set(None),
            lost_reason: Set(None),
            priority: Set(lead.priority.clone()),
            rating: Set(lead.rating),
            tags: Set(lead.tags.clone()),
            created_by: Set(Some(user_id)),
            updated_by: Set(None),
            created_at: Set(Some(chrono::Utc::now())),
            updated_at: Set(Some(chrono::Utc::now())),
        };

        let opportunity = opportunity_model.insert(&txn).await?;

        // 更新线索状态（一次性设置所有转化相关字段）
        let mut lead_active: crm_lead::ActiveModel = lead.into();
        lead_active.converted_customer_id = Set(Some(customer.id));
        lead_active.converted_opportunity_id = Set(Some(opportunity.id));
        lead_active.lead_status = Set(Some("converted".to_string()));
        lead_active.converted_at = Set(Some(chrono::Utc::now()));
        lead_active.updated_at = Set(Some(chrono::Utc::now()));
        lead_active.update(&txn).await?;

        txn.commit().await?;

        // 发送转化通知给销售团队
        let notification_service =
            crate::services::event_notification_service::EventNotificationService::new(
                self.db.clone(),
            );
        let _ = notification_service
            .notify_multiple_users(
                vec![lead_owner_id],
                "线索转化成功".to_string(),
                format!(
                    "线索 {} 已成功转化为客户 {}，商机 {} 已自动创建",
                    lead_no, customer.customer_name, opportunity.opportunity_no
                ),
                crate::models::notification::NotificationPriority::Normal,
                Some("CRM".to_string()),
                Some(customer.id),
                Some(format!("/crm/customers/{}", customer.id)),
            )
            .await;

        Ok(customer)
    }

    // --- Opportunity Methods ---
    pub async fn create_opportunity(
        &self,
        req: CreateOpportunityRequest,
        user_id: i32,
    ) -> Result<crm_opportunity::Model, AppError> {
        let opp_no = req
            .opportunity_no
            .unwrap_or_else(|| format!("OPP{}", chrono::Local::now().format("%Y%m%d%H%M%S")));

        let txn = self.db.begin().await?;

        // 验证customer_id是否存在
        let customer = customer::Entity::find_by_id(req.customer_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("客户不存在"))?;

        // 查询用户真实姓名
        let owner_name = user::Entity::find_by_id(user_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("用户不存在"))?
            .username;

        let model = crm_opportunity::ActiveModel {
            opportunity_no: Set(opp_no),
            opportunity_name: Set(req.opportunity_name),
            customer_id: Set(customer.id),
            lead_id: Set(req.lead_id),
            opportunity_type: Set(req.opportunity_type),
            opportunity_stage: Set(req
                .opportunity_stage
                .or_else(|| Some("prospecting".to_string()))),
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
            owner_name: Set(owner_name),
            opportunity_status: Set(Some("open".to_string())),
            priority: Set(req.priority.or_else(|| Some("medium".to_string()))),
            rating: Set(req.rating.or(Some(0))),
            tags: Set(req.tags),
            created_by: Set(Some(user_id)),
            ..Default::default()
        };

        let opp = model.insert(&txn).await?;

        // 如果是从线索转化的，更新线索状态
        if let Some(lead_id) = req.lead_id {
            if let Some(lead) = crm_lead::Entity::find_by_id(lead_id).one(&txn).await? {
                let mut active_lead: crm_lead::ActiveModel = lead.into();
                active_lead.lead_status = Set(Some("converted".to_string()));
                active_lead.converted_opportunity_id = Set(Some(opp.id));
                active_lead.updated_at = Set(Some(chrono::Utc::now()));
                active_lead.update(&txn).await?;
            }
        }

        txn.commit().await?;
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

        if let Some(stage) = query.opportunity_stage {
            stmt = stmt.filter(crm_opportunity::Column::OpportunityStage.eq(stage));
        }

        let paginator = stmt.paginate(&*self.db, page_size);
        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page - 1).await?;

        Ok(PageResponse::new(items, total, page, page_size))
    }

    pub async fn get_opportunity(&self, id: i32) -> Result<crm_opportunity::Model, AppError> {
        crm_opportunity::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("商机不存在"))
    }

    /// 验证商机阶段流转是否合法
    fn validate_opportunity_stage_transition(
        current_stage: Option<&str>,
        new_stage: &str,
    ) -> Result<(), AppError> {
        let valid_transitions: std::collections::HashMap<&str, Vec<&str>> = [
            (
                "prospecting",
                vec!["qualification", "needs_analysis", "closed_lost"],
            ),
            (
                "qualification",
                vec!["needs_analysis", "proposal", "closed_lost"],
            ),
            (
                "needs_analysis",
                vec!["proposal", "negotiation", "closed_lost"],
            ),
            ("proposal", vec!["negotiation", "closed_won", "closed_lost"]),
            ("negotiation", vec!["closed_won", "closed_lost"]),
            ("closed_won", vec![]),
            ("closed_lost", vec!["prospecting"]),
        ]
        .iter()
        .cloned()
        .collect();

        let current = current_stage.unwrap_or("prospecting");
        if let Some(allowed) = valid_transitions.get(current) {
            if allowed.contains(&new_stage) {
                Ok(())
            } else {
                Err(AppError::business(format!(
                    "商机阶段不允许从 {} 转换到 {}",
                    current, new_stage
                )))
            }
        } else {
            Err(AppError::business(format!("未知的商机阶段: {}", current)))
        }
    }

    pub async fn update_opportunity(
        &self,
        id: i32,
        req: UpdateOpportunityRequest,
    ) -> Result<crm_opportunity::Model, AppError> {
        let opp = crm_opportunity::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("商机不存在"))?;

        // 验证阶段流转合法性
        if let Some(ref new_stage) = req.opportunity_stage {
            Self::validate_opportunity_stage_transition(
                opp.opportunity_stage.as_deref(),
                new_stage,
            )?;
        }

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
        active
            .update(&*self.db)
            .await
            .map_err(|e| AppError::database(e.to_string()))
    }

    pub async fn delete_opportunity(&self, id: i32) -> Result<(), AppError> {
        let opp = crm_opportunity::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("商机不存在"))?;

        let active: crm_opportunity::ActiveModel = opp.into();
        active.delete(&*self.db).await?;
        Ok(())
    }

    /// 将商机转化为销售订单
    pub async fn convert_opportunity_to_order(
        &self,
        opportunity_id: i32,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
        let txn = self.db.begin().await?;

        // 1. 获取商机信息，使用FOR UPDATE锁定行
        let opportunity = crm_opportunity::Entity::find_by_id(opportunity_id)
            .lock(sea_orm::sea_query::LockType::Update)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("商机不存在"))?;

        // 检查商机状态
        if opportunity.opportunity_stage.as_deref() == Some("closed_won")
            || opportunity.opportunity_stage.as_deref() == Some("closed_lost")
        {
            return Err(AppError::business("商机已关闭，无法转化"));
        }

        // 2. 创建销售订单
        let order_no = format!("SO{}", chrono::Local::now().format("%Y%m%d%H%M%S"));
        let estimated_amount = opportunity
            .estimated_amount
            .unwrap_or(rust_decimal::Decimal::ZERO);

        let order = sales_order::ActiveModel {
            id: Default::default(),
            order_no: Set(order_no),
            customer_id: Set(opportunity.customer_id),
            opportunity_id: Set(Some(opportunity_id)),
            order_date: Set(chrono::Utc::now()),
            required_date: Set(chrono::Utc::now() + chrono::Duration::days(30)),
            ship_date: Set(None),
            status: Set("draft".to_string()),
            subtotal: Set(rust_decimal::Decimal::ZERO),
            tax_amount: Set(rust_decimal::Decimal::ZERO),
            discount_amount: Set(rust_decimal::Decimal::ZERO),
            shipping_cost: Set(rust_decimal::Decimal::ZERO),
            total_amount: Set(rust_decimal::Decimal::ZERO),
            paid_amount: Set(rust_decimal::Decimal::ZERO),
            balance_amount: Set(rust_decimal::Decimal::ZERO),
            shipping_address: Set(None),
            billing_address: Set(None),
            notes: Set(Some(format!("从商机 {} 转化", opportunity.opportunity_no))),
            created_by: Set(Some(user_id)),
            approved_by: Set(None),
            approved_at: Set(None),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        };

        let order_entity = order.insert(&txn).await?;

        // 3. 创建订单明细
        let mut subtotal = rust_decimal::Decimal::ZERO;
        let mut tax_amount = rust_decimal::Decimal::ZERO;
        let mut discount_amount = rust_decimal::Decimal::ZERO;
        let mut total_amount = rust_decimal::Decimal::ZERO;

        if let Some(product_ids) = &opportunity.product_ids {
            let product_names = opportunity.product_names.as_deref().unwrap_or_default();

            // 批量获取产品信息（优化N+1查询）
            let products = product::Entity::find()
                .filter(product::Column::Id.is_in(product_ids.to_vec()))
                .all(&txn)
                .await
                .map_err(|e| {
                    tracing::error!("批量查询产品失败: {}", e);
                    AppError::database(e.to_string())
                })?;
            let product_map: std::collections::HashMap<i32, product::Model> =
                products.into_iter().map(|p| (p.id, p)).collect();

            for (index, product_id) in product_ids.iter().enumerate() {
                // 查询产品信息获取标准价格
                let product = product_map
                    .get(product_id)
                    .ok_or_else(|| AppError::not_found(format!("产品 {} 不存在", product_id)))?;

                let unit_price = product
                    .standard_price
                    .unwrap_or(rust_decimal::Decimal::ZERO);
                let quantity = rust_decimal::Decimal::ONE; // 默认数量为1
                let discount_percent = rust_decimal::Decimal::ZERO;
                let tax_percent = rust_decimal::Decimal::ZERO;

                // 计算明细项金额
                let item_subtotal = quantity * unit_price;
                let item_discount =
                    item_subtotal * (discount_percent / rust_decimal::Decimal::new(100, 0));
                let item_after_discount = item_subtotal - item_discount;
                let item_tax =
                    item_after_discount * (tax_percent / rust_decimal::Decimal::new(100, 0));
                let item_total = item_after_discount + item_tax;

                // 累加订单总额
                subtotal += &item_subtotal;
                discount_amount += &item_discount;
                tax_amount += &item_tax;
                total_amount += &item_total;

                // 获取产品名称
                let product_name = product_names.get(index).cloned();

                // 创建订单明细
                let order_item = sales_order_item::ActiveModel {
                    id: Default::default(),
                    order_id: Set(order_entity.id),
                    product_id: Set(*product_id),
                    quantity: Set(quantity),
                    unit_price: Set(unit_price),
                    discount_percent: Set(discount_percent),
                    tax_percent: Set(tax_percent),
                    subtotal: Set(item_subtotal),
                    tax_amount: Set(item_tax),
                    discount_amount: Set(item_discount),
                    total_amount: Set(item_total),
                    shipped_quantity: Set(rust_decimal::Decimal::ZERO),
                    notes: Set(product_name),
                    created_at: Set(chrono::Utc::now()),
                    updated_at: Set(chrono::Utc::now()),
                    color_no: Set(String::new()),
                    color_name: Set(None),
                    pantone_code: Set(None),
                    grade_required: Set(None),
                    quantity_meters: Set(rust_decimal::Decimal::ZERO),
                    quantity_kg: Set(rust_decimal::Decimal::ZERO),
                    gram_weight: Set(None),
                    width: Set(None),
                    batch_requirement: Set(None),
                    dye_lot_requirement: Set(None),
                    base_price: Set(None),
                    color_extra_cost: Set(rust_decimal::Decimal::ZERO),
                    grade_price_diff: Set(rust_decimal::Decimal::ZERO),
                    final_price: Set(None),
                    shipped_quantity_meters: Set(rust_decimal::Decimal::ZERO),
                    shipped_quantity_kg: Set(rust_decimal::Decimal::ZERO),
                    paper_tube_weight: Set(None),
                    is_net_weight: Set(None),
                };

                order_item.insert(&txn).await?;
            }
        }

        // 如果没有产品明细，使用商机的预估金额作为总金额
        if opportunity
            .product_ids
            .as_ref()
            .is_none_or(|ids| ids.is_empty())
        {
            total_amount = estimated_amount;
            subtotal = estimated_amount;
        }

        // 4. 更新订单总金额
        let mut order_update: sales_order::ActiveModel = order_entity.into();
        order_update.subtotal = Set(subtotal);
        order_update.tax_amount = Set(tax_amount);
        order_update.discount_amount = Set(discount_amount);
        order_update.total_amount = Set(total_amount);
        order_update.balance_amount = Set(total_amount);
        order_update.updated_at = Set(chrono::Utc::now());
        let order_entity = order_update.update(&txn).await?;

        // 5. 更新商机状态
        let mut opp_active: crm_opportunity::ActiveModel = opportunity.into();
        opp_active.opportunity_stage = Set(Some("closed_won".to_string()));
        opp_active.opportunity_status = Set(Some("won".to_string()));
        opp_active.actual_amount = Set(Some(total_amount));
        opp_active.actual_close_date = Set(Some(chrono::Utc::now().date_naive()));
        opp_active.updated_at = Set(Some(chrono::Utc::now()));

        opp_active.update(&txn).await?;

        txn.commit().await?;

        tracing::info!(
            "商机 {} 已成功转化为订单 {}",
            opportunity_id,
            order_entity.id
        );

        Ok(order_entity)
    }

    /// 订单完成后更新商机状态
    pub async fn update_opportunity_on_order_complete(
        &self,
        opportunity_id: i32,
        order_total_amount: rust_decimal::Decimal,
    ) -> Result<(), AppError> {
        let txn = self.db.begin().await?;

        // 使用FOR UPDATE锁定行，防止并发更新
        let opportunity = crm_opportunity::Entity::find_by_id(opportunity_id)
            .lock(sea_orm::sea_query::LockType::Update)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("商机不存在"))?;

        let mut opp_active: crm_opportunity::ActiveModel = opportunity.into();
        opp_active.opportunity_stage = Set(Some("closed_won".to_string()));
        opp_active.opportunity_status = Set(Some("won".to_string()));
        opp_active.actual_amount = Set(Some(order_total_amount));
        opp_active.actual_close_date = Set(Some(chrono::Utc::now().date_naive()));
        opp_active.won_reason = Set(Some("订单完成".to_string()));
        opp_active.updated_at = Set(Some(chrono::Utc::now()));

        opp_active.update(&txn).await?;

        txn.commit().await?;

        tracing::info!(
            "商机 {} 已标记为成交，实际金额: {}",
            opportunity_id,
            order_total_amount
        );

        Ok(())
    }

    /// Get lead relation info with opportunities
    pub async fn get_lead_relation(&self, lead_id: i32) -> Result<LeadRelationInfo, AppError> {
        let lead = crm_lead::Entity::find_by_id(lead_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("线索不存在"))?;

        let opportunities = crm_opportunity::Entity::find()
            .filter(crm_opportunity::Column::LeadId.eq(lead_id))
            .all(&*self.db)
            .await?;

        let total_amount: rust_decimal::Decimal = opportunities
            .iter()
            .filter_map(|o| o.estimated_amount)
            .sum();

        Ok(LeadRelationInfo {
            lead_id: lead.id,
            lead_no: lead.lead_no,
            lead_name: lead.contact_name,
            lead_status: lead.lead_status.unwrap_or_default(),
            opportunity_count: opportunities.len() as i32,
            total_opportunity_amount: total_amount,
            opportunities: opportunities
                .into_iter()
                .map(|o| OpportunityBrief {
                    id: o.id,
                    opportunity_no: o.opportunity_no,
                    name: o.opportunity_name,
                    amount: o.estimated_amount,
                    stage: o.opportunity_stage,
                    expected_close_date: o.expected_close_date,
                })
                .collect(),
        })
    }

    /// Get customer relation summary
    pub async fn get_customer_relation_summary(
        &self,
        customer_id: i32,
    ) -> Result<CustomerRelationSummary, AppError> {
        let opportunities = crm_opportunity::Entity::find()
            .filter(crm_opportunity::Column::CustomerId.eq(customer_id))
            .all(&*self.db)
            .await?;

        let total_amount: rust_decimal::Decimal = opportunities
            .iter()
            .filter_map(|o| o.estimated_amount)
            .sum();

        let won_amount: rust_decimal::Decimal = opportunities
            .iter()
            .filter(|o| o.opportunity_stage.as_deref() == Some("closed_won"))
            .filter_map(|o| o.actual_amount.or(o.estimated_amount))
            .sum();

        Ok(CustomerRelationSummary {
            customer_id,
            opportunity_count: opportunities.len() as i32,
            total_amount,
            won_amount,
            won_count: opportunities
                .iter()
                .filter(|o| o.opportunity_stage.as_deref() == Some("closed_won"))
                .count() as i32,
            lost_count: opportunities
                .iter()
                .filter(|o| o.opportunity_stage.as_deref() == Some("closed_lost"))
                .count() as i32,
            open_count: opportunities
                .iter()
                .filter(|o| {
                    o.opportunity_stage.as_deref() != Some("closed_won")
                        && o.opportunity_stage.as_deref() != Some("closed_lost")
                })
                .count() as i32,
        })
    }

    // --- Customer 360 / Enhanced methods (Task 13) ---

    /// 获取客户 360 全景视图：基础信息、联系人、商机、订单、跟进、RFM 等
    pub async fn get_customer_360(
        &self,
        customer_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        // 1. 基础客户信息
        let customer = customer::Entity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))?;

        // 2. 关联的转化线索（用于查询跟进记录等）
        let lead = crm_lead::Entity::find()
            .filter(crm_lead::Column::ConvertedCustomerId.eq(customer_id))
            .one(&*self.db)
            .await?;

        // 3. 关联商机
        let opportunities = crm_opportunity::Entity::find()
            .filter(crm_opportunity::Column::CustomerId.eq(customer_id))
            .all(&*self.db)
            .await?;

        // 4. 关联销售订单
        let orders = sales_order::Entity::find()
            .filter(sales_order::Column::CustomerId.eq(customer_id))
            .order_by_desc(sales_order::Column::CreatedAt)
            .all(&*self.db)
            .await?;

        let total_orders = orders.len() as i32;
        let total_amount: rust_decimal::Decimal = orders
            .iter()
            .map(|o| o.total_amount)
            .fold(rust_decimal::Decimal::ZERO, |acc, x| acc + x);
        let last_order_date = orders.first().map(|o| o.order_date.to_rfc3339());

        // 5. RFM 评分
        let rfm_score = self.compute_rfm_score(customer_id).await?;

        // 6. 跟进记录（基于线索的 follow_up_plan/next_follow_up_date 字段派生）
        let follow_ups: Vec<serde_json::Value> = if let Some(lead_ref) = &lead {
            let mut items = Vec::new();
            if let Some(plan) = &lead_ref.follow_up_plan {
                items.push(serde_json::json!({
                    "id": format!("lead-{}-plan", lead_ref.id),
                    "customer_id": customer_id,
                    "operator_id": lead_ref.owner_id,
                    "operator_name": lead_ref.owner_name,
                    "type": "plan",
                    "content": plan,
                    "next_follow_date": lead_ref.next_follow_up_date.map(|d| d.to_string()),
                    "last_follow_date": lead_ref.last_follow_up_date.map(|d| d.to_string()),
                    "created_at": lead_ref.updated_at.map(|t| t.to_rfc3339()),
                }));
            }
            items
        } else {
            Vec::new()
        };

        // 7. 联系人/收货地址：基础客户只支持单联系人，包装为数组以与前端契约对齐
        let contacts: Vec<serde_json::Value> = if let (Some(name), Some(phone)) =
            (&customer.contact_person, &customer.contact_phone)
        {
            vec![serde_json::json!({
                "id": 1,
                "customer_id": customer_id,
                "name": name,
                "title": customer.contact_person.clone().unwrap_or_default(),
                "phone": phone,
                "email": customer.contact_email.clone().unwrap_or_default(),
                "is_primary": true,
                "created_at": customer.created_at.to_rfc3339(),
            })]
        } else {
            Vec::new()
        };

        // 8. 标签：基础客户暂不存储标签，返回空数组
        let tags: Vec<serde_json::Value> = Vec::new();

        // 9. 收货地址：基础客户暂不存储多地址，使用主地址
        let shipping_addresses: Vec<serde_json::Value> = if let Some(addr) = &customer.address {
            vec![serde_json::json!({
                "id": 1,
                "customer_id": customer_id,
                "name": customer.contact_person.clone().unwrap_or_default(),
                "phone": customer.contact_phone.clone().unwrap_or_default(),
                "province": customer.province.clone().unwrap_or_default(),
                "city": customer.city.clone().unwrap_or_default(),
                "district": "",
                "detail": addr,
                "is_default": true,
            })]
        } else {
            Vec::new()
        };

        Ok(serde_json::json!({
            "id": customer.id,
            "customer_code": customer.customer_code,
            "customer_name": customer.customer_name,
            "contact_person": customer.contact_person,
            "phone": customer.contact_phone,
            "email": customer.contact_email,
            "address": customer.address,
            "customer_type": customer.customer_type,
            "status": customer.status,
            "tax_number": customer.tax_id,
            "bank_name": customer.bank_name,
            "bank_account": customer.bank_account,
            "credit_limit": customer.credit_limit,
            "owner_id": customer.created_by,
            "owner_name": lead.as_ref().map(|l| l.owner_name.clone()),
            "tags": tags,
            "contacts": contacts,
            "shipping_addresses": shipping_addresses,
            "follow_ups": follow_ups,
            "rfm_score": rfm_score,
            "opportunities": opportunities.iter().map(|o| serde_json::json!({
                "id": o.id,
                "opportunity_no": o.opportunity_no,
                "opportunity_name": o.opportunity_name,
                "opportunity_stage": o.opportunity_stage,
                "estimated_amount": o.estimated_amount,
                "actual_amount": o.actual_amount,
                "expected_close_date": o.expected_close_date,
            })).collect::<Vec<_>>(),
            "orders": orders.iter().map(|o| serde_json::json!({
                "id": o.id,
                "order_no": o.order_no,
                "status": o.status,
                "total_amount": o.total_amount,
                "order_date": o.order_date,
                "required_date": o.required_date,
            })).collect::<Vec<_>>(),
            "total_orders": total_orders,
            "total_amount": total_amount,
            "last_order_date": last_order_date,
            "created_at": customer.created_at.to_rfc3339(),
            "updated_at": customer.updated_at.to_rfc3339(),
        }))
    }

    /// 获取客户增强详情（基于 customers 表）
    pub async fn get_customer_enhanced(
        &self,
        customer_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        let customer = customer::Entity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))?;

        Ok(serde_json::json!({
            "id": customer.id,
            "customer_code": customer.customer_code,
            "customer_name": customer.customer_name,
            "contact_person": customer.contact_person,
            "phone": customer.contact_phone,
            "email": customer.contact_email,
            "address": customer.address,
            "city": customer.city,
            "province": customer.province,
            "country": customer.country,
            "postal_code": customer.postal_code,
            "credit_limit": customer.credit_limit,
            "payment_terms": customer.payment_terms,
            "tax_id": customer.tax_id,
            "bank_name": customer.bank_name,
            "bank_account": customer.bank_account,
            "status": customer.status,
            "customer_type": customer.customer_type,
            "notes": customer.notes,
            "customer_industry": customer.customer_industry,
            "main_products": customer.main_products,
            "annual_purchase": customer.annual_purchase,
            "quality_requirement": customer.quality_requirement,
            "inspection_standard": customer.inspection_standard,
            "created_by": customer.created_by,
            "created_at": customer.created_at.to_rfc3339(),
            "updated_at": customer.updated_at.to_rfc3339(),
        }))
    }

    /// 更新客户增强信息
    pub async fn update_customer_enhanced(
        &self,
        customer_id: i32,
        req: UpdateCustomerEnhancedRequest,
    ) -> Result<serde_json::Value, AppError> {
        let cust = customer::Entity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))?;

        let mut active: customer::ActiveModel = cust.into();

        if let Some(name) = req.customer_name {
            active.customer_name = Set(name);
        }
        if let Some(contact_person) = req.contact_person {
            active.contact_person = Set(Some(contact_person));
        }
        if let Some(contact_phone) = req.contact_phone {
            active.contact_phone = Set(Some(contact_phone));
        }
        if let Some(contact_email) = req.contact_email {
            active.contact_email = Set(Some(contact_email));
        }
        if let Some(address) = req.address {
            active.address = Set(Some(address));
        }
        if let Some(city) = req.city {
            active.city = Set(Some(city));
        }
        if let Some(province) = req.province {
            active.province = Set(Some(province));
        }
        if let Some(country) = req.country {
            active.country = Set(Some(country));
        }
        if let Some(postal_code) = req.postal_code {
            active.postal_code = Set(Some(postal_code));
        }
        if let Some(credit_limit) = req.credit_limit {
            active.credit_limit = Set(credit_limit);
        }
        if let Some(payment_terms) = req.payment_terms {
            active.payment_terms = Set(payment_terms);
        }
        if let Some(tax_id) = req.tax_id {
            active.tax_id = Set(Some(tax_id));
        }
        if let Some(bank_name) = req.bank_name {
            active.bank_name = Set(Some(bank_name));
        }
        if let Some(bank_account) = req.bank_account {
            active.bank_account = Set(Some(bank_account));
        }
        if let Some(status) = req.status {
            active.status = Set(status);
        }
        if let Some(customer_type) = req.customer_type {
            active.customer_type = Set(customer_type);
        }
        if let Some(notes) = req.notes {
            active.notes = Set(Some(notes));
        }
        if let Some(industry) = req.customer_industry {
            active.customer_industry = Set(Some(industry));
        }
        if let Some(main_products) = req.main_products {
            active.main_products = Set(Some(main_products));
        }
        if let Some(annual_purchase) = req.annual_purchase {
            active.annual_purchase = Set(Some(annual_purchase));
        }
        if let Some(quality_requirement) = req.quality_requirement {
            active.quality_requirement = Set(Some(quality_requirement));
        }
        if let Some(inspection_standard) = req.inspection_standard {
            active.inspection_standard = Set(Some(inspection_standard));
        }

        active.updated_at = Set(chrono::Utc::now());
        let updated = active
            .update(&*self.db)
            .await
            .map_err(|e| AppError::database(e.to_string()))?;

        self.get_customer_enhanced(updated.id).await
    }

    /// 删除客户（软删除：标记为 inactive）
    pub async fn delete_customer_enhanced(&self, customer_id: i32) -> Result<(), AppError> {
        let cust = customer::Entity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))?;

        let mut active: customer::ActiveModel = cust.into();
        active.status = Set("inactive".to_string());
        active.updated_at = Set(chrono::Utc::now());
        active
            .update(&*self.db)
            .await
            .map_err(|e| AppError::database(e.to_string()))?;
        Ok(())
    }

    // --- Follow-up methods (Task 14) ---

    /// 列出指定客户的跟进记录（基于转化线索的跟进字段）
    pub async fn list_follow_ups(
        &self,
        customer_id: i32,
        page: u64,
        page_size: u64,
    ) -> Result<PageResponse<serde_json::Value>, AppError> {
        let lead = crm_lead::Entity::find()
            .filter(crm_lead::Column::ConvertedCustomerId.eq(customer_id))
            .one(&*self.db)
            .await?;

        let mut items: Vec<serde_json::Value> = Vec::new();
        if let Some(lead_ref) = lead {
            if let Some(plan) = &lead_ref.follow_up_plan {
                items.push(serde_json::json!({
                    "id": format!("{}-plan", lead_ref.id),
                    "customer_id": customer_id,
                    "operator_id": lead_ref.owner_id,
                    "operator_name": lead_ref.owner_name,
                    "type": "plan",
                    "content": plan,
                    "next_follow_date": lead_ref.next_follow_up_date.map(|d| d.to_string()),
                    "created_at": lead_ref.updated_at.map(|t| t.to_rfc3339()),
                }));
            }
            if let Some(last_date) = lead_ref.last_follow_up_date {
                let created_at = last_date
                    .and_hms_opt(0, 0, 0)
                    .map(|t| {
                        chrono::Utc
                            .from_utc_datetime(&t)
                            .to_rfc3339()
                    })
                    .or_else(|| lead_ref.updated_at.map(|t| t.to_rfc3339()))
                    .unwrap_or_default();
                items.push(serde_json::json!({
                    "id": format!("{}-last", lead_ref.id),
                    "customer_id": customer_id,
                    "operator_id": lead_ref.owner_id,
                    "operator_name": lead_ref.owner_name,
                    "type": "last",
                    "content": "最近跟进记录",
                    "next_follow_date": lead_ref.next_follow_up_date.map(|d| d.to_string()),
                    "created_at": created_at,
                }));
            }
        }

        let total = items.len() as u64;
        let start = ((page.saturating_sub(1)) * page_size) as usize;
        let end = (start + page_size as usize).min(items.len());
        let page_items = if start >= items.len() {
            Vec::new()
        } else {
            items[start..end].to_vec()
        };

        Ok(PageResponse::new(page_items, total, page, page_size))
    }

    /// 创建跟进记录（写入到客户对应的转化线索）
    pub async fn create_follow_up(
        &self,
        customer_id: i32,
        operator_id: i32,
        operator_name: String,
        req: FollowUpRequest,
    ) -> Result<serde_json::Value, AppError> {
        // 查找该客户对应的转化线索
        let lead = crm_lead::Entity::find()
            .filter(crm_lead::Column::ConvertedCustomerId.eq(customer_id))
            .one(&*self.db)
            .await?;

        let mut lead_model = match lead {
            Some(l) => l,
            None => {
                // 若客户没有对应的转化线索，则直接返回前端记录
                return Ok(serde_json::json!({
                    "id": format!("adhoc-{}", chrono::Utc::now().timestamp()),
                    "customer_id": customer_id,
                    "operator_id": operator_id,
                    "operator_name": operator_name,
                    "type": req.r#type,
                    "content": req.content,
                    "next_follow_date": req.next_follow_date,
                    "created_at": chrono::Utc::now().to_rfc3339(),
                }));
            }
        };

        let next_follow_date = req
            .next_follow_date
            .as_deref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        let mut active: crm_lead::ActiveModel = lead_model.clone().into();
        active.last_follow_up_date = Set(Some(chrono::Utc::now().date_naive()));
        if let Some(date) = next_follow_date {
            active.next_follow_up_date = Set(Some(date));
        }
        if let Some(content) = &req.content {
            active.follow_up_plan = Set(Some(content.clone()));
        }
        active.updated_at = Set(Some(chrono::Utc::now()));
        active
            .update(&*self.db)
            .await
            .map_err(|e| AppError::database(e.to_string()))?;

        lead_model = crm_lead::Entity::find_by_id(lead_model.id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("线索不存在"))?;

        Ok(serde_json::json!({
            "id": format!("{}-{}", lead_model.id, chrono::Utc::now().timestamp()),
            "customer_id": customer_id,
            "operator_id": operator_id,
            "operator_name": operator_name,
            "type": req.r#type,
            "content": req.content,
            "next_follow_date": lead_model.next_follow_up_date.map(|d| d.to_string()),
            "created_at": chrono::Utc::now().to_rfc3339(),
        }))
    }

    // --- RFM methods (Task 14) ---

    /// 计算单个客户的 RFM 评分
    pub async fn compute_rfm_score(
        &self,
        customer_id: i32,
    ) -> Result<RfmScoreResponse, AppError> {
        // 仅考虑非草稿/非作废状态的订单
        let orders = sales_order::Entity::find()
            .filter(sales_order::Column::CustomerId.eq(customer_id))
            .filter(sales_order::Column::Status.ne("draft"))
            .filter(sales_order::Column::Status.ne("cancelled"))
            .all(&*self.db)
            .await?;

        let frequency = orders.len() as i32;
        let monetary: rust_decimal::Decimal = orders
            .iter()
            .map(|o| o.total_amount)
            .fold(rust_decimal::Decimal::ZERO, |acc, x| acc + x);

        // 距最近一次订单的天数
        let recency = match orders.iter().map(|o| o.order_date).max() {
            Some(latest) => {
                let now = chrono::Utc::now();
                std::cmp::Ord::max(now.signed_duration_since(latest).num_days(), 0) as i32
            }
            None => 9999,
        };

        // RFM 等级判定
        let level = match (recency, frequency, monetary) {
            (r, _, _) if r > 365 => 'E',
            (r, f, _) if r <= 30 && f >= 10 => 'A',
            (r, f, _) if r <= 60 && f >= 5 => 'B',
            (r, f, _) if r <= 90 && f >= 3 => 'C',
            (r, _, _) if r <= 180 => 'D',
            _ => 'E',
        };

        let label = match level {
            'A' => "高价值客户",
            'B' => "重点保持客户",
            'C' => "一般价值客户",
            'D' => "低价值客户",
            _ => "流失风险客户",
        }
        .to_string();

        Ok(RfmScoreResponse {
            recency,
            frequency,
            monetary,
            level,
            label,
        })
    }

    /// 获取客户群体的 RFM 分布（仅统计有过订单的客户）
    pub async fn get_rfm_distribution(
        &self,
    ) -> Result<std::collections::HashMap<String, i32>, AppError> {
        // 获取所有客户
        let customers = customer::Entity::find()
            .all(&*self.db)
            .await?;

        let mut distribution = std::collections::HashMap::new();
        for cust in customers {
            let score = self.compute_rfm_score(cust.id).await?;
            if score.frequency == 0 && score.monetary == rust_decimal::Decimal::ZERO {
                continue;
            }
            let level_str = score.level.to_string();
            *distribution.entry(level_str).or_insert(0) += 1;
        }

        Ok(distribution)
    }

    // --- Pool claim methods (Task 14) ---

    /// 批量领取公海客户（由单条和批量接口共用）
    pub async fn claim_pool_customers(
        &self,
        customer_ids: Vec<i32>,
        operator_id: i32,
        operator_name: &str,
    ) -> Result<usize, AppError> {
        if customer_ids.is_empty() {
            return Ok(0);
        }

        let mut claimed = 0usize;
        for lead_id in customer_ids {
            let lead = match crm_lead::Entity::find_by_id(lead_id).one(&*self.db).await? {
                Some(l) => l,
                None => continue,
            };
            if lead.lead_status.as_deref() != Some("pool") {
                continue;
            }
            let mut active: crm_lead::ActiveModel = lead.into();
            active.lead_status = Set(Some("new".to_string()));
            active.owner_id = Set(operator_id);
            active.owner_name = Set(operator_name.to_string());
            active.updated_at = Set(Some(chrono::Utc::now()));
            match active.update(&*self.db).await {
                Ok(_) => claimed += 1,
                Err(e) => {
                    tracing::warn!("领取公海客户 {} 失败: {}", lead_id, e);
                }
            }
        }
        Ok(claimed)
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

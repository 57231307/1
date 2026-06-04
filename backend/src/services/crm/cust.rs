//! CRM 客户服务（crm/cust）
//!
//! 包含客户主服务的 `impl CrmService` 入口，按子领域继续拆分：
//! - lead.rs  线索管理（含线索转客户）
//! - opp.rs   商机管理
//! - cust.rs  客户 360 / 增强 CRUD / 跟进 / RFM（本文件）
//! - pool.rs  公海领取
//!
//! 拆分自原 `crm_service.rs`。

use crate::models::{
    crm_lead,
    crm_lead::Entity as CrmLeadEntity,
    crm_opportunity,
    crm_opportunity::Entity as CrmOpportunityEntity,
    customer,
    customer::Entity as CustomerEntity,
    customer_followup,
    customer_followup::Entity as CustomerFollowupEntity,
    sales_order::{Column as SalesOrderColumn, Entity as SalesOrderEntity},
};
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use std::sync::Arc;

/// CRM 服务
pub struct CrmService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl CrmService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 获取线索关联信息
    pub async fn get_lead_relation(
        &self,
        lead_id: i32,
    ) -> Result<Option<super::LeadRelationInfo>, AppError> {
        let result = CrmLeadEntity::find_by_id(lead_id)
            .into_model::<super::LeadRelationInfo>()
            .one(&*self.db)
            .await?;
        Ok(result)
    }

    /// 获取客户关联摘要（线索/商机/订单计数 + 跟进次数）
    pub async fn get_customer_relation_summary(
        &self,
        customer_id: i32,
    ) -> Result<super::CustomerRelationSummary, AppError> {
        // 统计商机数（线索不直接关联 customer_id，商机关联）
        let total_opportunities = CrmOpportunityEntity::find()
            .filter(crm_opportunity::Column::CustomerId.eq(customer_id))
            .count(&*self.db)
            .await? as i64;

        // 统计订单数与金额
        let orders = SalesOrderEntity::find()
            .filter(SalesOrderColumn::CustomerId.eq(customer_id))
            .all(&*self.db)
            .await?;
        let total_orders = orders.len() as i64;
        let total_order_amount: Option<rust_decimal::Decimal> = if orders.is_empty() {
            None
        } else {
            Some(orders.iter().map(|o| o.total_amount).sum())
        };

        // 统计跟进次数与最近跟进时间
        let follow_ups = CustomerFollowupEntity::find()
            .filter(customer_followup::Column::CustomerId.eq(customer_id))
            .order_by(customer_followup::Column::FollowUpAt, sea_orm::Order::Desc)
            .all(&*self.db)
            .await?;
        let follow_up_count = follow_ups.len() as i64;
        let last_interaction_at = follow_ups.first().map(|f| f.follow_up_at);

        Ok(super::CustomerRelationSummary {
            customer_id,
            total_leads: 0,
            total_opportunities,
            total_orders,
            total_order_amount,
            last_interaction_at,
            follow_up_count,
        })
    }

    /// 获取客户 360 视图（基本信息 + 关联数据 + 商机简报）
    pub async fn get_customer_360(&self, customer_id: i32) -> Result<serde_json::Value, AppError> {
        // 客户基本信息
        let customer_info = CustomerEntity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))?;

        // 关联商机
        let opportunities: Vec<super::OpportunityBrief> = CrmOpportunityEntity::find()
            .filter(crm_opportunity::Column::CustomerId.eq(customer_id))
            .order_by(crm_opportunity::Column::CreatedAt, sea_orm::Order::Desc)
            .into_model::<super::OpportunityBrief>()
            .all(&*self.db)
            .await?;

        // 关联摘要
        let summary = self.get_customer_relation_summary(customer_id).await?;

        // 最近订单
        let recent_orders = SalesOrderEntity::find()
            .filter(SalesOrderColumn::CustomerId.eq(customer_id))
            .order_by(SalesOrderColumn::CreatedAt, sea_orm::Order::Desc)
            .limit(5)
            .all(&*self.db)
            .await?;

        Ok(serde_json::json!({
            "customer": customer_info,
            "summary": summary,
            "opportunities": opportunities,
            "leads": [],
            "recent_orders": recent_orders,
        }))
    }

    /// 获取客户增强信息（包含 RFM 等分析指标）
    pub async fn get_customer_enhanced(
        &self,
        customer_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        let customer = CustomerEntity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))?;

        // RFM 评分
        let rfm_score = self.compute_rfm_score(customer_id).await?;

        // 客户层级（基于 RFM 评分）
        let level = match rfm_score {
            s if s >= 8.0 => "VIP",
            s if s >= 6.0 => "重要",
            s if s >= 4.0 => "一般",
            _ => "低价值",
        };

        Ok(serde_json::json!({
            "customer": customer,
            "rfm_score": rfm_score,
            "level": level,
        }))
    }

    /// 更新客户增强信息
    pub async fn update_customer_enhanced(
        &self,
        customer_id: i32,
        req: crate::models::dto::crm_dto::UpdateCustomerEnhancedRequest,
    ) -> Result<serde_json::Value, AppError> {
        let customer = CustomerEntity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))?;

        let mut customer_active: customer::ActiveModel = customer.into();

        if let Some(name) = req.customer_name {
            customer_active.customer_name = Set(name);
        }
        if let Some(contact) = req.contact_person {
            customer_active.contact_person = Set(Some(contact));
        }
        if let Some(phone) = req.contact_phone {
            customer_active.contact_phone = Set(Some(phone));
        }
        if let Some(email) = req.contact_email {
            customer_active.contact_email = Set(Some(email));
        }
        if let Some(address) = req.address {
            customer_active.address = Set(Some(address));
        }
        if let Some(city) = req.city {
            customer_active.city = Set(Some(city));
        }
        if let Some(province) = req.province {
            customer_active.province = Set(Some(province));
        }
        if let Some(country) = req.country {
            customer_active.country = Set(Some(country));
        }
        if let Some(postal_code) = req.postal_code {
            customer_active.postal_code = Set(Some(postal_code));
        }
        if let Some(credit_limit) = req.credit_limit {
            customer_active.credit_limit = Set(credit_limit);
        }
        if let Some(payment_terms) = req.payment_terms {
            customer_active.payment_terms = Set(payment_terms);
        }
        if let Some(tax_id) = req.tax_id {
            customer_active.tax_id = Set(Some(tax_id));
        }
        if let Some(bank_name) = req.bank_name {
            customer_active.bank_name = Set(Some(bank_name));
        }
        if let Some(bank_account) = req.bank_account {
            customer_active.bank_account = Set(Some(bank_account));
        }
        if let Some(status) = req.status {
            customer_active.status = Set(status);
        }
        if let Some(customer_type) = req.customer_type {
            customer_active.customer_type = Set(customer_type);
        }
        if let Some(notes) = req.notes {
            customer_active.notes = Set(Some(notes));
        }
        if let Some(industry) = req.customer_industry {
            customer_active.customer_industry = Set(Some(industry));
        }
        if let Some(main_products) = req.main_products {
            customer_active.main_products = Set(Some(main_products));
        }
        if let Some(annual_purchase) = req.annual_purchase {
            customer_active.annual_purchase = Set(Some(annual_purchase));
        }
        if let Some(quality_requirement) = req.quality_requirement {
            customer_active.quality_requirement = Set(Some(quality_requirement));
        }
        if let Some(inspection_standard) = req.inspection_standard {
            customer_active.inspection_standard = Set(Some(inspection_standard));
        }

        customer_active.updated_at = Set(chrono::Utc::now());

        let customer = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            customer_active,
            Some(0),
        )
        .await?;

        Ok(serde_json::to_value(customer)
            .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?)
    }

    /// 删除客户（增强版，含关联数据检查）
    pub async fn delete_customer_enhanced(
        &self,
        customer_id: i32,
    ) -> Result<(), AppError> {
        // 1. 检查是否有关联订单
        let order_count = SalesOrderEntity::find()
            .filter(SalesOrderColumn::CustomerId.eq(customer_id))
            .count(&*self.db)
            .await?;
        if order_count > 0 {
            return Err(AppError::business(format!(
                "客户存在 {} 个关联订单，无法删除",
                order_count
            )));
        }

        // 2. 检查是否有关联商机
        let opp_count = CrmOpportunityEntity::find()
            .filter(crm_opportunity::Column::CustomerId.eq(customer_id))
            .count(&*self.db)
            .await?;
        if opp_count > 0 {
            return Err(AppError::business(format!(
                "客户存在 {} 个关联商机，无法删除",
                opp_count
            )));
        }

        // 3. 删除客户
        CustomerEntity::delete_by_id(customer_id)
            .exec(&*self.db)
            .await?;

        Ok(())
    }

    /// 列出客户的跟进记录
    pub async fn list_follow_ups(
        &self,
        customer_id: i32,
        page: u64,
        page_size: u64,
    ) -> Result<serde_json::Value, AppError> {
        let paginator = CustomerFollowupEntity::find()
            .filter(customer_followup::Column::CustomerId.eq(customer_id))
            .order_by(customer_followup::Column::FollowUpAt, sea_orm::Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items: Vec<customer_followup::Model> = paginator.fetch_page(page - 1).await?;
        Ok(serde_json::json!({
            "items": items,
            "total": total,
            "page": page,
            "page_size": page_size,
        }))
    }

    /// 创建跟进记录
    pub async fn create_follow_up(
        &self,
        customer_id: i32,
        user_id: i32,
        operator_name: String,
        req: crate::models::dto::crm_dto::FollowUpRequest,
    ) -> Result<serde_json::Value, AppError> {
        // 1. 验证客户存在
        let _customer = CustomerEntity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))?;

        // 2. 创建跟进记录
        let follow_up_type = req.r#type.clone().unwrap_or_else(|| "general".to_string());
        let content = req.content.clone().unwrap_or_default();
        let follow_up_at = chrono::Utc::now();
        let next_follow_up_at: Option<chrono::DateTime<chrono::Utc>> = req
            .next_follow_date
            .as_ref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .map(|d| d.and_hms_opt(0, 0, 0).unwrap().and_utc());

        let follow_up = customer_followup::ActiveModel {
            id: Set(uuid::Uuid::new_v4().to_string()),
            customer_id: Set(customer_id),
            follow_up_type: Set(follow_up_type),
            content: Set(content),
            follow_up_at: Set(follow_up_at),
            next_follow_up_at: Set(next_follow_up_at),
            notes: Set(Some(operator_name)),
            created_by: Set(Some(user_id)),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        }
        .insert(&*self.db)
        .await?;

        Ok(serde_json::to_value(follow_up)
            .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))?)
    }

    /// 计算 RFM 评分（R: 最近一次消费, F: 消费频率, M: 消费金额）
    /// 评分范围 1-5，3 个维度综合 = 平均分
    pub async fn compute_rfm_score(&self, customer_id: i32) -> Result<f64, AppError> {
        // R: Recency - 最近一次订单距今天数
        let recent_order = SalesOrderEntity::find()
            .filter(SalesOrderColumn::CustomerId.eq(customer_id))
            .order_by(SalesOrderColumn::CreatedAt, sea_orm::Order::Desc)
            .one(&*self.db)
            .await?;

        let r_score = if let Some(order) = recent_order {
            let days_since = (chrono::Utc::now() - order.created_at).num_days();
            match days_since {
                0..=30 => 5.0,
                31..=60 => 4.0,
                61..=90 => 3.0,
                91..=180 => 2.0,
                _ => 1.0,
            }
        } else {
            1.0
        };

        // F: Frequency - 历史订单数
        let order_count = SalesOrderEntity::find()
            .filter(SalesOrderColumn::CustomerId.eq(customer_id))
            .count(&*self.db)
            .await?;

        let f_score = match order_count {
            0 => 1.0,
            1..=2 => 2.0,
            3..=5 => 3.0,
            6..=10 => 4.0,
            _ => 5.0,
        };

        // M: Monetary - 总消费金额
        let orders = SalesOrderEntity::find()
            .filter(SalesOrderColumn::CustomerId.eq(customer_id))
            .all(&*self.db)
            .await?;
        let total_amount: f64 = orders
            .iter()
            .map(|o| o.total_amount.to_string().parse::<f64>().unwrap_or(0.0))
            .sum();

        let m_score = match total_amount {
            t if t >= 1_000_000.0 => 5.0,
            t if t >= 500_000.0 => 4.0,
            t if t >= 100_000.0 => 3.0,
            t if t >= 10_000.0 => 2.0,
            _ => 1.0,
        };

        Ok((r_score + f_score + m_score) / 3.0)
    }

    /// 获取 RFM 评分分布
    pub async fn get_rfm_distribution(&self) -> Result<serde_json::Value, AppError> {
        // 简化实现：返回示例分布
        Ok(serde_json::json!({
            "VIP": 0,
            "重要": 0,
            "一般": 0,
            "低价值": 0,
            "total_customers": 0,
            "note": "需要批量计算所有客户的 RFM 评分，请调用 compute_rfm_score 逐个计算"
        }))
    }
}

/// 引用 Arc 别名
#[allow(dead_code)]
pub(crate) type DbArc = Arc<DatabaseConnection>;

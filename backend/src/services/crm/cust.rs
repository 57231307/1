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
    crm_lead::Entity as CrmLeadEntity,
    crm_opportunity,
    crm_opportunity::Entity as CrmOpportunityEntity,
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
        // P2 3-25 修复：改用数据库聚合 sum/count/max，避免查所有订单/跟进后内存计算（大客户性能问题）
        use sea_orm::sea_query::Expr;

        // 统计商机数（线索不直接关联 customer_id，商机关联）
        let total_opportunities = CrmOpportunityEntity::find()
            .filter(crm_opportunity::Column::CustomerId.eq(customer_id))
            .count(&*self.db)
            .await? as i64;

        // 订单数 + 订单总金额（单次聚合查询，原为 all() 拉全表后内存 len()+sum()）
        let order_agg = SalesOrderEntity::find()
            .filter(SalesOrderColumn::CustomerId.eq(customer_id))
            .select_only()
            .column_as(Expr::col(SalesOrderColumn::Id).count(), "order_count")
            .column_as(
                Expr::col(SalesOrderColumn::TotalAmount).sum(),
                "total_amount",
            )
            .into_tuple::<(i64, Option<rust_decimal::Decimal>)>()
            .one(&*self.db)
            .await?;
        let (total_orders, total_order_amount) = order_agg.unwrap_or((0, None));

        // 跟进次数 + 最近跟进时间（单次聚合查询，原为 all() 拉全表后内存 len()+first()）
        let follow_up_agg = CustomerFollowupEntity::find()
            .filter(customer_followup::Column::CustomerId.eq(customer_id))
            .select_only()
            .column_as(
                Expr::col(customer_followup::Column::Id).count(),
                "follow_up_count",
            )
            .column_as(
                Expr::col(customer_followup::Column::FollowUpAt).max(),
                "last_interaction_at",
            )
            .into_tuple::<(i64, Option<chrono::DateTime<chrono::Utc>>)>()
            .one(&*self.db)
            .await?;
        let (follow_up_count, last_interaction_at) = follow_up_agg.unwrap_or((0, None));

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
        let items: Vec<customer_followup::Model> = paginator.fetch_page(page.saturating_sub(1)).await?;
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

        serde_json::to_value(follow_up)
            .map_err(|e| AppError::internal(format!("序列化失败: {}", e)))
    }

    /// 计算 RFM 评分（R: 最近一次消费, F: 消费频率, M: 消费金额）
    /// 评分范围 1-5，3 个维度综合 = 平均分
    pub async fn compute_rfm_score(&self, customer_id: i32) -> Result<f64, AppError> {
        // P2 3-23 修复：合并原 3 次独立查询（recent_order / count / all）为 1 次查询，内存计算 R/F/M
        let orders = SalesOrderEntity::find()
            .filter(SalesOrderColumn::CustomerId.eq(customer_id))
            .order_by(SalesOrderColumn::CreatedAt, sea_orm::Order::Desc)
            .all(&*self.db)
            .await?;

        // R: Recency - 最近一次订单距今天数（orders 已按 CreatedAt 倒序，first 即最近）
        let r_score = orders.first().map(|order| {
            let days_since = (chrono::Utc::now() - order.created_at).num_days();
            match days_since {
                0..=30 => 5.0,
                31..=60 => 4.0,
                61..=90 => 3.0,
                91..=180 => 2.0,
                _ => 1.0,
            }
        }).unwrap_or(1.0);

        // F: Frequency - 历史订单数
        let order_count = orders.len() as u64;
        let f_score = match order_count {
            0 => 1.0,
            1..=2 => 2.0,
            3..=5 => 3.0,
            6..=10 => 4.0,
            _ => 5.0,
        };

        // M: Monetary - 总消费金额
        // P2 3-24 修复：直接 Decimal 求和再转 f64，避免原 total_amount.to_string().parse::<f64>() 的精度丢失
        use rust_decimal::prelude::ToPrimitive;
        let total_amount_decimal: rust_decimal::Decimal =
            orders.iter().map(|o| o.total_amount).sum();
        let total_amount: f64 = total_amount_decimal.to_f64().unwrap_or(0.0);

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

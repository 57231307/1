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
    customer,
    customer::Entity as CustomerEntity,
    customer_followup,
    customer_followup::Entity as CustomerFollowupEntity,
    sales_order::{Column as SalesOrderColumn, Entity as SalesOrderEntity},
};
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{check_resource_owner, DataScopeContext};
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
    pub async fn get_customer_360(
        &self,
        customer_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<serde_json::Value, AppError> {
        // 客户基本信息
        let customer_info = CustomerEntity::find_by_id(customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))?;

        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // customer 表无 department_id，Dept 退化为 Self；
        // customer.created_by 是 Option<i32>，可能为 None（None 时 Self 范围拒绝访问）。
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, customer_info.created_by, None) {
                return Err(AppError::permission_denied(format!(
                    "无权访问客户 {} 的 360 视图（数据范围限制）", customer_id
                )));
            }
        }

        // 关联商机
        let opportunities: Vec<super::OpportunityBrief> = CrmOpportunityEntity::find()
            .filter(crm_opportunity::Column::CustomerId.eq(customer_id))
            .order_by(crm_opportunity::Column::CreatedAt, sea_orm::Order::Desc)
            .into_model::<super::OpportunityBrief>()
            .all(&*self.db)
            .await?;

        // 关联摘要（内部调用传 None，权限已在 customer_info 校验）
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
        data_scope: Option<&DataScopeContext>,
    ) -> Result<serde_json::Value, AppError> {
        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // 先校验用户是否有权访问该 customer，再返回其跟进记录。
        // customer 表无 department_id，Dept 退化为 Self。
        if let Some(ctx) = data_scope {
            let customer_info = CustomerEntity::find_by_id(customer_id)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", customer_id)))?;
            if !check_resource_owner(ctx, customer_info.created_by, None) {
                return Err(AppError::permission_denied(format!(
                    "无权访问客户 {} 的跟进记录（数据范围限制）", customer_id
                )));
            }
        }

        let paginator = CustomerFollowupEntity::find()
            .filter(customer_followup::Column::CustomerId.eq(customer_id))
            .order_by(customer_followup::Column::FollowUpAt, sea_orm::Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
        let items: Vec<customer_followup::Model> = paginator.fetch_page(page.clamp(1, 1000).saturating_sub(1)).await?;
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
        // P3 维度 3 修复（批次 87）：消除 expect panic，使用 unwrap_or_default 兜底
        // CI 修复：and_hms_opt 返回 Option<NaiveDateTime>，用 unwrap_or_default 替代
        // unwrap_or_else(T::default)（clippy::unwrap_or_default 建议）
        let next_follow_up_at: Option<chrono::DateTime<chrono::Utc>> = req
            .next_follow_date
            .as_ref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .map(|d| {
                d.and_hms_opt(0, 0, 0)
                    .unwrap_or_default()
                    .and_utc()
            });

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
    ///
    /// v14 P0-6 修复：真实批量计算所有客户的 RFM 评分并聚合分布
    /// - 一次性查询所有客户 ID + 订单聚合数据（避免 N+1 查询）
    /// - 在内存中按 compute_rfm_score 相同规则计算每个客户的 RFM 评分
    /// - 按评分分桶聚合：VIP(>=4.5) / 重要(>=3.5) / 一般(>=2.5) / 低价值(<2.5)
    pub async fn get_rfm_distribution(&self) -> Result<serde_json::Value, AppError> {
        use rust_decimal::prelude::ToPrimitive;
        use sea_orm::sea_query::Expr;
        use std::collections::HashMap;

        // 订单聚合行：(customer_id, order_count, last_order_at, total_amount)
        // 提取 type 别名避免 clippy type_complexity 警告
        type OrderAggRow = (
            i32,
            i64,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<rust_decimal::Decimal>,
        );
        // 客户订单统计：(order_count, last_order_at, total_amount_f64)
        type CustomerOrderStats = (i64, Option<chrono::DateTime<chrono::Utc>>, f64);

        // 1. 查询所有客户 ID（含无订单客户，评分 = 1.0）
        let customers: Vec<customer::Model> = CustomerEntity::find().all(&*self.db).await?;
        let customer_ids: Vec<i32> = customers.iter().map(|c| c.id).collect();

        // 2. 查询所有客户的订单聚合（按 customer_id 分组：订单数 + 最近订单时间 + 总金额）
        let order_aggs: Vec<OrderAggRow> = SalesOrderEntity::find()
            .select_only()
            .column(SalesOrderColumn::CustomerId)
            .column_as(Expr::col(SalesOrderColumn::Id).count(), "order_count")
            .column_as(
                Expr::col(SalesOrderColumn::CreatedAt).max(),
                "last_order_at",
            )
            .column_as(
                Expr::col(SalesOrderColumn::TotalAmount).sum(),
                "total_amount",
            )
            .group_by(SalesOrderColumn::CustomerId)
            .into_tuple()
            .all(&*self.db)
            .await?;

        // 3. 构建 customer_id -> CustomerOrderStats 映射
        let order_map: HashMap<i32, CustomerOrderStats> = order_aggs
            .into_iter()
            .map(|(cid, count, last_order, total)| {
                let total_f64 = total.and_then(|d| d.to_f64()).unwrap_or(0.0);
                (cid, (count, last_order, total_f64))
            })
            .collect();

        // 4. 计算每个客户的 RFM 评分并分桶（评分规则与 compute_rfm_score 完全一致）
        let mut vip_count = 0u64;
        let mut important_count = 0u64;
        let mut normal_count = 0u64;
        let mut low_value_count = 0u64;

        let now = chrono::Utc::now();
        for cid in &customer_ids {
            let (order_count, last_order_at, total_amount) =
                order_map.get(cid).copied().unwrap_or((0, None, 0.0));

            // R: Recency - 最近一次订单距今天数
            let r_score = last_order_at
                .map(|dt| {
                    let days_since = (now - dt).num_days();
                    match days_since {
                        0..=30 => 5.0,
                        31..=60 => 4.0,
                        61..=90 => 3.0,
                        91..=180 => 2.0,
                        _ => 1.0,
                    }
                })
                .unwrap_or(1.0);

            // F: Frequency - 历史订单数
            let f_score = match order_count {
                0 => 1.0,
                1..=2 => 2.0,
                3..=5 => 3.0,
                6..=10 => 4.0,
                _ => 5.0,
            };

            // M: Monetary - 总消费金额
            let m_score = match total_amount {
                t if t >= 1_000_000.0 => 5.0,
                t if t >= 500_000.0 => 4.0,
                t if t >= 100_000.0 => 3.0,
                t if t >= 10_000.0 => 2.0,
                _ => 1.0,
            };

            let score = (r_score + f_score + m_score) / 3.0;

            // 分桶（VIP >= 4.5 / 重要 >= 3.5 / 一般 >= 2.5 / 低价值 < 2.5）
            if score >= 4.5 {
                vip_count += 1;
            } else if score >= 3.5 {
                important_count += 1;
            } else if score >= 2.5 {
                normal_count += 1;
            } else {
                low_value_count += 1;
            }
        }

        Ok(serde_json::json!({
            "VIP": vip_count,
            "重要": important_count,
            "一般": normal_count,
            "低价值": low_value_count,
            "total_customers": customer_ids.len() as u64,
        }))
    }
}

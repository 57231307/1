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

/// 订单聚合行：(customer_id, order_count, last_order_at, total_amount)
type OrderAggRow = (
    i32,
    i64,
    Option<chrono::DateTime<chrono::Utc>>,
    Option<rust_decimal::Decimal>,
);

/// 客户订单统计：(order_count, last_order_at, total_amount_f64)
type CustomerOrderStats = (i64, Option<chrono::DateTime<chrono::Utc>>, f64);

/// RFM 分布分桶计数
#[derive(Default)]
struct RfmDistributionCounts {
    vip: u64,
    important: u64,
    normal: u64,
    low_value: u64,
}

impl RfmDistributionCounts {
    /// 按评分累加到对应分桶（VIP>=4.5 / 重要>=3.5 / 一般>=2.5 / 低价值<2.5）
    fn add_score(&mut self, score: f64) {
        if score >= 4.5 {
            self.vip += 1;
        } else if score >= 3.5 {
            self.important += 1;
        } else if score >= 2.5 {
            self.normal += 1;
        } else {
            self.low_value += 1;
        }
    }
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
                    "无权访问客户 {} 的 360 视图（数据范围限制）",
                    customer_id
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
                    "无权访问客户 {} 的跟进记录（数据范围限制）",
                    customer_id
                )));
            }
        }

        let paginator = CustomerFollowupEntity::find()
            .filter(customer_followup::Column::CustomerId.eq(customer_id))
            .order_by(customer_followup::Column::FollowUpAt, sea_orm::Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
        let items: Vec<customer_followup::Model> = paginator
            .fetch_page(page.clamp(1, 1000).saturating_sub(1))
            .await?;
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
            .map(|d| d.and_hms_opt(0, 0, 0).unwrap_or_default().and_utc());

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
        let r_score = orders
            .first()
            .map(|order| {
                let days_since = (chrono::Utc::now() - order.created_at).num_days();
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

    /// 获取 RFM 评分分布（查询所有客户的订单聚合（按 customer_id 分组：订单数 + 最近订单时间 + 总金额））
    async fn query_customer_order_aggregations(
        db: &DatabaseConnection,
    ) -> Result<Vec<OrderAggRow>, AppError> {
        use sea_orm::sea_query::Expr;
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
            .all(db)
            .await?;
        Ok(order_aggs)
    }

    /// 构建 customer_id -> CustomerOrderStats 映射
    fn build_customer_order_stats_map(
        order_aggs: Vec<OrderAggRow>,
    ) -> std::collections::HashMap<i32, CustomerOrderStats> {
        use rust_decimal::prelude::ToPrimitive;
        order_aggs
            .into_iter()
            .map(|(cid, count, last_order, total)| {
                let total_f64 = total.and_then(|d| d.to_f64()).unwrap_or(0.0);
                (cid, (count, last_order, total_f64))
            })
            .collect()
    }

    /// 计算单个客户的 RFM 评分（R+F+M 均值，规则与 compute_rfm_score 一致）
    fn compute_rfm_score_for_customer(
        order_count: i64,
        last_order_at: Option<chrono::DateTime<chrono::Utc>>,
        total_amount: f64,
        now: chrono::DateTime<chrono::Utc>,
    ) -> f64 {
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
        let f_score = match order_count {
            0 => 1.0,
            1..=2 => 2.0,
            3..=5 => 3.0,
            6..=10 => 4.0,
            _ => 5.0,
        };
        let m_score = match total_amount {
            t if t >= 1_000_000.0 => 5.0,
            t if t >= 500_000.0 => 4.0,
            t if t >= 100_000.0 => 3.0,
            t if t >= 10_000.0 => 2.0,
            _ => 1.0,
        };
        (r_score + f_score + m_score) / 3.0
    }

    /// 批量计算所有客户的 RFM 评分并聚合分布
    pub async fn get_rfm_distribution(&self) -> Result<serde_json::Value, AppError> {
        let customers: Vec<customer::Model> = CustomerEntity::find().all(&*self.db).await?;
        let customer_ids: Vec<i32> = customers.iter().map(|c| c.id).collect();
        let order_aggs = Self::query_customer_order_aggregations(&*self.db).await?;
        let order_map = Self::build_customer_order_stats_map(order_aggs);
        let now = chrono::Utc::now();
        let mut counts = RfmDistributionCounts::default();
        for cid in &customer_ids {
            let (order_count, last_order_at, total_amount) =
                order_map.get(cid).copied().unwrap_or((0, None, 0.0));
            let score =
                Self::compute_rfm_score_for_customer(order_count, last_order_at, total_amount, now);
            counts.add_score(score);
        }
        Ok(serde_json::json!({
            "VIP": counts.vip,
            "重要": counts.important,
            "一般": counts.normal,
            "低价值": counts.low_value,
            "total_customers": customer_ids.len() as u64,
        }))
    }

    /// V15 P2 18.4-D5: 获取客户字段权限配置
    pub async fn get_customer_field_permissions(
        &self,
        role_id: i32,
    ) -> Result<Vec<crate::models::customer_field_permission::Model>, AppError> {
        use crate::models::customer_field_permission;

        let permissions = customer_field_permission::Entity::find()
            .filter(customer_field_permission::Column::RoleId.eq(role_id))
            .all(&*self.db)
            .await?;

        Ok(permissions)
    }

    /// V15 P2 18.4-D5: 设置客户字段权限
    pub async fn set_customer_field_permission(
        &self,
        req: SetFieldPermissionRequest,
    ) -> Result<crate::models::customer_field_permission::Model, AppError> {
        use crate::models::customer_field_permission;

        // 检查是否已存在该角色+字段的权限配置
        let existing = customer_field_permission::Entity::find()
            .filter(customer_field_permission::Column::RoleId.eq(req.role_id))
            .filter(customer_field_permission::Column::FieldName.eq(&req.field_name))
            .one(&*self.db)
            .await?;

        if let Some(record) = existing {
            // 更新现有记录
            let mut active: customer_field_permission::ActiveModel = record.into();
            active.permission = sea_orm::Set(req.permission);
            active.mask_pattern = sea_orm::Set(req.mask_pattern);
            active.updated_at = sea_orm::Set(Some(chrono::Utc::now()));
            let updated = active.update(&*self.db).await?;
            Ok(updated)
        } else {
            // 创建新记录
            let new_record = customer_field_permission::ActiveModel {
                id: Default::default(),
                role_id: sea_orm::Set(req.role_id),
                field_name: sea_orm::Set(req.field_name),
                permission: sea_orm::Set(req.permission),
                mask_pattern: sea_orm::Set(req.mask_pattern),
                created_at: sea_orm::Set(Some(chrono::Utc::now())),
                updated_at: sea_orm::Set(Some(chrono::Utc::now())),
            }
            .insert(&*self.db)
            .await?;
            Ok(new_record)
        }
    }

    /// V15 P2 18.4-D6: 记录客户操作日志
    pub async fn log_customer_operation(
        &self,
        req: CreateAuditLogRequest,
    ) -> Result<(), AppError> {
        use crate::models::customer_audit_log;

        customer_audit_log::ActiveModel {
            id: Default::default(),
            customer_id: sea_orm::Set(req.customer_id),
            operation: sea_orm::Set(req.operation),
            field_name: sea_orm::Set(req.field_name),
            old_value: sea_orm::Set(req.old_value),
            new_value: sea_orm::Set(req.new_value),
            user_id: sea_orm::Set(req.user_id),
            user_name: sea_orm::Set(req.user_name),
            ip_address: sea_orm::Set(req.ip_address),
            user_agent: sea_orm::Set(req.user_agent),
            created_at: sea_orm::Set(Some(chrono::Utc::now())),
        }
        .insert(&*self.db)
        .await?;

        Ok(())
    }

    /// V15 P2 18.4-D6: 获取客户操作日志列表
    pub async fn list_customer_audit_logs(
        &self,
        customer_id: i32,
        operation: Option<&str>,
    ) -> Result<Vec<crate::models::customer_audit_log::Model>, AppError> {
        use crate::models::customer_audit_log;

        let mut q = customer_audit_log::Entity::find()
            .filter(customer_audit_log::Column::CustomerId.eq(customer_id));
        if let Some(op) = operation {
            q = q.filter(customer_audit_log::Column::Operation.eq(op));
        }
        let logs = q
            .order_by(customer_audit_log::Column::CreatedAt, sea_orm::Order::Desc)
            .all(&*self.db)
            .await?;

        Ok(logs)
    }

    /// V15 P2 18.5-D5: 计算客户全生命周期价值（CLV）
    pub async fn calculate_customer_clv(
        &self,
        customer_id: i32,
    ) -> Result<crate::models::customer_lifetime_value::Model, AppError> {
        use crate::models::{customer_lifetime_value, sales_order};

        // 获取客户所有订单
        let orders = sales_order::Entity::find()
            .filter(sales_order::Column::CustomerId.eq(customer_id))
            .order_by(sales_order::Column::CreatedAt, sea_orm::Order::Asc)
            .all(&*self.db)
            .await?;

        let total_orders = orders.len() as i32;
        let total_revenue: rust_decimal::Decimal = orders
            .iter()
            .map(|o| o.total_amount)
            .sum();
        let avg_order_value = if total_orders > 0 {
            total_revenue / rust_decimal::Decimal::from(total_orders)
        } else {
            rust_decimal::Decimal::ZERO
        };

        let first_order_date = orders.first().map(|o| o.created_at.date_naive());
        let last_order_date = orders.last().map(|o| o.created_at.date_naive());

        // 计算客户生命周期天数
        let lifespan_days = if let (Some(first), Some(last)) = (first_order_date, last_order_date) {
            (last - first).num_days() as i32
        } else {
            0
        };

        // 计算购买频率（订单数/年）
        let purchase_frequency = if lifespan_days > 0 && total_orders > 0 {
            let years = lifespan_days as f64 / 365.0;
            rust_decimal::Decimal::from(total_orders) / rust_decimal::Decimal::try_from(years).unwrap_or(rust_decimal::Decimal::ONE)
        } else {
            rust_decimal::Decimal::ZERO
        };

        // CLV = 平均订单金额 * 购买频率 * 客户生命周期年数
        let clv_score = avg_order_value * purchase_frequency * rust_decimal::Decimal::try_from(lifespan_days as f64 / 365.0).unwrap_or(rust_decimal::Decimal::ONE);

        // 客户分层
        let segment = if clv_score >= rust_decimal::Decimal::from(100000) {
            "champion"
        } else if clv_score >= rust_decimal::Decimal::from(50000) {
            "loyal"
        } else if clv_score >= rust_decimal::Decimal::from(10000) {
            "potential"
        } else if clv_score >= rust_decimal::Decimal::from(1000) {
            "at_risk"
        } else {
            "lost"
        };

        // 保存或更新 CLV 记录
        let existing = customer_lifetime_value::Entity::find()
            .filter(customer_lifetime_value::Column::CustomerId.eq(customer_id))
            .one(&*self.db)
            .await?;

        let clv_record = if let Some(record) = existing {
            let mut active: customer_lifetime_value::ActiveModel = record.into();
            active.total_orders = sea_orm::Set(total_orders);
            active.total_revenue = sea_orm::Set(total_revenue);
            active.avg_order_value = sea_orm::Set(avg_order_value);
            active.first_order_date = sea_orm::Set(first_order_date);
            active.last_order_date = sea_orm::Set(last_order_date);
            active.customer_lifespan_days = sea_orm::Set(lifespan_days);
            active.purchase_frequency = sea_orm::Set(purchase_frequency);
            active.clv_score = sea_orm::Set(clv_score);
            active.segment = sea_orm::Set(Some(segment.to_string()));
            active.calculated_at = sea_orm::Set(Some(chrono::Utc::now()));
            active.update(&*self.db).await?
        } else {
            let new_record = customer_lifetime_value::ActiveModel {
                id: Default::default(),
                customer_id: sea_orm::Set(customer_id),
                total_orders: sea_orm::Set(total_orders),
                total_revenue: sea_orm::Set(total_revenue),
                avg_order_value: sea_orm::Set(avg_order_value),
                first_order_date: sea_orm::Set(first_order_date),
                last_order_date: sea_orm::Set(last_order_date),
                customer_lifespan_days: sea_orm::Set(lifespan_days),
                purchase_frequency: sea_orm::Set(purchase_frequency),
                clv_score: sea_orm::Set(clv_score),
                segment: sea_orm::Set(Some(segment.to_string())),
                calculated_at: sea_orm::Set(Some(chrono::Utc::now())),
            }
            .insert(&*self.db)
            .await?;
            new_record
        };

        Ok(clv_record)
    }

    /// V15 P2 18.5-D5: 获取客户 CLV 信息
    pub async fn get_customer_clv(
        &self,
        customer_id: i32,
    ) -> Result<Option<crate::models::customer_lifetime_value::Model>, AppError> {
        use crate::models::customer_lifetime_value;

        let clv = customer_lifetime_value::Entity::find()
            .filter(customer_lifetime_value::Column::CustomerId.eq(customer_id))
            .one(&*self.db)
            .await?;

        Ok(clv)
    }
}

/// V15 P2 18.4-D5: 设置字段权限请求
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SetFieldPermissionRequest {
    pub role_id: i32,
    pub field_name: String,
    pub permission: String,
    pub mask_pattern: Option<String>,
}

/// V15 P2 18.4-D6: 创建操作日志请求
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateAuditLogRequest {
    pub customer_id: i32,
    pub operation: String,
    pub field_name: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub user_id: i32,
    pub user_name: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

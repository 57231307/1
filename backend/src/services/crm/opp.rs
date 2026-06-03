//! CRM 商机服务（crm/opp）
//!
//! 包含商机 CRUD、阶段流转、商机转订单等。
//! 拆分自原 `crm_service.rs`。

use crate::models::{customer, crm_opportunity, sales_order};
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Set,
};
use std::sync::Arc;
use uuid::Uuid;

use super::cust::CrmService;

impl CrmService {
    /// 创建商机
    pub async fn create_opportunity(
        &self,
        req: super::CreateOpportunityRequest,
        user_id: i32,
    ) -> Result<crm_opportunity::Model, AppError> {
        // 验证客户存在
        if let Some(cid) = req.customer_id {
            let _ = customer::Entity::find_by_id(cid)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", cid)))?;
        }

        let opportunity = crm_opportunity::ActiveModel {
            id: Set(Uuid::new_v4().to_string()),
            customer_id: Set(req.customer_id),
            name: Set(req.name),
            amount: Set(req.amount),
            stage: Set(req.stage.unwrap_or_else(|| "QUALIFICATION".to_string())),
            probability: Set(req.probability),
            expected_close_date: Set(req.expected_close_date),
            owner_id: Set(req.owner_id.or(Some(user_id))),
            source: Set(req.source),
            description: Set(req.description),
            lead_id: Set(req.lead_id),
            opportunity_status: Set(Some("OPEN".to_string())),
            created_by: Set(Some(user_id)),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        }
        .insert(&*self.db)
        .await?;

        Ok(opportunity)
    }

    /// 列出商机
    pub async fn list_opportunities(
        &self,
        page: u64,
        page_size: u64,
        stage: Option<String>,
        customer_id: Option<i32>,
    ) -> Result<(Vec<crm_opportunity::Model>, u64), AppError> {
        let mut query = crm_opportunity::Entity::find();

        if let Some(s) = stage {
            query = query.filter(crm_opportunity::Column::Stage.eq(s));
        }
        if let Some(cid) = customer_id {
            query = query.filter(crm_opportunity::Column::CustomerId.eq(cid));
        }

        let paginator = query
            .order_by(crm_opportunity::Column::CreatedAt, sea_orm::Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page - 1).await?;
        Ok((items, total))
    }

    /// 获取商机详情
    pub async fn get_opportunity(
        &self,
        opportunity_id: &str,
    ) -> Result<crm_opportunity::Model, AppError> {
        let opportunity = crm_opportunity::Entity::find_by_id(opportunity_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("商机 {} 不存在", opportunity_id)))?;
        Ok(opportunity)
    }

    /// 校验商机阶段流转合法性
    fn validate_opportunity_stage_transition(&self, current: &str, next: &str) -> Result<(), AppError> {
        let valid_next = match current {
            "QUALIFICATION" => vec!["NEEDS_ANALYSIS", "PROPOSAL"],
            "NEEDS_ANALYSIS" => vec!["PROPOSAL", "QUALIFICATION"],
            "PROPOSAL" => vec!["NEGOTIATION", "NEEDS_ANALYSIS"],
            "NEGOTIATION" => vec!["CLOSED_WON", "CLOSED_LOST", "PROPOSAL"],
            _ => vec![],
        };

        if !valid_next.contains(&next) && current != next {
            return Err(AppError::business(format!(
                "商机阶段不允许从 {} 流转到 {}",
                current, next
            )));
        }
        Ok(())
    }

    /// 更新商机
    pub async fn update_opportunity(
        &self,
        opportunity_id: &str,
        req: super::UpdateOpportunityRequest,
        user_id: i32,
    ) -> Result<crm_opportunity::Model, AppError> {
        let opportunity = self.get_opportunity(opportunity_id).await?;

        // 关闭后的商机不能修改
        if let Some(status) = &opportunity.opportunity_status {
            if status == "CLOSED_WON" || status == "CLOSED_LOST" {
                return Err(AppError::business("已关闭的商机不能修改".to_string()));
            }
        }

        // 权限检查
        if let Some(owner) = opportunity.owner_id {
            if owner != user_id {
                return Err(AppError::permission_denied(
                    "只能修改自己负责的商机".to_string(),
                ));
            }
        }

        let mut opportunity_active: crm_opportunity::ActiveModel = opportunity.clone().into();

        if let Some(name) = req.name {
            opportunity_active.name = Set(name);
        }
        if let Some(amount) = req.amount {
            opportunity_active.amount = Set(Some(amount));
        }
        if let Some(stage) = req.stage.clone() {
            self.validate_opportunity_stage_transition(&opportunity.stage, &stage)?;
            opportunity_active.stage = Set(stage);
        }
        if let Some(probability) = req.probability {
            opportunity_active.probability = Set(Some(probability));
        }
        if let Some(expected_close_date) = req.expected_close_date {
            opportunity_active.expected_close_date = Set(Some(expected_close_date));
        }
        if let Some(owner_id) = req.owner_id {
            opportunity_active.owner_id = Set(Some(owner_id));
        }
        if let Some(description) = req.description {
            opportunity_active.description = Set(Some(description));
        }

        opportunity_active.updated_at = Set(chrono::Utc::now());

        let opportunity = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            opportunity_active,
            Some(0),
        )
        .await?;

        Ok(opportunity)
    }

    /// 删除商机
    pub async fn delete_opportunity(
        &self,
        opportunity_id: &str,
        user_id: i32,
    ) -> Result<(), AppError> {
        let opportunity = self.get_opportunity(opportunity_id).await?;

        if let Some(owner) = opportunity.owner_id {
            if owner != user_id {
                return Err(AppError::permission_denied(
                    "只能删除自己负责的商机".to_string(),
                ));
            }
        }

        // 已赢单的商机不能删除
        if let Some(status) = &opportunity.opportunity_status {
            if status == "CLOSED_WON" {
                return Err(AppError::business("已赢单的商机不能删除".to_string()));
            }
        }

        crm_opportunity::Entity::delete_by_id(opportunity_id)
            .exec(&*self.db)
            .await?;

        Ok(())
    }

    /// 商机转订单（赢单流程）
    pub async fn convert_opportunity_to_order(
        &self,
        opportunity_id: &str,
        user_id: i32,
    ) -> Result<i32, AppError> {
        let opportunity = self.get_opportunity(opportunity_id).await?;

        if let Some(status) = &opportunity.opportunity_status {
            if status == "CLOSED_WON" {
                return Err(AppError::business("商机已赢单".to_string()));
            }
        }

        // 校验：商机必须有关联客户
        let customer_id = opportunity
            .customer_id
            .ok_or_else(|| AppError::business("商机必须关联客户才能转订单".to_string()))?;

        let txn = (*self.db).begin().await?;

        // 1. 创建销售订单（草稿状态）
        let order = sales_order::ActiveModel {
            id: Default::default(),
            order_no: Set(format!("SO-TEMP-{}", chrono::Utc::now().timestamp())),
            customer_id: Set(customer_id),
            opportunity_id: Set(Some(opportunity_id.to_string())),
            order_date: Set(chrono::Utc::now()),
            required_date: Set(chrono::Utc::now() + chrono::Duration::days(30)),
            status: Set("draft".to_string()),
            subtotal: Set(rust_decimal::Decimal::ZERO),
            tax_amount: Set(rust_decimal::Decimal::ZERO),
            discount_amount: Set(rust_decimal::Decimal::ZERO),
            shipping_cost: Set(rust_decimal::Decimal::ZERO),
            total_amount: Set(opportunity.amount.unwrap_or(rust_decimal::Decimal::ZERO)),
            paid_amount: Set(rust_decimal::Decimal::ZERO),
            balance_amount: Set(opportunity.amount.unwrap_or(rust_decimal::Decimal::ZERO)),
            notes: Set(Some(format!(
                "从商机自动创建: {} - 预期金额: {:?}",
                opportunity.name, opportunity.amount
            ))),
            created_by: Set(Some(user_id)),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // 2. 更新商机状态
        let mut opp_active: crm_opportunity::ActiveModel = opportunity.into();
        opp_active.opportunity_status = Set(Some("CLOSED_WON".to_string()));
        opp_active.stage = Set("CLOSED_WON".to_string());
        opp_active.actual_amount = Set(opp_active.amount.clone().take());
        opp_active.actual_close_date = Set(Some(chrono::Utc::now().date_naive()));
        opp_active.updated_at = Set(chrono::Utc::now());
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            opp_active,
            Some(0),
        )
        .await?;

        // 3. 提交事务
        txn.commit().await?;

        Ok(order.id)
    }

    /// 订单完成后回调商机（更新实际金额、关单）
    pub async fn update_opportunity_on_order_complete(
        &self,
        opportunity_id: &str,
        actual_amount: rust_decimal::Decimal,
    ) -> Result<(), AppError> {
        let opportunity = self.get_opportunity(opportunity_id).await?;

        let mut opp_active: crm_opportunity::ActiveModel = opportunity.into();
        opp_active.actual_amount = Set(Some(actual_amount));
        opp_active.opportunity_status = Set(Some("CLOSED_WON".to_string()));
        opp_active.stage = Set("CLOSED_WON".to_string());
        opp_active.actual_close_date = Set(Some(chrono::Utc::now().date_naive()));
        opp_active.updated_at = Set(chrono::Utc::now());

        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            opp_active,
            Some(0),
        )
        .await?;

        Ok(())
    }
}

/// 引用 Arc 别名
#[allow(dead_code)]
pub(crate) type DbArc = Arc<DatabaseConnection>;

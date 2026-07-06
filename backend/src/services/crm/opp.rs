//! CRM 商机服务（crm/opp）
//!
//! 包含商机 CRUD、阶段流转、商机转订单等。
//! 拆分自原 `crm_service.rs`。

use crate::models::{crm_opportunity, customer, sales_order};
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    Set, TransactionTrait,
};

use super::cust::CrmService;

/// CSV 字段转义：字段包含逗号/引号/换行时用双引号包裹，内部引号双写
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

impl CrmService {
    /// 创建商机
    pub async fn create_opportunity(
        &self,
        req: crate::models::dto::crm_dto::CreateOpportunityRequest,
        user_id: i32,
    ) -> Result<crm_opportunity::Model, AppError> {
        // 验证客户存在（批次 98 P2-C 修复 v5 复审：去掉冗余 let _ = ，明确父级校验已通过 ? 传播错误）
        customer::Entity::find_by_id(req.customer_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("客户 {} 不存在", req.customer_id)))?;

        let opportunity_no = req
            .opportunity_no
            .unwrap_or_else(|| format!("OPP{}", chrono::Utc::now().format("%Y%m%d%H%M%S")));
        let opportunity_name = req.opportunity_name.clone();
        let opportunity_stage = req
            .opportunity_stage
            .clone()
            .unwrap_or_else(|| "QUALIFICATION".to_string());
        let owner_id = user_id;
        let owner_name = format!("用户{}", user_id);
        let now = chrono::Utc::now();

        let opportunity = crm_opportunity::ActiveModel {
            id: Default::default(),
            opportunity_no: Set(opportunity_no),
            opportunity_name: Set(opportunity_name),
            customer_id: Set(req.customer_id),
            lead_id: Set(req.lead_id),
            opportunity_type: Set(req.opportunity_type),
            opportunity_stage: Set(Some(opportunity_stage)),
            win_probability: Set(req.win_probability),
            estimated_amount: Set(req.estimated_amount),
            actual_amount: Set(req.actual_amount),
            currency: Set(req.currency),
            expected_close_date: Set(req.expected_close_date),
            actual_close_date: Set(req.actual_close_date),
            product_ids: Set(req.product_ids),
            product_names: Set(req.product_names),
            product_desc: Set(req.product_desc),
            owner_id: Set(owner_id),
            owner_name: Set(owner_name),
            opportunity_status: Set(Some("OPEN".to_string())),
            priority: Set(req.priority),
            rating: Set(req.rating),
            tags: Set(req.tags),
            created_by: Set(Some(user_id)),
            created_at: Set(Some(now)),
            updated_at: Set(Some(now)),
            ..Default::default()
        }
        .insert(&*self.db)
        .await?;

        Ok(opportunity)
    }

    /// 列出商机（返回分页结果）
    pub async fn list_opportunities(
        &self,
        query: crate::models::dto::crm_dto::OpportunityQuery,
    ) -> Result<serde_json::Value, AppError> {
        let page = query.page.unwrap_or(1).clamp(1, 1000);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100); // v10 P2-3 修复：crm 模块统一 clamp(1,100) 防 DoS

        let mut q = crm_opportunity::Entity::find();

        if let Some(s) = query.opportunity_stage {
            q = q.filter(crm_opportunity::Column::OpportunityStage.eq(s));
        }

        let paginator = q
            .order_by(crm_opportunity::Column::CreatedAt, sea_orm::Order::Desc)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        // 批次 98 P2-A 修复（v5 复审）：page clamp 防 DoS
        let items: Vec<crm_opportunity::Model> = paginator.fetch_page(page.clamp(1, 1000).saturating_sub(1)).await?;

        Ok(serde_json::json!({
            "data": items,
            "total": total,
            "page": page,
            "page_size": page_size,
        }))
    }

    /// 导出商机为 CSV（UTF-8 BOM，Excel 兼容）
    ///
    /// v11 批次 141 新增：前端 exportOpportunities API 真实接入，原缺失导出路由。
    /// 查询所有匹配条件（不分页）的商机，生成 CSV 字符串。
    /// 导出字段：商机编号/商机名称/客户ID/商机阶段/预估金额/实际金额/预期成交日期/负责人/创建时间
    pub async fn export_opportunities(
        &self,
        query: crate::models::dto::crm_dto::OpportunityQuery,
    ) -> Result<String, AppError> {
        let mut q = crm_opportunity::Entity::find();

        if let Some(s) = query.opportunity_stage {
            q = q.filter(crm_opportunity::Column::OpportunityStage.eq(s));
        }

        // 限制导出最大 10000 条，防止 DoS
        let opportunities: Vec<crm_opportunity::Model> = q
            .order_by(crm_opportunity::Column::CreatedAt, sea_orm::Order::Desc)
            .limit(10000)
            .all(&*self.db)
            .await?;

        // UTF-8 BOM（Excel 正确识别中文）
        let mut csv = String::from("\u{FEFF}");
        csv.push_str("商机编号,商机名称,客户ID,商机阶段,预估金额,实际金额,预期成交日期,实际成交日期,负责人,优先级,创建时间\n");

        for opp in opportunities {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{}\n",
                csv_escape(&opp.opportunity_no),
                csv_escape(&opp.opportunity_name),
                opp.customer_id,
                csv_escape(opp.opportunity_stage.as_deref().unwrap_or("")),
                opp.estimated_amount.map(|d| d.to_string()).unwrap_or_default(),
                opp.actual_amount.map(|d| d.to_string()).unwrap_or_default(),
                opp.expected_close_date
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                opp.actual_close_date
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
                csv_escape(&opp.owner_name),
                csv_escape(opp.priority.as_deref().unwrap_or("")),
                opp.created_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
            ));
        }

        Ok(csv)
    }

    /// 获取商机详情
    pub async fn get_opportunity(
        &self,
        opportunity_id: i32,
    ) -> Result<crm_opportunity::Model, AppError> {
        let opportunity = crm_opportunity::Entity::find_by_id(opportunity_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("商机 {} 不存在", opportunity_id)))?;
        Ok(opportunity)
    }

    /// 校验商机阶段流转合法性
    fn validate_opportunity_stage_transition(
        &self,
        current: &Option<String>,
        next: &str,
    ) -> Result<(), AppError> {
        let current_str = current.clone().unwrap_or_default();
        let valid_next = match current_str.as_str() {
            "QUALIFICATION" => vec!["NEEDS_ANALYSIS", "PROPOSAL"],
            "NEEDS_ANALYSIS" => vec!["PROPOSAL", "QUALIFICATION"],
            "PROPOSAL" => vec!["NEGOTIATION", "NEEDS_ANALYSIS"],
            "NEGOTIATION" => vec!["CLOSED_WON", "CLOSED_LOST", "PROPOSAL"],
            _ => vec![],
        };

        if !valid_next.contains(&next) && current_str != next {
            return Err(AppError::business(format!(
                "商机阶段不允许从 {} 流转到 {}",
                current_str, next
            )));
        }
        Ok(())
    }

    /// 更新商机
    pub async fn update_opportunity(
        &self,
        opportunity_id: i32,
        req: crate::models::dto::crm_dto::UpdateOpportunityRequest,
        user_id: i32,
    ) -> Result<crm_opportunity::Model, AppError> {
        let opportunity = self.get_opportunity(opportunity_id).await?;

        // 关闭后的商机不能修改
        if let Some(status) = &opportunity.opportunity_status {
            if status == "CLOSED_WON" || status == "CLOSED_LOST" {
                return Err(AppError::business("已关闭的商机不能修改".to_string()));
            }
        }

        let mut opportunity_active: crm_opportunity::ActiveModel = opportunity.into();

        if let Some(v) = req.opportunity_name {
            opportunity_active.opportunity_name = Set(v);
        }
        if let Some(v) = req.customer_id {
            opportunity_active.customer_id = Set(v);
        }
        if let Some(v) = req.lead_id {
            opportunity_active.lead_id = Set(Some(v));
        }
        if let Some(v) = req.opportunity_type {
            opportunity_active.opportunity_type = Set(Some(v));
        }
        if let Some(v) = req.opportunity_stage.clone() {
            self.validate_opportunity_stage_transition(
                opportunity_active.opportunity_stage.as_ref(),
                &v,
            )?;
            opportunity_active.opportunity_stage = Set(Some(v));
        }
        if let Some(v) = req.win_probability {
            opportunity_active.win_probability = Set(Some(v));
        }
        if let Some(v) = req.estimated_amount {
            opportunity_active.estimated_amount = Set(Some(v));
        }
        if let Some(v) = req.actual_amount {
            opportunity_active.actual_amount = Set(Some(v));
        }
        if let Some(v) = req.currency {
            opportunity_active.currency = Set(Some(v));
        }
        if let Some(v) = req.expected_close_date {
            opportunity_active.expected_close_date = Set(Some(v));
        }
        if let Some(v) = req.actual_close_date {
            opportunity_active.actual_close_date = Set(Some(v));
        }
        if let Some(v) = req.product_ids {
            opportunity_active.product_ids = Set(Some(v));
        }
        if let Some(v) = req.product_names {
            opportunity_active.product_names = Set(Some(v));
        }
        if let Some(v) = req.product_desc {
            opportunity_active.product_desc = Set(Some(v));
        }
        if let Some(v) = req.priority {
            opportunity_active.priority = Set(Some(v));
        }
        if let Some(v) = req.rating {
            opportunity_active.rating = Set(Some(v));
        }
        if let Some(v) = req.tags {
            opportunity_active.tags = Set(Some(v));
        }

        opportunity_active.updated_at = Set(Some(chrono::Utc::now()));

        let opportunity = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            opportunity_active,
            // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
        )
        .await?;

        Ok(opportunity)
    }

    /// 删除商机
    pub async fn delete_opportunity(
        &self,
        opportunity_id: i32,
        user_id: i32,
    ) -> Result<(), AppError> {
        let opportunity = self.get_opportunity(opportunity_id).await?;

        if let Some(status) = &opportunity.opportunity_status {
            if status == "CLOSED_WON" {
                return Err(AppError::business("已赢单的商机不能删除".to_string()));
            }
        }

        // P0 8-3 修复：delete 操作补审计日志
        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        crate::services::audit_log_service::AuditLogService::delete_with_audit::<
            crm_opportunity::Entity,
            _,
        >(&*self.db, "crm_opportunity", opportunity_id, Some(user_id))
        .await
    }

    /// 商机转订单（赢单流程）
    pub async fn convert_opportunity_to_order(
        &self,
        opportunity_id: i32,
        user_id: i32,
    ) -> Result<serde_json::Value, AppError> {
        let opportunity = self.get_opportunity(opportunity_id).await?;

        if let Some(status) = &opportunity.opportunity_status {
            if status == "CLOSED_WON" {
                return Err(AppError::business("商机已赢单".to_string()));
            }
        }

        // 校验：商机必须有关联客户
        let customer_id = opportunity.customer_id;

        let txn = self.db.begin().await?;

        // 1. 创建销售订单（草稿状态）
        let order_no = format!("SO-TEMP-{}", chrono::Utc::now().timestamp());
        let total_amount = opportunity
            .estimated_amount
            .unwrap_or(rust_decimal::Decimal::ZERO);
        let order = sales_order::ActiveModel {
            id: Default::default(),
            order_no: Set(order_no.clone()),
            customer_id: Set(customer_id),
            opportunity_id: Set(Some(opportunity_id)),
            order_date: Set(chrono::Utc::now()),
            required_date: Set(chrono::Utc::now() + chrono::Duration::days(30)),
            ship_date: Set(None),
            status: Set("draft".to_string()),
            subtotal: Set(rust_decimal::Decimal::ZERO),
            tax_amount: Set(rust_decimal::Decimal::ZERO),
            discount_amount: Set(rust_decimal::Decimal::ZERO),
            shipping_cost: Set(rust_decimal::Decimal::ZERO),
            total_amount: Set(total_amount),
            paid_amount: Set(rust_decimal::Decimal::ZERO),
            balance_amount: Set(total_amount),
            shipping_address: Set(None),
            billing_address: Set(None),
            notes: Set(Some(format!(
                "从商机自动创建: {} - 预期金额: {:?}",
                opportunity.opportunity_name, opportunity.estimated_amount
            ))),
            created_by: Set(Some(user_id)),
            approved_by: Set(None),
            approved_at: Set(None),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
        }
        .insert(&txn)
        .await?;

        // 2. 更新商机状态
        let mut opp_active: crm_opportunity::ActiveModel = opportunity.into();
        opp_active.opportunity_status = Set(Some("CLOSED_WON".to_string()));
        opp_active.opportunity_stage = Set(Some("CLOSED_WON".to_string()));
        // 估算金额 -> 实际金额：解包 ActiveValue
        let estimated: Option<rust_decimal::Decimal> = match opp_active.estimated_amount {
            sea_orm::ActiveValue::Set(v) => v,
            _ => None,
        };
        opp_active.estimated_amount = Set(None);
        opp_active.actual_amount = Set(estimated);
        opp_active.actual_close_date = Set(Some(chrono::Utc::now().date_naive()));
        opp_active.updated_at = Set(Some(chrono::Utc::now()));
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            opp_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        // 3. 提交事务
        txn.commit().await?;

        Ok(serde_json::json!({
            "order_id": order.id,
            "order_no": order.order_no,
        }))
    }
}

//! CRM 商机服务（crm/opp）
//!
//! 包含商机 CRUD、阶段流转、商机转订单等。
//! 拆分自原 `crm_service.rs`。
//!
//! V15 P0-B08（Batch 482）：赢率自动计算 — 按阶段配置默认赢率，
//! 创建/更新商机时若用户未传 win_probability 则按阶段自动填充，
//! 阶段流转时自动重算赢率（用户显式传值时仍可覆盖默认值）。
//!
//! V15 P0-B09（Batch 482）：输单原因记录 — 新增 close_as_lost 方法，
//! 商机转 CLOSED_LOST 时强制要求 lost_reason 字段写入。

use crate::models::{crm_opportunity, customer, sales_order};
// 批次 236 v13 P1-1：商机状态常量接入（规则 0）
use crate::models::status::crm_opportunity as opp_status;
// V15 P0-S01：行级数据权限工具
use crate::utils::data_scope::{apply_data_scope, check_resource_owner, DataScopeContext};
use crate::utils::error::AppError;
use crate::utils::xlsx_export::XlsxTable;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    Set, TransactionTrait,
};

use super::cust::CrmService;

// V15 P0-B08：阶段默认赢率（百分比，0-100）
// 设计依据：审计报告 §18.2-D1 建议 QUALIFICATION 10% / NEGOTIATION 50% / CLOSED_WON 100%
// 5 个阶段全部覆盖，CLOSED_LOST 固定为 0
//
// 注意：rust_decimal 1.42 中 `Decimal::new` 不是 `const fn`（仅 `Decimal::ZERO`/
// `Decimal::ONE`/`Decimal::TEN`/`Decimal::ONE_HUNDRED` 等为 const），
// 故阶段赢率通过 `fn default_win_probability_by_stage` 内联返回，不声明为 const。
// 参考：批次 481 `budget_overrun_amount_threshold()` 同样使用 `fn` 而非 `const`。

/// 按商机阶段返回默认赢率（百分比 0-100）
///
/// V15 P0-B08：赢率自动计算
/// - QUALIFICATION（资质确认）→ 10%
/// - NEEDS_ANALYSIS（需求分析）→ 25%
/// - PROPOSAL（方案报价）→ 40%
/// - NEGOTIATION（谈判议价）→ 50%
/// - CLOSED_WON（赢单）→ 100%
/// - CLOSED_LOST（输单）→ 0%
/// - 其他/空 → None（无法自动计算）
fn default_win_probability_by_stage(stage: &str) -> Option<Decimal> {
    match stage {
        "QUALIFICATION" => Some(Decimal::new(10, 0)),
        "NEEDS_ANALYSIS" => Some(Decimal::new(25, 0)),
        "PROPOSAL" => Some(Decimal::new(40, 0)),
        "NEGOTIATION" => Some(Decimal::new(50, 0)),
        // Decimal::ONE_HUNDRED / Decimal::ZERO 为 const，可直接使用
        opp_status::CLOSED_WON => Some(Decimal::ONE_HUNDRED),
        opp_status::CLOSED_LOST => Some(Decimal::ZERO),
        _ => None,
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

        // V15 P0-B08：赢率自动计算
        // 用户未传 win_probability 时，按阶段默认赢率填充；显式传值时保留用户输入
        let win_probability = req.win_probability.or_else(|| default_win_probability_by_stage(&opportunity_stage));

        let opportunity = crm_opportunity::ActiveModel {
            id: Default::default(),
            opportunity_no: Set(opportunity_no),
            opportunity_name: Set(opportunity_name),
            customer_id: Set(req.customer_id),
            lead_id: Set(req.lead_id),
            opportunity_type: Set(req.opportunity_type),
            opportunity_stage: Set(Some(opportunity_stage)),
            win_probability: Set(win_probability),
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
        data_scope: Option<&DataScopeContext>,
    ) -> Result<serde_json::Value, AppError> {
        let page = query.page.unwrap_or(1).clamp(1, 1000);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100); // v10 P2-3 修复：crm 模块统一 clamp(1,100) 防 DoS

        let mut q = crm_opportunity::Entity::find();

        if let Some(s) = query.opportunity_stage {
            q = q.filter(crm_opportunity::Column::OpportunityStage.eq(s));
        }

        // V15 P0-S01：行级数据权限过滤
        // crm_opportunity 表无 department_id，Dept 退化为 Self；
        // CRM 业务数据权限语义为"我负责的商机"，使用 owner_id（i32 必填）作为 owner_column。
        if let Some(ctx) = data_scope {
            q = apply_data_scope(
                q,
                ctx,
                crm_opportunity::Column::OwnerId,
                crm_opportunity::Column::OwnerId, // 无 department_id，Dept 退化为 Self，复用 owner_id
            );
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

    /// 导出商机为 xlsx（v11 批次 142 升级：CSV → xlsx，规则 3 强制要求）
    ///
    /// v11 批次 141 新增：前端 exportOpportunities API 真实接入。
    /// v11 批次 142 升级：导出格式从 CSV 升级为 xlsx（Excel 标准格式）。
    /// 查询所有匹配条件（不分页）的商机，生成 XlsxTable。
    /// 导出字段：商机编号/商机名称/客户ID/商机阶段/预估金额/实际金额/预期成交日期/实际成交日期/负责人/优先级/创建时间
    pub async fn export_opportunities(
        &self,
        query: crate::models::dto::crm_dto::OpportunityQuery,
    ) -> Result<XlsxTable, AppError> {
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

        let headers = vec![
            "商机编号".to_string(),
            "商机名称".to_string(),
            "客户ID".to_string(),
            "商机阶段".to_string(),
            "预估金额".to_string(),
            "实际金额".to_string(),
            "预期成交日期".to_string(),
            "实际成交日期".to_string(),
            "负责人".to_string(),
            "优先级".to_string(),
            "创建时间".to_string(),
        ];

        let rows: Vec<Vec<String>> = opportunities
            .iter()
            .map(|opp| {
                vec![
                    opp.opportunity_no.clone(),
                    opp.opportunity_name.clone(),
                    opp.customer_id.to_string(),
                    opp.opportunity_stage.clone().unwrap_or_default(),
                    opp.estimated_amount.map(|d| d.to_string()).unwrap_or_default(),
                    opp.actual_amount.map(|d| d.to_string()).unwrap_or_default(),
                    opp.expected_close_date
                        .map(|d| d.to_string())
                        .unwrap_or_default(),
                    opp.actual_close_date
                        .map(|d| d.to_string())
                        .unwrap_or_default(),
                    opp.owner_name.clone(),
                    opp.priority.clone().unwrap_or_default(),
                    opp.created_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
                ]
            })
            .collect();

        Ok(XlsxTable {
            sheet_name: "商机列表".to_string(),
            headers,
            rows,
        })
    }

    /// 获取商机详情
    pub async fn get_opportunity(
        &self,
        opportunity_id: i32,
        data_scope: Option<&DataScopeContext>,
    ) -> Result<crm_opportunity::Model, AppError> {
        let opportunity = crm_opportunity::Entity::find_by_id(opportunity_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("商机 {} 不存在", opportunity_id)))?;
        // V15 P0-S01：行级数据权限校验（IDOR 防护）
        // crm_opportunity 表无 department_id，Dept 退化为 Self；
        // 使用 owner_id（业务负责人）作为归属判定字段。
        if let Some(ctx) = data_scope {
            if !check_resource_owner(ctx, Some(opportunity.owner_id), None) {
                return Err(AppError::permission_denied(format!(
                    "无权访问商机 {}（数据范围限制）", opportunity_id
                )));
            }
        }
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
            "NEGOTIATION" => vec![opp_status::CLOSED_WON, opp_status::CLOSED_LOST, "PROPOSAL"],
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
        let opportunity = self.get_opportunity(opportunity_id, None).await?;

        // 关闭后的商机不能修改
        if let Some(status) = &opportunity.opportunity_status {
            if status == opp_status::CLOSED_WON || status == opp_status::CLOSED_LOST {
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
            // V15 P0-B08：阶段流转时自动重算赢率
            // 用户未显式传 win_probability 时，按新阶段的默认赢率填充
            // 若用户同时传了 win_probability，下方 req.win_probability 分支会覆盖此默认值
            // 注意：需在 Set(Some(v)) 移动 v 之前计算默认赢率
            let default_prob = if req.win_probability.is_none() {
                default_win_probability_by_stage(&v)
            } else {
                None
            };
            opportunity_active.opportunity_stage = Set(Some(v));
            if let Some(prob) = default_prob {
                opportunity_active.win_probability = Set(Some(prob));
            }
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
        let opportunity = self.get_opportunity(opportunity_id, None).await?;

        if let Some(status) = &opportunity.opportunity_status {
            if status == opp_status::CLOSED_WON {
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
        let opportunity = self.get_opportunity(opportunity_id, None).await?;

        if let Some(status) = &opportunity.opportunity_status {
            if status == opp_status::CLOSED_WON {
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
        opp_active.opportunity_status = Set(Some(opp_status::CLOSED_WON.to_string()));
        opp_active.opportunity_stage = Set(Some(opp_status::CLOSED_WON.to_string()));
        // V15 P0-B08：赢单时赢率自动设为 100%
        opp_active.win_probability = Set(Some(Decimal::ONE_HUNDRED));
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

    /// 关单（输单流程）— V15 P0-B09（Batch 482）
    ///
    /// 将商机状态置为 CLOSED_LOST，强制要求写入流失原因 lost_reason。
    /// 设计依据：审计报告 §18.2-D2 — 输单原因未记录，销售改进无依据
    ///
    /// 业务规则：
    /// 1. 商机当前状态不能是 CLOSED_WON / CLOSED_LOST（已关闭不可重复关单）
    /// 2. lost_reason 必填且非空（保证销售改进有依据）
    /// 3. 阶段置为 CLOSED_LOST，状态置为 CLOSED_LOST
    /// 4. 赢率自动置为 0（V15 P0-B08 联动）
    /// 5. 实际关闭日期置为今天
    pub async fn close_as_lost(
        &self,
        opportunity_id: i32,
        lost_reason: String,
        user_id: i32,
    ) -> Result<crm_opportunity::Model, AppError> {
        // 流失原因必填校验（非空字符串）
        let lost_reason_trimmed = lost_reason.trim().to_string();
        if lost_reason_trimmed.is_empty() {
            return Err(AppError::validation("输单原因不能为空"));
        }
        if lost_reason_trimmed.chars().count() > 500 {
            return Err(AppError::validation("输单原因长度不能超过 500 字符"));
        }

        let opportunity = self.get_opportunity(opportunity_id, None).await?;

        // 已关闭的商机不能再关单
        if let Some(status) = &opportunity.opportunity_status {
            if status == opp_status::CLOSED_WON {
                return Err(AppError::business("已赢单的商机不能转为输单".to_string()));
            }
            if status == opp_status::CLOSED_LOST {
                return Err(AppError::business("商机已输单，不能重复关单".to_string()));
            }
        }

        let mut opp_active: crm_opportunity::ActiveModel = opportunity.into();
        opp_active.opportunity_status = Set(Some(opp_status::CLOSED_LOST.to_string()));
        opp_active.opportunity_stage = Set(Some(opp_status::CLOSED_LOST.to_string()));
        opp_active.win_probability = Set(Some(Decimal::ZERO));
        opp_active.lost_reason = Set(Some(lost_reason_trimmed));
        opp_active.actual_close_date = Set(Some(chrono::Utc::now().date_naive()));
        opp_active.updated_at = Set(Some(chrono::Utc::now()));

        let opportunity = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            opp_active,
            Some(user_id),
        )
        .await?;

        Ok(opportunity)
    }
}

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
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
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
/// V15 P0-B08：赢率自动计算；QUALIFICATION（资质确认）→ 10%；NEEDS_ANALYSIS（需求分析）→ 25%；PROPOSAL（方案报价）→ 40%；NEGOTIATION（谈判议价）→ 50%；CLOSED_WON（赢单）→ 100%；CLOSED_LOST（输单）→ 0%；其他/空 → None（无法自动计算）
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
        let win_probability = req
            .win_probability
            .or_else(|| default_win_probability_by_stage(&opportunity_stage));

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
        let items: Vec<crm_opportunity::Model> = paginator
            .fetch_page(page.clamp(1, 1000).saturating_sub(1))
            .await?;

        Ok(serde_json::json!({
            "data": items,
            "total": total,
            "page": page,
            "page_size": page_size,
        }))
    }

    /// 导出商机为 xlsx（v11 批次 142 升级：CSV → xlsx，规则 3 强制要求）
    /// v11 批次 141 新增：前端 exportOpportunities API 真实接入。；v11 批次 142 升级：导出格式从 CSV 升级为 xlsx（Excel 标准格式）。；查询所有匹配条件（不分页）的商机，生成 XlsxTable。；导出字段：商机编号/商机名称/客户ID/商机阶段/预估金额/实际金额/预期成交日期/实际成交日期/负责人/优先级/创建时间
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
                    opp.estimated_amount
                        .map(|d| d.to_string())
                        .unwrap_or_default(),
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
                    "无权访问商机 {}（数据范围限制）",
                    opportunity_id
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
        self.ensure_opportunity_not_closed(&opportunity)?;

        let mut active: crm_opportunity::ActiveModel = opportunity.into();
        Self::apply_opportunity_basic_fields(&mut active, &req);
        self.apply_opportunity_stage_and_rest(&mut active, &req)
            .await?;
        active.updated_at = Set(Some(chrono::Utc::now()));

        let opportunity = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            active,
            // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
        )
        .await?;

        Ok(opportunity)
    }

    /// 关闭后的商机不能修改
    fn ensure_opportunity_not_closed(
        &self,
        opportunity: &crm_opportunity::Model,
    ) -> Result<(), AppError> {
        if let Some(status) = &opportunity.opportunity_status {
            if status == opp_status::CLOSED_WON || status == opp_status::CLOSED_LOST {
                return Err(AppError::business("已关闭的商机不能修改".to_string()));
            }
        }
        Ok(())
    }

    /// 应用基础字段（不含阶段流转和赢率，这两者需在 stage_and_rest 中按顺序处理）
    fn apply_opportunity_basic_fields(
        active: &mut crm_opportunity::ActiveModel,
        req: &crate::models::dto::crm_dto::UpdateOpportunityRequest,
    ) {
        if let Some(v) = &req.opportunity_name {
            active.opportunity_name = Set(v.clone());
        }
        if let Some(v) = &req.customer_id {
            active.customer_id = Set(*v);
        }
        if let Some(v) = &req.lead_id {
            active.lead_id = Set(Some(*v));
        }
        if let Some(v) = &req.opportunity_type {
            active.opportunity_type = Set(Some(v.clone()));
        }
        if let Some(v) = &req.estimated_amount {
            active.estimated_amount = Set(Some(*v));
        }
        if let Some(v) = &req.actual_amount {
            active.actual_amount = Set(Some(*v));
        }
        if let Some(v) = &req.currency {
            active.currency = Set(Some(v.clone()));
        }
        if let Some(v) = &req.expected_close_date {
            active.expected_close_date = Set(Some(*v));
        }
        if let Some(v) = &req.actual_close_date {
            active.actual_close_date = Set(Some(*v));
        }
    }

    /// 应用阶段流转（含默认赢率重算）+ 赢率覆盖 + 其余字段
    /// 顺序约束：阶段流转必须在 win_probability 之前，以允许用户显式传入的赢率覆盖阶段默认值
    async fn apply_opportunity_stage_and_rest(
        &self,
        active: &mut crm_opportunity::ActiveModel,
        req: &crate::models::dto::crm_dto::UpdateOpportunityRequest,
    ) -> Result<(), AppError> {
        if let Some(v) = &req.opportunity_stage {
            self.validate_opportunity_stage_transition(active.opportunity_stage.as_ref(), v)?;
            // V15 P0-B08：阶段流转时自动重算赢率
            // 用户未显式传 win_probability 时，按新阶段的默认赢率填充
            // 若用户同时传了 win_probability，下方 req.win_probability 分支会覆盖此默认值
            let default_prob = if req.win_probability.is_none() {
                default_win_probability_by_stage(v)
            } else {
                None
            };
            active.opportunity_stage = Set(Some(v.clone()));
            if let Some(prob) = default_prob {
                active.win_probability = Set(Some(prob));
            }
        }
        if let Some(v) = &req.win_probability {
            active.win_probability = Set(Some(*v));
        }
        if let Some(v) = &req.product_ids {
            active.product_ids = Set(Some(v.clone()));
        }
        if let Some(v) = &req.product_names {
            active.product_names = Set(Some(v.clone()));
        }
        if let Some(v) = &req.product_desc {
            active.product_desc = Set(Some(v.clone()));
        }
        if let Some(v) = &req.priority {
            active.priority = Set(Some(v.clone()));
        }
        if let Some(v) = &req.rating {
            active.rating = Set(Some(*v));
        }
        if let Some(v) = &req.tags {
            active.tags = Set(Some(v.clone()));
        }
        Ok(())
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
        let customer_id = Self::validate_opportunity_for_conversion(&opportunity)?;
        let txn = self.db.begin().await?;
        let order = Self::create_draft_sales_order_from_opp(
            &txn,
            &opportunity,
            opportunity_id,
            customer_id,
            user_id,
        )
        .await?;
        Self::mark_opportunity_won_with_audit(&txn, opportunity, user_id).await?;
        txn.commit().await?;
        Ok(serde_json::json!({
            "order_id": order.id,
            "order_no": order.order_no,
        }))
    }

    /// 校验商机可转订单（未关闭赢单且有关联客户）
    fn validate_opportunity_for_conversion(
        opportunity: &crm_opportunity::Model,
    ) -> Result<i32, AppError> {
        if let Some(status) = &opportunity.opportunity_status {
            if status == opp_status::CLOSED_WON {
                return Err(AppError::business("商机已赢单".to_string()));
            }
        }
        Ok(opportunity.customer_id)
    }

    /// 从商机创建草稿销售订单
    async fn create_draft_sales_order_from_opp(
        txn: &sea_orm::DatabaseTransaction,
        opportunity: &crm_opportunity::Model,
        opportunity_id: i32,
        customer_id: i32,
        user_id: i32,
    ) -> Result<sales_order::Model, AppError> {
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
        .insert(txn)
        .await?;
        Ok(order)
    }

    /// 更新商机为赢单状态并写入审计日志
    async fn mark_opportunity_won_with_audit(
        txn: &sea_orm::DatabaseTransaction,
        opportunity: crm_opportunity::Model,
        user_id: i32,
    ) -> Result<(), AppError> {
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
        // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
        crate::services::audit_log_service::AuditLogService::update_with_audit(
            txn,
            "auto_audit",
            opp_active,
            Some(user_id),
        )
        .await?;
        Ok(())
    }

    /// 关单（输单流程）— V15 P0-B09（Batch 482）
    /// 将商机状态置为 CLOSED_LOST，强制要求写入流失原因 lost_reason。；设计依据：审计报告 §18.2-D2 — 输单原因未记录，销售改进无依据；业务规则：1. 商机当前状态不能是 CLOSED_WON / CLOSED_LOST（已关闭不可重复关单）；2. lost_reason 必填且非空（保证销售改进有依据）；3. 阶段置为 CLOSED_LOST，状态置为 CLOSED_LOST；4. 赢率自动置为 0（V15 P0-B08 联动）；5. 实际关闭日期置为今天
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

    /// V15 P1 18.2-D3：预测准确率分析
    /// 月度预测准确率 = 实际成交金额 / 预测金额 × 100%；预测金额 = 当月 expected_close_date 的商机 estimated_amount 之和；实际成交金额 = 当月 actual_close_date 且 CLOSED_WON 的商机 actual_amount 之和
    pub async fn forecast_accuracy(
        &self,
        year: i32,
        month: u32,
    ) -> Result<ForecastAccuracyResult, AppError> {
        use chrono::NaiveDate;

        let month_start = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| AppError::validation("无效的年月参数"))?;
        let month_end = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .unwrap_or(month_start);

        // 预测金额：当月预计成交的商机预估金额之和
        let forecast_opps = crm_opportunity::Entity::find()
            .filter(crm_opportunity::Column::ExpectedCloseDate.gte(month_start))
            .filter(crm_opportunity::Column::ExpectedCloseDate.lt(month_end))
            .all(&*self.db)
            .await?;

        let forecast_amount: Decimal = forecast_opps
            .iter()
            .map(|o| o.estimated_amount.unwrap_or(Decimal::ZERO))
            .sum();
        let forecast_count = forecast_opps.len() as i64;

        // 实际成交金额：当月实际成交的商机金额之和
        let won_opps = crm_opportunity::Entity::find()
            .filter(crm_opportunity::Column::ActualCloseDate.gte(month_start))
            .filter(crm_opportunity::Column::ActualCloseDate.lt(month_end))
            .filter(crm_opportunity::Column::OpportunityStage.eq(opp_status::CLOSED_WON))
            .all(&*self.db)
            .await?;

        let actual_amount: Decimal = won_opps
            .iter()
            .map(|o| {
                o.actual_amount
                    .unwrap_or(o.estimated_amount.unwrap_or(Decimal::ZERO))
            })
            .sum();
        let won_count = won_opps.len() as i64;

        // 准确率 = 实际 / 预测
        let accuracy_rate = if forecast_amount > Decimal::ZERO {
            let rate = actual_amount / forecast_amount;
            // 转为 f64 百分比
            rate.to_string().parse::<f64>().unwrap_or(0.0) * 100.0
        } else {
            0.0
        };

        Ok(ForecastAccuracyResult {
            year,
            month,
            forecast_amount,
            forecast_count,
            actual_amount,
            won_count,
            accuracy_rate,
        })
    }

    /// V15 P1 18.2-D4：加权销售预测（加权预测金额 = 商机金额 × 赢率（win_probability / 100）；返回所有 open 状态商机的加权预测汇总与明细。）
    pub async fn weighted_forecast(
        &self,
        owner_id: Option<i32>,
    ) -> Result<WeightedForecastResult, AppError> {
        let mut query = crm_opportunity::Entity::find();
        if let Some(oid) = owner_id {
            query = query.filter(crm_opportunity::Column::OwnerId.eq(oid));
        }

        let all_opps = query.all(&*self.db).await?;
        // 排除已关闭的商机（在 Rust 中过滤，兼容 NULL stage）
        let opps: Vec<&crm_opportunity::Model> = all_opps
            .iter()
            .filter(|o| {
                o.opportunity_stage.as_deref() != Some(opp_status::CLOSED_WON)
                    && o.opportunity_stage.as_deref() != Some(opp_status::CLOSED_LOST)
            })
            .collect();

        let mut total_estimated = Decimal::ZERO;
        let mut total_weighted = Decimal::ZERO;
        let mut details: Vec<WeightedForecastItem> = Vec::new();

        for opp in &opps {
            let estimated = opp.estimated_amount.unwrap_or(Decimal::ZERO);
            let win_prob = opp.win_probability.unwrap_or(Decimal::ZERO);
            // 加权金额 = 金额 × 赢率 / 100
            let weighted = estimated * win_prob / Decimal::from(100);
            total_estimated += estimated;
            total_weighted += weighted;
            details.push(WeightedForecastItem {
                opportunity_id: opp.id,
                opportunity_no: opp.opportunity_no.clone(),
                opportunity_name: opp.opportunity_name.clone(),
                stage: opp.opportunity_stage.clone().unwrap_or_default(),
                estimated_amount: estimated,
                win_probability: win_prob,
                weighted_amount: weighted,
                expected_close_date: opp.expected_close_date,
            });
        }

        Ok(WeightedForecastResult {
            total_opportunities: opps.len() as i64,
            total_estimated_amount: total_estimated,
            total_weighted_amount: total_weighted,
            details,
        })
    }

    /// V15 P1 18.5-D1：CRM 专用转化率分析（按月统计商机各阶段转化率，识别转化瓶颈。）
    pub async fn conversion_rate_analysis(
        &self,
        months_back: u32,
    ) -> Result<ConversionRateAnalysis, AppError> {
        let now = chrono::Utc::now();
        let start = now
            .checked_sub_signed(chrono::Duration::days((months_back as i64) * 30))
            .unwrap_or(now);

        let all_opps = crm_opportunity::Entity::find()
            .filter(crm_opportunity::Column::CreatedAt.gte(start))
            .all(&*self.db)
            .await?;

        let total = all_opps.len() as i64;
        let mut stage_counts: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();
        let mut won_count = 0i64;
        let mut lost_count = 0i64;

        for opp in &all_opps {
            let stage = opp
                .opportunity_stage
                .clone()
                .unwrap_or_else(|| "UNKNOWN".to_string());
            *stage_counts.entry(stage).or_insert(0) += 1;
            if opp.opportunity_stage.as_deref() == Some(opp_status::CLOSED_WON) {
                won_count += 1;
            } else if opp.opportunity_stage.as_deref() == Some(opp_status::CLOSED_LOST) {
                lost_count += 1;
            }
        }

        let closed_total = won_count + lost_count;
        let win_rate = if closed_total > 0 {
            (won_count as f64 / closed_total as f64) * 100.0
        } else {
            0.0
        };
        let conversion_rate = if total > 0 {
            (won_count as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        // 阶段分布
        let stage_distribution: Vec<StageCount> = stage_counts
            .into_iter()
            .map(|(stage, count)| StageCount { stage, count })
            .collect();

        Ok(ConversionRateAnalysis {
            period_start: start,
            period_end: now,
            total_opportunities: total,
            won_count,
            lost_count,
            open_count: total - won_count - lost_count,
            win_rate,
            conversion_rate,
            stage_distribution,
        })
    }

    /// V15 P1 18.5-D2：完整销售漏斗报表（线索→商机→报价→订单→回款 完整漏斗，各阶段数量与金额。）
    pub async fn sales_funnel_report(
        &self,
        start_date: Option<chrono::NaiveDate>,
        end_date: Option<chrono::NaiveDate>,
    ) -> Result<SalesFunnelReport, AppError> {
        use crate::models::{crm_lead, sales_quotation};

        let (start_dt, end_dt) = match (start_date, end_date) {
            (Some(s), Some(e)) => (
                Some(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                    s.and_hms_opt(0, 0, 0).unwrap_or_default(),
                    chrono::Utc,
                )),
                Some(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                    e.and_hms_opt(23, 59, 59).unwrap_or_default(),
                    chrono::Utc,
                )),
            ),
            _ => (None, None),
        };

        // 1. 线索数
        let mut lead_q = crm_lead::Entity::find();
        if let (Some(s), Some(e)) = (start_dt, end_dt) {
            lead_q = lead_q
                .filter(crm_lead::Column::CreatedAt.gte(s))
                .filter(crm_lead::Column::CreatedAt.lte(e));
        }
        let lead_count = lead_q.count(&*self.db).await?;

        // 2. 商机数与金额
        let mut opp_q = crm_opportunity::Entity::find();
        if let (Some(s), Some(e)) = (start_dt, end_dt) {
            opp_q = opp_q
                .filter(crm_opportunity::Column::CreatedAt.gte(s))
                .filter(crm_opportunity::Column::CreatedAt.lte(e));
        }
        let opps = opp_q.clone().all(&*self.db).await?;
        let opp_count = opps.len() as i64;
        let opp_amount: Decimal = opps
            .iter()
            .map(|o| o.estimated_amount.unwrap_or(Decimal::ZERO))
            .sum();

        // 3. 已成交商机数与金额
        let won_opps: Vec<&crm_opportunity::Model> = opps
            .iter()
            .filter(|o| o.opportunity_stage.as_deref() == Some(opp_status::CLOSED_WON))
            .collect();
        let won_count = won_opps.len() as i64;
        let won_amount: Decimal = won_opps
            .iter()
            .map(|o| {
                o.actual_amount
                    .unwrap_or(o.estimated_amount.unwrap_or(Decimal::ZERO))
            })
            .sum();

        // 4. 报价数
        let mut quot_q = sales_quotation::Entity::find();
        if let (Some(_s), Some(_e)) = (start_dt, end_dt) {
            quot_q = quot_q
                .filter(sales_quotation::Column::QuotationDate.gte(start_date.unwrap()))
                .filter(sales_quotation::Column::QuotationDate.lte(end_date.unwrap()));
        }
        let quotation_count = quot_q.count(&*self.db).await?;

        // 5. 订单数与金额
        let mut order_q = sales_order::Entity::find();
        if let (Some(s), Some(e)) = (start_dt, end_dt) {
            order_q = order_q
                .filter(sales_order::Column::CreatedAt.gte(s))
                .filter(sales_order::Column::CreatedAt.lte(e));
        }
        let orders = order_q.all(&*self.db).await?;
        let order_count = orders.len() as i64;
        let order_amount: Decimal = orders.iter().map(|o| o.total_amount).sum();

        // 6. 回款金额（已付款金额）
        let collected_amount: Decimal = orders.iter().map(|o| o.paid_amount).sum();

        // 转化率
        let lead_to_opp = if lead_count > 0 {
            (opp_count as f64 / lead_count as f64) * 100.0
        } else {
            0.0
        };
        let opp_to_quotation = if opp_count > 0 {
            (quotation_count as f64 / opp_count as f64) * 100.0
        } else {
            0.0
        };
        let opp_to_order = if opp_count > 0 {
            (order_count as f64 / opp_count as f64) * 100.0
        } else {
            0.0
        };
        let order_to_collection = if order_amount > Decimal::ZERO {
            (collected_amount / order_amount)
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0)
                * 100.0
        } else {
            0.0
        };

        Ok(SalesFunnelReport {
            lead_count: lead_count as i64,
            opportunity_count: opp_count,
            opportunity_amount: opp_amount,
            quotation_count: quotation_count as i64,
            won_count,
            won_amount,
            order_count,
            order_amount,
            collected_amount,
            lead_to_opp_rate: lead_to_opp,
            opp_to_quotation_rate: opp_to_quotation,
            opp_to_order_rate: opp_to_order,
            order_to_collection_rate: order_to_collection,
        })
    }

    /// V15 P2 18.2-D5: 阶段停留时长分析（统计每个商机在各阶段的停留天数）
    pub async fn stage_duration_analysis(
        &self,
        opportunity_id: Option<i32>,
    ) -> Result<Vec<StageDurationItem>, AppError> {
        use crate::models::opportunity_stage_history;

        let mut q = opportunity_stage_history::Entity::find();
        if let Some(opp_id) = opportunity_id {
            q = q.filter(opportunity_stage_history::Column::OpportunityId.eq(opp_id));
        }
        let records = q
            .order_by(opportunity_stage_history::Column::ChangedAt, sea_orm::Order::Desc)
            .all(&*self.db)
            .await?;

        let mut results = Vec::new();
        for record in &records {
            results.push(StageDurationItem {
                opportunity_id: record.opportunity_id,
                from_stage: record.from_stage.clone().unwrap_or_default(),
                to_stage: record.to_stage.clone(),
                changed_at: record.changed_at,
                duration_days: record.duration_days.unwrap_or(0),
            });
        }

        Ok(results)
    }

    /// V15 P2 18.2-D5: 记录商机阶段变更（自动计算停留天数）
    pub async fn record_stage_change(
        &self,
        opportunity_id: i32,
        from_stage: Option<String>,
        to_stage: &str,
        user_id: i32,
    ) -> Result<(), AppError> {
        use crate::models::opportunity_stage_history;

        // 计算在原阶段的停留天数
        let duration_days = if let Some(ref old_stage) = from_stage {
            // 查找上一次进入该阶段的时间
            let last_entry = opportunity_stage_history::Entity::find()
                .filter(opportunity_stage_history::Column::OpportunityId.eq(opportunity_id))
                .filter(opportunity_stage_history::Column::ToStage.eq(old_stage))
                .order_by(opportunity_stage_history::Column::ChangedAt, sea_orm::Order::Desc)
                .one(&*self.db)
                .await?;

            if let Some(entry) = last_entry {
                let now = chrono::Utc::now();
                let duration = now.signed_duration_since(entry.changed_at);
                Some(duration.num_days() as i32)
            } else {
                None
            }
        } else {
            None
        };

        let new_record = opportunity_stage_history::ActiveModel {
            id: Default::default(),
            opportunity_id: sea_orm::Set(opportunity_id),
            from_stage: sea_orm::Set(from_stage),
            to_stage: sea_orm::Set(to_stage.to_string()),
            changed_at: sea_orm::Set(chrono::Utc::now()),
            changed_by: sea_orm::Set(Some(user_id)),
            duration_days: sea_orm::Set(duration_days),
        }
        .insert(&*self.db)
        .await?;

        Ok(())
    }

    /// V15 P2 18.2-D6: 创建竞争对手
    pub async fn create_competitor(
        &self,
        req: CreateCompetitorRequest,
    ) -> Result<crate::models::competitor::Model, AppError> {
        use crate::models::competitor;

        let new_competitor = competitor::ActiveModel {
            id: Default::default(),
            name: sea_orm::Set(req.name),
            strengths: sea_orm::Set(req.strengths),
            weaknesses: sea_orm::Set(req.weaknesses),
            website: sea_orm::Set(req.website),
            notes: sea_orm::Set(req.notes),
            created_at: sea_orm::Set(Some(chrono::Utc::now())),
            updated_at: sea_orm::Set(Some(chrono::Utc::now())),
        }
        .insert(&*self.db)
        .await?;

        Ok(new_competitor)
    }

    /// V15 P2 18.2-D6: 获取竞争对手列表
    pub async fn list_competitors(&self) -> Result<Vec<crate::models::competitor::Model>, AppError> {
        use crate::models::competitor;

        let competitors = competitor::Entity::find()
            .order_by(competitor::Column::Name, sea_orm::Order::Asc)
            .all(&*self.db)
            .await?;

        Ok(competitors)
    }

    /// V15 P2 18.2-D6: 添加商机竞争对手关联
    pub async fn add_opportunity_competitor(
        &self,
        opportunity_id: i32,
        req: AddOpportunityCompetitorRequest,
    ) -> Result<crate::models::opportunity_competitor::Model, AppError> {
        use crate::models::opportunity_competitor;

        let new_record = opportunity_competitor::ActiveModel {
            id: Default::default(),
            opportunity_id: sea_orm::Set(opportunity_id),
            competitor_id: sea_orm::Set(req.competitor_id),
            threat_level: sea_orm::Set(req.threat_level),
            notes: sea_orm::Set(req.notes),
            created_at: sea_orm::Set(Some(chrono::Utc::now())),
        }
        .insert(&*self.db)
        .await?;

        Ok(new_record)
    }

    /// V15 P2 18.2-D6: 获取商机竞争对手列表
    pub async fn list_opportunity_competitors(
        &self,
        opportunity_id: i32,
    ) -> Result<Vec<OpportunityCompetitorItem>, AppError> {
        use crate::models::{competitor, opportunity_competitor};

        let records = opportunity_competitor::Entity::find()
            .filter(opportunity_competitor::Column::OpportunityId.eq(opportunity_id))
            .all(&*self.db)
            .await?;

        let mut results = Vec::new();
        for record in &records {
            let comp = competitor::Entity::find_by_id(record.competitor_id)
                .one(&*self.db)
                .await?;
            if let Some(c) = comp {
                results.push(OpportunityCompetitorItem {
                    id: record.id,
                    competitor_id: c.id,
                    competitor_name: c.name,
                    threat_level: record.threat_level.clone().unwrap_or_else(|| "medium".to_string()),
                    notes: record.notes.clone(),
                });
            }
        }

        Ok(results)
    }

    /// V15 P2 18.2-D7: 创建商机跟进记录
    pub async fn create_opportunity_follow_up(
        &self,
        opportunity_id: i32,
        req: CreateOpportunityFollowUpRequest,
        user_id: i32,
        user_name: String,
    ) -> Result<crate::models::opportunity_follow_up::Model, AppError> {
        use crate::models::opportunity_follow_up;

        let new_record = opportunity_follow_up::ActiveModel {
            id: Default::default(),
            opportunity_id: sea_orm::Set(opportunity_id),
            follow_up_type: sea_orm::Set(req.follow_up_type),
            content: sea_orm::Set(req.content),
            follow_up_time: sea_orm::Set(req.follow_up_time.unwrap_or_else(|| chrono::Utc::now())),
            next_follow_up_date: sea_orm::Set(req.next_follow_up_date),
            user_id: sea_orm::Set(user_id),
            user_name: sea_orm::Set(user_name),
            created_at: sea_orm::Set(Some(chrono::Utc::now())),
        }
        .insert(&*self.db)
        .await?;

        // 更新商机的最近跟进日期
        let opportunity = crm_opportunity::Entity::find_by_id(opportunity_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("商机不存在：{}", opportunity_id)))?;
        let mut opp_active: crm_opportunity::ActiveModel = opportunity.into();
        opp_active.last_follow_up_date = sea_orm::Set(Some(chrono::Utc::now().date_naive()));
        if let Some(next_date) = req.next_follow_up_date {
            opp_active.next_follow_up_date = sea_orm::Set(Some(next_date));
        }
        opp_active.updated_at = sea_orm::Set(Some(chrono::Utc::now()));
        opp_active.update(&*self.db).await?;

        Ok(new_record)
    }

    /// V15 P2 18.2-D7: 获取商机跟进记录列表
    pub async fn list_opportunity_follow_ups(
        &self,
        opportunity_id: i32,
    ) -> Result<Vec<crate::models::opportunity_follow_up::Model>, AppError> {
        use crate::models::opportunity_follow_up;

        let records = opportunity_follow_up::Entity::find()
            .filter(opportunity_follow_up::Column::OpportunityId.eq(opportunity_id))
            .order_by(opportunity_follow_up::Column::FollowUpTime, sea_orm::Order::Desc)
            .all(&*self.db)
            .await?;

        Ok(records)
    }
}

/// V15 P2 18.2-D5: 阶段停留时长项
#[derive(Debug, Clone, serde::Serialize)]
pub struct StageDurationItem {
    pub opportunity_id: i32,
    pub from_stage: String,
    pub to_stage: String,
    pub changed_at: chrono::DateTime<chrono::Utc>,
    pub duration_days: i32,
}

/// V15 P2 18.2-D6: 创建竞争对手请求
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateCompetitorRequest {
    pub name: String,
    pub strengths: Option<String>,
    pub weaknesses: Option<String>,
    pub website: Option<String>,
    pub notes: Option<String>,
}

/// V15 P2 18.2-D6: 添加商机竞争对手请求
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AddOpportunityCompetitorRequest {
    pub competitor_id: i32,
    pub threat_level: Option<String>,
    pub notes: Option<String>,
}

/// V15 P2 18.2-D6: 商机竞争对手项
#[derive(Debug, Clone, serde::Serialize)]
pub struct OpportunityCompetitorItem {
    pub id: i32,
    pub competitor_id: i32,
    pub competitor_name: String,
    pub threat_level: String,
    pub notes: Option<String>,
}

/// V15 P2 18.2-D7: 创建商机跟进记录请求
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateOpportunityFollowUpRequest {
    pub follow_up_type: String,
    pub content: String,
    pub follow_up_time: Option<chrono::DateTime<chrono::Utc>>,
    pub next_follow_up_date: Option<chrono::NaiveDate>,
}

/// V15 P2 18.2-D5: 预测准确性结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct ForecastAccuracyResult {
    pub year: i32,
    pub month: u32,
    pub forecast_amount: rust_decimal::Decimal,
    pub forecast_count: i64,
    pub actual_amount: rust_decimal::Decimal,
    pub won_count: i64,
    pub accuracy_rate: f64,
}

/// V15 P2 18.2-D5: 加权预测结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct WeightedForecastResult {
    pub total_opportunities: i64,
    pub total_estimated_amount: rust_decimal::Decimal,
    pub total_weighted_amount: rust_decimal::Decimal,
    pub details: Vec<WeightedForecastItem>,
}

/// V15 P2 18.2-D5: 加权预测项
#[derive(Debug, Clone, serde::Serialize)]
pub struct WeightedForecastItem {
    pub opportunity_id: i32,
    pub opportunity_no: String,
    pub opportunity_name: String,
    pub stage: String,
    pub estimated_amount: rust_decimal::Decimal,
    pub win_probability: rust_decimal::Decimal,
    pub weighted_amount: rust_decimal::Decimal,
    pub expected_close_date: Option<chrono::NaiveDate>,
}

/// V15 P2 18.2-D5: 转化率分析
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConversionRateAnalysis {
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
    pub total_opportunities: i64,
    pub won_count: i64,
    pub lost_count: i64,
    pub open_count: i64,
    pub win_rate: f64,
    pub conversion_rate: f64,
    pub stage_distribution: Vec<StageCount>,
}

/// V15 P2 18.2-D5: 阶段计数
#[derive(Debug, Clone, serde::Serialize)]
pub struct StageCount {
    pub stage: String,
    pub count: i64,
}

/// V15 P2 18.2-D5: 销售漏斗报告
#[derive(Debug, Clone, serde::Serialize)]
pub struct SalesFunnelReport {
    pub lead_count: i64,
    pub opportunity_count: i64,
    pub opportunity_amount: rust_decimal::Decimal,
    pub quotation_count: i64,
    pub won_count: i64,
    pub won_amount: rust_decimal::Decimal,
    pub order_count: i64,
    pub order_amount: rust_decimal::Decimal,
    pub collected_amount: rust_decimal::Decimal,
    pub lead_to_opp_rate: f64,
    pub opp_to_quotation_rate: f64,
    pub opp_to_order_rate: f64,
    pub order_to_collection_rate: f64,
}

/// V15 P2 18.2-D5: 漏斗阶段
#[derive(Debug, Clone, serde::Serialize)]
pub struct FunnelStage {
    pub stage: String,
    pub count: i32,
    pub amount: rust_decimal::Decimal,
}

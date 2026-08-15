use std::sync::Arc;

use axum::{Json, extract::State};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use serde::Deserialize;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::*;
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 客户合并请求
#[derive(Deserialize)]
pub struct MergeCustomerRequest {
    /// 源客户 ID（将被合并的客户）
    pub source_customer_id: i32,
    /// 目标客户 ID（合并后的主客户）
    pub target_customer_id: i32,
    /// 合并原因
    pub reason: Option<String>,
}

/// POST /api/v1/erp/customers/merge - 客户合并
/// batch-15 P3: 客户合并功能
/// P0-1 修复：转移所有关联数据到目标客户
pub async fn merge_customers(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<MergeCustomerRequest>,
) -> Result<Json<ApiResponse<String>>, AppError> {
    if req.source_customer_id == req.target_customer_id {
        return Err(AppError::bad_request("不能合并同一个客户"));
    }

    // 检查源客户是否存在
    let source = customer::Entity::find_by_id(req.source_customer_id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("源客户不存在"))?;

    // 检查目标客户是否存在
    let target = customer::Entity::find_by_id(req.target_customer_id)
        .one(&*state.db)
        .await?
        .ok_or_else(|| AppError::not_found("目标客户不存在"))?;

    // 开始事务
    let txn = state.db.begin().await?;

    // 1. 转移销售订单
    sales_order::Entity::update_many()
        .filter(sales_order::Column::CustomerId.eq(req.source_customer_id))
        .set(sales_order::ActiveModel {
            customer_id: Set(req.target_customer_id),
            ..Default::default()
        })
        .exec(&txn)
        .await?;

    // 2. 转移销售合同
    sales_contract::Entity::update_many()
        .filter(sales_contract::Column::CustomerId.eq(req.source_customer_id))
        .set(sales_contract::ActiveModel {
            customer_id: Set(req.target_customer_id),
            ..Default::default()
        })
        .exec(&txn)
        .await?;

    // 3. 转移销售报价
    sales_quotation::Entity::update_many()
        .filter(sales_quotation::Column::CustomerId.eq(req.source_customer_id as i64))
        .set(sales_quotation::ActiveModel {
            customer_id: Set(req.target_customer_id as i64),
            ..Default::default()
        })
        .exec(&txn)
        .await?;

    // 4. 转移应收账款发票
    ar_invoice::Entity::update_many()
        .filter(ar_invoice::Column::CustomerId.eq(req.source_customer_id))
        .set(ar_invoice::ActiveModel {
            customer_id: Set(req.target_customer_id),
            ..Default::default()
        })
        .exec(&txn)
        .await?;

    // 5. 转移应收账款对账
    ar_reconciliation::Entity::update_many()
        .filter(ar_reconciliation::Column::CustomerId.eq(req.source_customer_id))
        .set(ar_reconciliation::ActiveModel {
            customer_id: Set(req.target_customer_id),
            ..Default::default()
        })
        .exec(&txn)
        .await?;

    // 6. 转移 CRM 线索
    crm_lead::Entity::update_many()
        .filter(crm_lead::Column::ConvertedCustomerId.eq(Some(req.source_customer_id)))
        .set(crm_lead::ActiveModel {
            converted_customer_id: Set(Some(req.target_customer_id)),
            ..Default::default()
        })
        .exec(&txn)
        .await?;

    // 7. 转移 CRM 商机
    crm_opportunity::Entity::update_many()
        .filter(crm_opportunity::Column::CustomerId.eq(req.source_customer_id))
        .set(crm_opportunity::ActiveModel {
            customer_id: Set(req.target_customer_id),
            ..Default::default()
        })
        .exec(&txn)
        .await?;

    // 8. 转移色卡发放记录
    color_card_issue::Entity::update_many()
        .filter(color_card_issue::Column::CustomerId.eq(req.source_customer_id as i64))
        .set(color_card_issue::ActiveModel {
            customer_id: Set(req.target_customer_id as i64),
            ..Default::default()
        })
        .exec(&txn)
        .await?;

    // 9. 转移客户联系人
    customer_contact::Entity::update_many()
        .filter(customer_contact::Column::CustomerId.eq(req.source_customer_id))
        .set(customer_contact::ActiveModel {
            customer_id: Set(req.target_customer_id),
            ..Default::default()
        })
        .exec(&txn)
        .await?;

    // 10. 转移客户地址
    customer_address::Entity::update_many()
        .filter(customer_address::Column::CustomerId.eq(req.source_customer_id))
        .set(customer_address::ActiveModel {
            customer_id: Set(req.target_customer_id),
            ..Default::default()
        })
        .exec(&txn)
        .await?;

    // 11. 转移客户跟进记录
    customer_followup::Entity::update_many()
        .filter(customer_followup::Column::CustomerId.eq(req.source_customer_id))
        .set(customer_followup::ActiveModel {
            customer_id: Set(req.target_customer_id),
            ..Default::default()
        })
        .exec(&txn)
        .await?;

    // 12. 转移客户信用记录
    customer_credit::Entity::update_many()
        .filter(customer_credit::Column::CustomerId.eq(req.source_customer_id))
        .set(customer_credit::ActiveModel {
            customer_id: Set(req.target_customer_id),
            ..Default::default()
        })
        .exec(&txn)
        .await?;

    // 13. 将源客户标记为已合并
    let mut source_active: customer::ActiveModel = source.into();
    source_active.status = Set("merged".to_string());
    source_active.notes = Set(Some(format!(
        "已合并到客户 {} (ID: {})。原因: {}",
        target.customer_name,
        req.target_customer_id,
        req.reason.unwrap_or_else(|| "无".to_string())
    )));
    source_active.update(&txn).await?;

    // 14. 记录审计日志
    let audit_event = AuditEvent {
        user_id: Some(auth.user_id),
        username: Some(auth.username.clone()),
        operation_type: audit_log::OperationType::Update,
        severity: audit_log::Severity::Info,
        resource_type: Some("customer".to_string()),
        resource_id: Some(req.source_customer_id.to_string()),
        resource_name: Some(format!(
            "客户合并: {} -> {}",
            req.source_customer_id, req.target_customer_id
        )),
        description: Some(format!(
            "将客户 {} 合并到客户 {}，转移了所有关联数据",
            req.source_customer_id, req.target_customer_id
        )),
        request_method: Some("POST".to_string()),
        request_path: Some("/api/v1/erp/customers/merge".to_string()),
        before_snapshot: None,
        after_snapshot: None,
    };
    let audit_svc = Arc::new(AuditLogService::new(state.db.clone()));
    audit_svc.record_async(audit_event, None);

    txn.commit().await?;

    Ok(Json(ApiResponse::success(format!(
        "客户 {} 已成功合并到 {}，所有关联数据已转移",
        req.source_customer_id, req.target_customer_id
    ))))
}

//! 定制订单质检服务
//!
//! 处理质量异常上报、解决、查询
//! 行业规则：GB/T 26377-2022 颜色标准 + ISO 105 色牢度
//! 创建时间: 2026-06-17

use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait};
use std::sync::Arc;
use thiserror::Error;

use crate::models::quality_issue::{self, ActiveModel, Entity};
use crate::models::quality_issue_dto::{ReportQualityIssueDto, ResolveQualityIssueDto};
use crate::utils::app_state::AppState;
use crate::utils::error::AppError;

/// 业务错误
#[derive(Debug, Error)]
pub enum QualityError {
    #[error("异常记录不存在")]
    NotFound,
    #[error("非法状态: {0}")]
    InvalidState(String),
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// 质检服务
pub struct CustomOrderQualityService {
    db: Arc<DatabaseConnection>,
}

impl CustomOrderQualityService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 上报质量异常
    pub async fn report_issue(
        &self,
        dto: ReportQualityIssueDto,
    ) -> Result<quality_issue::Model, QualityError> {
        // 校验严重度
        if !["low", "medium", "high", "critical"].contains(&dto.severity.as_str()) {
            return Err(QualityError::Validation(format!(
                "非法严重度: {}",
                dto.severity
            )));
        }

        // GB/T 26377 颜色标准校验：色差 ΔE 阈值
        if let Some(delta_e) = dto.color_delta_e {
            if delta_e < rust_decimal::Decimal::ZERO {
                return Err(QualityError::Validation("色差 ΔE 不能为负数".to_string()));
            }
            // v11 批次 156 P2-D：接入 color_space_converter::delta_e_is_acceptable 判定可接受色差
            let delta_e_f64 = delta_e.to_string().parse::<f64>().unwrap_or(f64::MAX);
            if !crate::utils::color_space_converter::delta_e_is_acceptable(delta_e_f64) {
                tracing::warn!(
                    "GB/T 26377-2022 提示：色差 ΔE={} 超过可接受阈值 3.0",
                    delta_e
                );
            }
        }

        // ISO 105 色牢度校验：等级范围 1-5
        if let Some(grade) = dto.color_fastness_grade {
            if !(1..=5).contains(&grade) {
                return Err(QualityError::Validation(
                    "ISO 105 色牢度等级必须在 1-5 之间".to_string(),
                ));
            }
        }

        let now = Utc::now();
        let active = ActiveModel {
            id: Default::default(),
            custom_order_id: Set(dto.custom_order_id),
            process_node_id: Set(dto.process_node_id),
            issue_type: Set(dto.issue_type),
            severity: Set(dto.severity),
            description: Set(dto.description),
            discovered_at: Set(now),
            resolved_at: Set(None),
            resolution: Set(None),
            status: Set("open".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let result = active.insert(&*self.db).await?;
        Ok(result)
    }

    /// 解决异常
    // 批次 94 P2-15 修复：移除 let _ = dto.operator_id 占位，用 update_with_audit 记录 operator_id 到审计日志；
    // 返回类型改为 AppError 以兼容审计服务（update_with_audit 返回 AppError）
    pub async fn resolve_issue(
        &self,
        id: i64,
        dto: ResolveQualityIssueDto,
    ) -> Result<quality_issue::Model, AppError> {
        // P2-6 修复（批次 84 v1 复审）：状态门 + update 移入单一事务，加 lock_exclusive 串行化
        // 原实现状态门查询在 self.db 上、update 也在 self.db 上，无事务边界，
        // 并发场景下可能在状态检查通过后、update 前发生状态变更，导致已关闭异常被重复解决。
        let txn = (*self.db).begin().await?;

        let existing = Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("异常记录不存在"))?;

        if existing.status == "closed" {
            return Err(AppError::business("已关闭的异常不可再次解决".to_string()));
        }

        let now = Utc::now();
        let mut active: ActiveModel = existing.into();
        active.resolution = Set(Some(dto.resolution));
        active.resolved_at = Set(Some(now));
        active.status = Set("resolved".to_string());
        active.updated_at = Set(now);
        // 批次 94 P2-15 修复：用 update_with_audit 替换 active.update，记录 operator_id 到审计日志
        // quality_issue 模型无 operator_id 字段，操作人只能通过 audit_log 追溯
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "quality_issue",
            active,
            Some(dto.operator_id as i32),
        )
        .await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 列出订单的所有异常
    pub async fn list_by_order(
        &self,
        order_id: i64,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<quality_issue::Model>, u64), QualityError> {
        let query = Entity::find()
            .filter(quality_issue::Column::CustomOrderId.eq(order_id));

        let paginator = query
            .order_by_desc(quality_issue::Column::DiscoveredAt)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.saturating_sub(1)).await?;
        Ok((items, total))
    }

}

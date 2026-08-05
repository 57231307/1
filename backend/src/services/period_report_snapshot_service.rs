//! 期末报表快照服务
//!
//! 提供报表快照的创建、查询、验证功能

use crate::models::period_report_snapshot;
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing::info;

/// 创建报表快照请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSnapshotRequest {
    /// 会计期间 ID
    pub period_id: i32,
    /// 报表类型（balance_sheet/income_statement/cash_flow/trial_balance）
    pub report_type: String,
    /// 报表数据（JSON）
    pub report_data: serde_json::Value,
}

/// 报表快照查询参数
#[derive(Debug, Clone, Default)]
pub struct SnapshotQueryParams {
    pub period_id: Option<i32>,
    pub report_type: Option<String>,
    pub page: u64,
    pub page_size: u64,
}

pub struct PeriodReportSnapshotService {
    db: Arc<DatabaseConnection>,
}

impl PeriodReportSnapshotService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建报表快照
    pub async fn create(
        &self,
        req: CreateSnapshotRequest,
        user_id: i32,
    ) -> Result<period_report_snapshot::Model, AppError> {
        info!(
            "用户 {} 正在创建报表快照：期间 {}，类型 {}",
            user_id, req.period_id, req.report_type
        );

        // 计算快照哈希（SHA-256）
        let data_str = serde_json::to_string(&req.report_data)
            .map_err(|e| AppError::internal(format!("序列化报表数据失败: {}", e)))?;
        let hash = format!("{:x}", Sha256::digest(data_str.as_bytes()));

        let active = period_report_snapshot::ActiveModel {
            period_id: Set(req.period_id),
            report_type: Set(req.report_type),
            report_data: Set(req.report_data),
            snapshot_hash: Set(hash),
            created_by: Set(user_id),
            ..Default::default()
        };

        let snapshot = active.insert(&*self.db).await?;
        info!("报表快照创建成功：ID {}", snapshot.id);
        Ok(snapshot)
    }

    /// 查询快照列表
    pub async fn list(
        &self,
        params: SnapshotQueryParams,
    ) -> Result<(Vec<period_report_snapshot::Model>, u64), AppError> {
        let mut query = period_report_snapshot::Entity::find();

        if let Some(period_id) = params.period_id {
            query = query.filter(period_report_snapshot::Column::PeriodId.eq(period_id));
        }
        if let Some(ref report_type) = params.report_type {
            query = query.filter(period_report_snapshot::Column::ReportType.eq(report_type.as_str()));
        }

        let total = query.clone().count(&*self.db).await?;

        let snapshots = query
            .order_by(period_report_snapshot::Column::CreatedAt, Order::Desc)
            .offset((params.page * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((snapshots, total))
    }

    /// 获取快照详情
    pub async fn get_by_id(&self, id: i32) -> Result<period_report_snapshot::Model, AppError> {
        let snapshot = period_report_snapshot::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("报表快照不存在：{}", id)))?;
        Ok(snapshot)
    }

    /// 验证快照完整性（重新计算哈希比对）
    pub async fn verify_integrity(&self, id: i32) -> Result<bool, AppError> {
        let snapshot = self.get_by_id(id).await?;

        let data_str = serde_json::to_string(&snapshot.report_data)
            .map_err(|e| AppError::internal(format!("序列化报表数据失败: {}", e)))?;
        let expected_hash = format!("{:x}", Sha256::digest(data_str.as_bytes()));

        Ok(snapshot.snapshot_hash == expected_hash)
    }
}

//! 染缸占用服务（V15 P2 B05-P2-6）
//!
//! 提供染缸设备占用/释放/可用性查询能力：
//! - occupy：缸号进入 dyeing 状态时占用染缸（唯一约束防重复占用）
//! - release：缸号离开 dyeing 状态时释放染缸
//! - check_availability：查询染缸当前是否可用
//! - list_occupations：按状态列出占用记录
//!
//! 调用方：event_bus_ops/listener.rs handle_dye_batch_status_changed

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use std::sync::Arc;
use tracing::{info, warn};

use crate::models::dye_vat_occupation::occupation_status;
use crate::models::dye_vat_occupation::{self, ActiveModel, Entity, Model};
use crate::utils::error::AppError;

/// 染缸占用服务
pub struct DyeVatOccupationService {
    db: Arc<DatabaseConnection>,
}

#[allow(dead_code)]
impl DyeVatOccupationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 占用染缸（缸号进入 dyeing 状态时调用）。
    /// 若该染缸已被占用则返回业务错误（唯一约束兜底 + 应用层预检）。
    pub async fn occupy(
        &self,
        vat_id: i32,
        batch_id: i32,
        batch_no: Option<String>,
    ) -> Result<Model, AppError> {
        if let Some(existing) = self.find_active_occupation_by_vat(vat_id).await? {
            return Err(AppError::business(format!(
                "染缸 {} 已被缸号 {} 占用，不可重复占用",
                vat_id, existing.batch_id
            )));
        }
        let now = Utc::now();
        let active = ActiveModel {
            id: Default::default(),
            vat_id: Set(vat_id),
            batch_id: Set(batch_id),
            batch_no: Set(batch_no),
            occupied_at: Set(now),
            released_at: Set(None),
            status: Set(occupation_status::OCCUPIED.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
        };
        let model = active
            .insert(&*self.db)
            .await
            .map_err(|e| AppError::database(format!("染缸占用记录插入失败: {}", e)))?;
        info!(vat_id, batch_id, "染缸已占用（B05-P2-6 设备资源管理闭环）");
        Ok(model)
    }

    /// 释放染缸（缸号离开 dyeing 状态时调用，按 batch_id 释放）。
    /// 幂等：若无活跃占用记录则跳过（warn 提示）。
    pub async fn release(&self, batch_id: i32) -> Result<Option<Model>, AppError> {
        let existing = Entity::find()
            .filter(dye_vat_occupation::Column::BatchId.eq(batch_id))
            .filter(dye_vat_occupation::Column::Status.eq(occupation_status::OCCUPIED))
            .one(&*self.db)
            .await?;
        let Some(model) = existing else {
            warn!(
                batch_id,
                "缸号无活跃染缸占用记录，跳过释放（B05-P2-6 幂等）"
            );
            return Ok(None);
        };
        let now = Utc::now();
        let mut active: ActiveModel = model.into();
        active.released_at = Set(Some(now));
        active.status = Set(occupation_status::RELEASED.to_string());
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        info!(
            batch_id,
            vat_id = updated.vat_id,
            "染缸已释放（B05-P2-6 设备资源管理闭环）"
        );
        Ok(Some(updated))
    }

    /// 查询染缸当前是否可用（无活跃占用记录即可用）。
    pub async fn check_availability(&self, vat_id: i32) -> Result<bool, AppError> {
        let occupied = self.find_active_occupation_by_vat(vat_id).await?;
        Ok(occupied.is_none())
    }

    /// 按状态列出占用记录（分页）。
    pub async fn list_occupations(
        &self,
        status: Option<&str>,
        limit: u64,
    ) -> Result<Vec<Model>, AppError> {
        let mut query = Entity::find().order_by_desc(dye_vat_occupation::Column::OccupiedAt);
        if let Some(s) = status {
            query = query.filter(dye_vat_occupation::Column::Status.eq(s));
        }
        let items = query.limit(limit).all(&*self.db).await?;
        Ok(items)
    }

    /// 查询指定染缸的活跃占用记录（status='occupied'）。
    async fn find_active_occupation_by_vat(&self, vat_id: i32) -> Result<Option<Model>, AppError> {
        let model = Entity::find()
            .filter(dye_vat_occupation::Column::VatId.eq(vat_id))
            .filter(dye_vat_occupation::Column::Status.eq(occupation_status::OCCUPIED))
            .one(&*self.db)
            .await?;
        Ok(model)
    }
}

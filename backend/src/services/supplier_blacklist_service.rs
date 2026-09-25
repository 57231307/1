//! 供应商黑名单服务
//!
//! 提供黑名单记录的分页列表、加入黑名单（create）、解除黑名单（release）操作。
//! 供采购门控（创建订单/收货/付款）调用的 `check_supplier_not_blacklisted` 校验方法。

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, Set,
};
use serde::Deserialize;
use std::sync::Arc;
use validator::Validate;

use crate::models::{supplier, supplier_blacklist};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use crate::utils::redis_cache::{cache_key, redis_cache_del};
use crate::utils::response::PaginatedResponse;

/// 黑名单有效状态（与表 DEFAULT 'PENDING' 一致）
const RELEASE_STATUS_PENDING: &str = "PENDING";
/// 黑名单已解除状态
const RELEASE_STATUS_RELEASED: &str = "RELEASED";

/// 供应商黑名单服务
pub struct SupplierBlacklistService {
    db: Arc<DatabaseConnection>,
}

impl SupplierBlacklistService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 分页查询黑名单记录（可按 supplier_id / release_status 筛选）
    pub async fn list_blacklists(
        &self,
        params: BlacklistQueryParams,
    ) -> Result<PaginatedResponse<BlacklistListItem>, AppError> {
        let mut query = supplier_blacklist::Entity::find();

        if let Some(sid) = params.supplier_id {
            query = query.filter(supplier_blacklist::Column::SupplierId.eq(sid));
        }
        if let Some(ref status) = params.release_status {
            query = query.filter(supplier_blacklist::Column::ReleaseStatus.eq(status.as_str()));
        }

        query = query.order_by(supplier_blacklist::Column::BlacklistDate, Order::Desc);

        let page = params.page.unwrap_or(1).clamp(1, 1000);
        let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

        let paginator = query.paginate(&*self.db, page_size);
        let (records, total) = paginate_with_total(paginator, page).await?;

        // 富化供应商名称（避免 N+1：收集所有 supplier_id 批量查询）
        let supplier_ids: Vec<i32> = records.iter().map(|r| r.supplier_id).collect();
        let supplier_names: std::collections::HashMap<i32, String> = if supplier_ids.is_empty() {
            Default::default()
        } else {
            supplier::Entity::find()
                .filter(supplier::Column::Id.is_in(supplier_ids))
                .all(&*self.db)
                .await?
                .into_iter()
                .map(|s| (s.id, s.supplier_name))
                .collect()
        };

        let items: Vec<BlacklistListItem> = records
            .into_iter()
            .map(|r| {
                let supplier_name = supplier_names
                    .get(&r.supplier_id)
                    .cloned()
                    .unwrap_or_else(|| format!("供应商#{}", r.supplier_id));
                BlacklistListItem {
                    id: r.id,
                    supplier_id: r.supplier_id,
                    supplier_name,
                    blacklist_date: r.blacklist_date,
                    blacklist_reason: r.blacklist_reason,
                    detail_description: r.detail_description,
                    evidence: r.evidence,
                    approver_id: r.approver_id,
                    approval_date: r.approval_date,
                    is_permanent: r.is_permanent,
                    release_date: r.release_date,
                    release_condition: r.release_condition,
                    release_status: r.release_status,
                    release_date_actual: r.release_date_actual,
                    remarks: r.remarks,
                    created_at: r.created_at,
                    created_by: r.created_by,
                }
            })
            .collect();

        Ok(PaginatedResponse::new(items, total, page, page_size))
    }

    /// 将供应商加入黑名单
    pub async fn add_to_blacklist(
        &self,
        req: CreateBlacklistRequest,
        user_id: i32,
    ) -> Result<supplier_blacklist::Model, AppError> {
        req.validate()?;

        // 校验供应商存在
        let supplier_exists = supplier::Entity::find_by_id(req.supplier_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("供应商 {} 不存在", req.supplier_id)))?;

        // 校验该供应商当前没有处于有效黑名单中（release_status=PENDING）
        let active_count = supplier_blacklist::Entity::find()
            .filter(supplier_blacklist::Column::SupplierId.eq(req.supplier_id))
            .filter(supplier_blacklist::Column::ReleaseStatus.eq(RELEASE_STATUS_PENDING))
            .count(&*self.db)
            .await?;
        if active_count > 0 {
            return Err(AppError::business(format!(
                "供应商 '{}' 已在黑名单中，不可重复加入",
                supplier_exists.supplier_name
            )));
        }

        let now = chrono::Utc::now().date_naive();
        let record = supplier_blacklist::ActiveModel {
            supplier_id: Set(req.supplier_id),
            blacklist_date: Set(req.blacklist_date.unwrap_or(now)),
            blacklist_reason: Set(req.blacklist_reason),
            detail_description: Set(req.detail_description),
            evidence: Set(req.evidence),
            approver_id: Set(req.approver_id),
            approval_date: Set(req.approval_date.unwrap_or(now)),
            is_permanent: Set(req.is_permanent),
            release_date: Set(req.release_date),
            release_condition: Set(req.release_condition),
            release_status: Set(RELEASE_STATUS_PENDING.to_string()),
            remarks: Set(req.remarks),
            created_by: Set(Some(user_id)),
            updated_by: Set(Some(user_id)),
            ..Default::default()
        }
        .insert(&*self.db)
        .await?;

        // 同步更新供应商主表状态为 blacklisted
        let mut sup_active: supplier::ActiveModel = supplier_exists.into();
        sup_active.status = Set(Some("blacklisted".to_string()));
        sup_active.updated_by = Set(Some(user_id));
        sup_active.update(&*self.db).await?;

        // 失效供应商缓存（状态已变更）
        redis_cache_del(&cache_key("supplier", req.supplier_id)).await;

        tracing::info!(
            supplier_id = record.supplier_id,
            blacklist_id = record.id,
            operator = user_id,
            "供应商已加入黑名单"
        );

        Ok(record)
    }

    /// 解除黑名单（设置 release_status = RELEASED）
    pub async fn release_from_blacklist(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<supplier_blacklist::Model, AppError> {
        let record = supplier_blacklist::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("黑名单记录 {} 不存在", id)))?;

        if record.release_status != RELEASE_STATUS_PENDING {
            return Err(AppError::business(format!(
                "该黑名单记录当前状态为 {}，不可重复解除",
                record.release_status
            )));
        }

        let now = chrono::Utc::now().date_naive();
        let mut active: supplier_blacklist::ActiveModel = record.into();
        active.release_status = Set(RELEASE_STATUS_RELEASED.to_string());
        active.release_date_actual = Set(Some(now));
        active.updated_by = Set(Some(user_id));
        let updated = active.update(&*self.db).await?;

        // 检查该供应商是否还有其他有效黑名单记录，若无则恢复供应商状态
        let remaining_pending = supplier_blacklist::Entity::find()
            .filter(supplier_blacklist::Column::SupplierId.eq(updated.supplier_id))
            .filter(supplier_blacklist::Column::ReleaseStatus.eq(RELEASE_STATUS_PENDING))
            .count(&*self.db)
            .await?;
        if remaining_pending == 0 {
            if let Some(sup) = supplier::Entity::find_by_id(updated.supplier_id)
                .one(&*self.db)
                .await?
            {
                let mut sup_active: supplier::ActiveModel = sup.into();
                sup_active.status =
                    Set(Some(crate::models::status::master_data::ACTIVE.to_string()));
                sup_active.updated_by = Set(Some(user_id));
                sup_active.update(&*self.db).await?;
            }
        }

        // 失效供应商缓存（状态可能已变更）
        redis_cache_del(&cache_key("supplier", updated.supplier_id)).await;

        tracing::info!(
            supplier_id = updated.supplier_id,
            blacklist_id = updated.id,
            operator = user_id,
            "供应商已从黑名单解除"
        );

        Ok(updated)
    }

    /// 采购门控核心方法：校验供应商是否处于有效黑名单。
    ///
    /// 命中时返回 `Err(AppError::business(...))` 含供应商名称与拉黑原因；
    /// 未命中或无记录时返回 `Ok(())`。
    pub async fn check_supplier_not_blacklisted(&self, supplier_id: i32) -> Result<(), AppError> {
        let record = supplier_blacklist::Entity::find()
            .filter(supplier_blacklist::Column::SupplierId.eq(supplier_id))
            .filter(supplier_blacklist::Column::ReleaseStatus.eq(RELEASE_STATUS_PENDING))
            .one(&*self.db)
            .await?;

        if let Some(blacklist_entry) = record {
            let supplier_name = supplier::Entity::find_by_id(supplier_id)
                .one(&*self.db)
                .await?
                .map(|s| s.supplier_name)
                .unwrap_or_else(|| format!("供应商#{}", supplier_id));

            tracing::warn!(
                supplier_id,
                blacklist_id = blacklist_entry.id,
                reason = %blacklist_entry.blacklist_reason,
                "采购门控拦截：供应商在有效黑名单中"
            );

            return Err(AppError::business(format!(
                "供应商 '{}' 已被列入黑名单，禁止采购操作。拉黑原因：{}",
                supplier_name, blacklist_entry.blacklist_reason
            )));
        }

        Ok(())
    }
}

/// 黑名单列表查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct BlacklistQueryParams {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub supplier_id: Option<i32>,
    pub release_status: Option<String>,
}

/// 加入黑名单请求
#[derive(Debug, Deserialize, Validate)]
pub struct CreateBlacklistRequest {
    #[validate(range(min = 1, message = "供应商ID必须大于0"))]
    pub supplier_id: i32,
    #[validate(length(min = 1, max = 200, message = "拉黑原因不能为空且不超过200字符"))]
    pub blacklist_reason: String,
    #[validate(length(min = 1, message = "详细说明不能为空"))]
    pub detail_description: String,
    pub evidence: Option<String>,
    #[validate(range(min = 1, message = "审批人ID必须大于0"))]
    pub approver_id: i32,
    pub blacklist_date: Option<chrono::NaiveDate>,
    pub approval_date: Option<chrono::NaiveDate>,
    pub is_permanent: bool,
    pub release_date: Option<chrono::NaiveDate>,
    pub release_condition: Option<String>,
    pub remarks: Option<String>,
}

/// 黑名单列表返回项（含富化的供应商名称）
#[derive(Debug, Clone, serde::Serialize)]
pub struct BlacklistListItem {
    pub id: i32,
    pub supplier_id: i32,
    pub supplier_name: String,
    pub blacklist_date: chrono::NaiveDate,
    pub blacklist_reason: String,
    pub detail_description: String,
    pub evidence: Option<String>,
    pub approver_id: i32,
    pub approval_date: chrono::NaiveDate,
    pub is_permanent: bool,
    pub release_date: Option<chrono::NaiveDate>,
    pub release_condition: Option<String>,
    pub release_status: String,
    pub release_date_actual: Option<chrono::NaiveDate>,
    pub remarks: Option<String>,
    pub created_at: sea_orm::prelude::DateTimeWithTimeZone,
    pub created_by: Option<i32>,
}

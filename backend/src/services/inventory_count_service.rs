//! 库存盘点服务
//!
//! 提供库存盘点单的全套业务能力：
//! - 盘点单 CRUD（创建 / 查询 / 更新 / 删除）
//! - 盘点明细管理（基于库存快照自动生成明细）
//! - 实盘数量录入与差异计算（quantity_actual - quantity_before）
//! - 盘点审批流（提交审批 / 审批通过 / 审批驳回）
//! - 盘点完成（审批后落库实际数量，更新库存）
//!
//! v11 批次 143 P1-1：v8 删除的占位 handler 已真实实现，
//! 模型 inventory_count / inventory_count_item 已通过迁移对齐 schema。

use crate::models::status::inventory_count as count_status;
use crate::models::{inventory_count, inventory_count_item, inventory_stock};
use crate::services::audit_log_service::AuditLogService;
use crate::utils::error::AppError;
// 批次 260 修复：接入 paginate_with_total 统一分页逻辑
use crate::utils::pagination::paginate_with_total;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;

/// 创建盘点单请求
#[derive(Debug, Clone)]
pub struct CreateCountRequest {
    pub warehouse_id: i32,
    pub count_date: DateTime<Utc>,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    /// 指定库存快照 ID 列表，None 表示仓库下全部库存
    pub stock_ids: Option<Vec<i32>>,
}

/// 更新盘点单请求
#[derive(Debug, Clone, Default)]
pub struct UpdateCountRequest {
    pub count_date: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}

/// 盘点明细录入请求
#[derive(Debug, Clone)]
pub struct CountItemInput {
    pub stock_id: i32,
    pub quantity_actual: Decimal,
    pub notes: Option<String>,
}

/// 盘点单详情
#[derive(Debug, Clone)]
pub struct CountDetail {
    pub count: inventory_count::Model,
    pub items: Vec<inventory_count_item::Model>,
}

#[derive(Debug, Clone)]
pub struct InventoryCountService {
    db: Arc<DatabaseConnection>,
}

impl InventoryCountService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建盘点单（事务内生成单号 + 主表 + 明细快照）
    ///
    /// 批次 340 v11 复审 P1 修复：移除 `#[allow(clippy::default_constructed_unit_structs)]` 抑制，
    /// SeaORM unit struct `Entity` 直接作为值传递，无需 `::default()` 构造。
    pub async fn create_count(&self, req: CreateCountRequest) -> Result<CountDetail, AppError> {
        let txn = (*self.db).begin().await?;

        // 生成盘点单号：IC{YYYYMMDD}{3位流水}
        let count_no = crate::utils::number_generator::DocumentNumberGenerator::generate_no_with_txn(
            &txn,
            "IC",
            inventory_count::Entity,
            inventory_count::Column::CountNo,
        )
        .await?;

        // 查询仓库下的库存快照（按 stock_ids 过滤或全量）
        let stock_query = inventory_stock::Entity::find()
            .filter(inventory_stock::Column::WarehouseId.eq(req.warehouse_id));
        let stocks: Vec<inventory_stock::Model> = match req.stock_ids.as_ref() {
            Some(ids) if !ids.is_empty() => {
                stock_query
                    .filter(inventory_stock::Column::Id.is_in(ids.clone()))
                    .all(&txn)
                    .await?
            }
            _ => stock_query.all(&txn).await?,
        };

        if stocks.is_empty() {
            return Err(AppError::business(format!(
                "仓库 {} 下无库存记录，无法创建盘点单",
                req.warehouse_id
            )));
        }

        let total_items = stocks.len() as i32;

        // 创建盘点单主表
        let count_active = inventory_count::ActiveModel {
            id: Default::default(),
            count_no: Set(count_no.clone()),
            warehouse_id: Set(req.warehouse_id),
            count_date: Set(req.count_date),
            status: Set(count_status::PENDING.to_string()),
            total_items: Set(total_items),
            counted_items: Set(0),
            variance_items: Set(0),
            notes: Set(req.notes),
            created_by: Set(req.created_by),
            approved_by: Set(None),
            approved_at: Set(None),
            completed_at: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };
        let count_model = count_active.insert(&txn).await?;

        // 批量创建盘点明细（quantity_actual 初始化为 0，待实际盘点时录入）
        let mut item_models = Vec::with_capacity(stocks.len());
        for stock in stocks {
            let item = inventory_count_item::ActiveModel {
                id: Default::default(),
                count_id: Set(count_model.id),
                stock_id: Set(stock.id),
                product_id: Set(stock.product_id),
                warehouse_id: Set(stock.warehouse_id),
                quantity_before: Set(stock.quantity_on_hand),
                quantity_actual: Set(Decimal::ZERO),
                quantity_difference: Set(Decimal::ZERO),
                unit_cost: Set(Decimal::ZERO),
                total_cost: Set(Decimal::ZERO),
                notes: Set(None),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
            };
            item_models.push(item.insert(&txn).await?);
        }

        txn.commit().await?;

        Ok(CountDetail {
            count: count_model,
            items: item_models,
        })
    }

    /// 查询盘点单列表（分页）
    pub async fn list_counts(
        &self,
        page: u64,
        page_size: u64,
        warehouse_id: Option<i32>,
        status: Option<String>,
    ) -> Result<(Vec<inventory_count::Model>, u64), AppError> {
        let mut query = inventory_count::Entity::find();
        if let Some(wid) = warehouse_id {
            query = query.filter(inventory_count::Column::WarehouseId.eq(wid));
        }
        if let Some(s) = status {
            query = query.filter(inventory_count::Column::Status.eq(s));
        }
        // 批次 260 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = query
            .order_by(inventory_count::Column::CreatedAt, Order::Desc)
            .paginate(&*self.db, page_size);
        let (counts, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((counts, total))
    }

    /// 查询盘点单详情
    pub async fn get_count(&self, count_id: i32) -> Result<CountDetail, AppError> {
        let count = inventory_count::Entity::find_by_id(count_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("盘点单 {} 不存在", count_id)))?;
        let items = inventory_count_item::Entity::find()
            .filter(inventory_count_item::Column::CountId.eq(count_id))
            .order_by(inventory_count_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;
        Ok(CountDetail { count, items })
    }

    /// 更新盘点单（仅 pending 状态可更新）
    pub async fn update_count(
        &self,
        count_id: i32,
        req: UpdateCountRequest,
        user_id: Option<i32>,
    ) -> Result<inventory_count::Model, AppError> {
        let txn = (*self.db).begin().await?;
        let count_model = inventory_count::Entity::find_by_id(count_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("盘点单 {} 不存在", count_id)))?;
        if count_model.status != count_status::PENDING {
            return Err(AppError::business(
                "只有待盘点状态的盘点单可以更新".to_string(),
            ));
        }
        let mut active: inventory_count::ActiveModel = count_model.into();
        if let Some(d) = req.count_date {
            active.count_date = Set(d);
        }
        if let Some(n) = req.notes {
            active.notes = Set(Some(n));
        }
        active.updated_at = Set(Utc::now());
        let updated = AuditLogService::update_with_audit::<
            inventory_count::Entity,
            inventory_count::ActiveModel,
            _,
        >(&txn, "inventory_count", active, user_id)
        .await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 删除盘点单（仅 pending 状态可删除，连带删除明细）
    pub async fn delete_count(&self, count_id: i32) -> Result<(), AppError> {
        let txn = (*self.db).begin().await?;
        let count_model = inventory_count::Entity::find_by_id(count_id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("盘点单 {} 不存在", count_id)))?;
        if count_model.status != count_status::PENDING {
            return Err(AppError::business(
                "只有待盘点状态的盘点单可以删除".to_string(),
            ));
        }
        inventory_count_item::Entity::delete_many()
            .filter(inventory_count_item::Column::CountId.eq(count_id))
            .exec(&txn)
            .await?;
        inventory_count::Entity::delete_by_id(count_id)
            .exec(&txn)
            .await?;
        txn.commit().await?;
        Ok(())
    }

    /// 录入实盘数量并自动计算差异
    ///
    /// 差异 = 实盘 - 账面；同步更新盘点单 counted_items / variance_items 统计字段。
    /// 仅 pending 状态允许录入。
    pub async fn record_count_items(
        &self,
        count_id: i32,
        items: Vec<CountItemInput>,
    ) -> Result<CountDetail, AppError> {
        let txn = (*self.db).begin().await?;
        let count_model = inventory_count::Entity::find_by_id(count_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("盘点单 {} 不存在", count_id)))?;
        if count_model.status != count_status::PENDING {
            return Err(AppError::business(
                "只有待盘点状态的盘点单可以录入实盘数量".to_string(),
            ));
        }

        // 批量加载明细，避免循环内 N+1 查询
        let existing_items = inventory_count_item::Entity::find()
            .filter(inventory_count_item::Column::CountId.eq(count_id))
            .all(&txn)
            .await?;
        let mut item_map: std::collections::HashMap<i32, inventory_count_item::Model> =
            existing_items.into_iter().map(|it| (it.stock_id, it)).collect();

        let mut counted = 0i32;
        let mut variance = 0i32;
        for input in items {
            let item_model = item_map
                .get(&input.stock_id)
                .ok_or_else(|| AppError::not_found(format!("库存 {} 不在盘点明细中", input.stock_id)))?;
            let difference = input.quantity_actual - item_model.quantity_before;
            let mut active: inventory_count_item::ActiveModel = item_model.clone().into();
            active.quantity_actual = Set(input.quantity_actual);
            active.quantity_difference = Set(difference);
            active.notes = Set(input.notes.clone().or(item_model.notes.clone()));
            active.updated_at = Set(Utc::now());
            let _ = active.update(&txn).await?;
            counted += 1;
            if difference != Decimal::ZERO {
                variance += 1;
            }
            // 已处理的项从 map 移除，避免后续重复 update
            item_map.remove(&input.stock_id);
        }

        // 更新主表统计
        let mut count_active: inventory_count::ActiveModel = count_model.into();
        count_active.counted_items = Set(counted);
        count_active.variance_items = Set(variance);
        count_active.updated_at = Set(Utc::now());
        let updated_count = count_active.update(&txn).await?;

        txn.commit().await?;

        // 重新查询完整明细
        let items = inventory_count_item::Entity::find()
            .filter(inventory_count_item::Column::CountId.eq(count_id))
            .order_by(inventory_count_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;

        Ok(CountDetail {
            count: updated_count,
            items,
        })
    }

    /// 提交盘点单进入审批
    pub async fn submit_for_approval(
        &self,
        count_id: i32,
    ) -> Result<inventory_count::Model, AppError> {
        let txn = (*self.db).begin().await?;
        let count_model = inventory_count::Entity::find_by_id(count_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("盘点单 {} 不存在", count_id)))?;
        if count_model.status != count_status::PENDING {
            return Err(AppError::business("只有待盘点状态的盘点单可以提交审批".to_string()));
        }
        if count_model.counted_items == 0 {
            return Err(AppError::business("盘点单尚未录入任何实盘数量，无法提交审批".to_string()));
        }
        let mut active: inventory_count::ActiveModel = count_model.into();
        active.status = Set("in_review".to_string());
        active.updated_at = Set(Utc::now());
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 审批通过并完成盘点（同步更新库存数量）
    ///
    /// 审批通过后：
    /// 1. 盘点单状态 → approved → completed
    /// 2. 对每个有差异的明细，更新 inventory_stocks.quantity_on_hand
    /// 3. 记录 approved_by / approved_at / completed_at
    pub async fn approve_count(
        &self,
        count_id: i32,
        approver_id: i32,
    ) -> Result<inventory_count::Model, AppError> {
        let txn = (*self.db).begin().await?;
        let count_model = inventory_count::Entity::find_by_id(count_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("盘点单 {} 不存在", count_id)))?;
        if count_model.status != "in_review" {
            return Err(AppError::business("只有待审批状态的盘点单可以审批通过".to_string()));
        }

        let items = inventory_count_item::Entity::find()
            .filter(inventory_count_item::Column::CountId.eq(count_id))
            .all(&txn)
            .await?;

        // 批量加载涉及的库存记录
        let stock_ids: Vec<i32> = items.iter().map(|it| it.stock_id).collect();
        let stocks = if stock_ids.is_empty() {
            Vec::new()
        } else {
            inventory_stock::Entity::find()
                .filter(inventory_stock::Column::Id.is_in(stock_ids))
                .all(&txn)
                .await?
        };
        let stock_map: std::collections::HashMap<i32, inventory_stock::Model> =
            stocks.into_iter().map(|s| (s.id, s)).collect();

        // 对有差异的明细更新库存数量（使用乐观锁 version）
        for item in &items {
            if item.quantity_difference == Decimal::ZERO {
                continue;
            }
            let stock = stock_map
                .get(&item.stock_id)
                .ok_or_else(|| AppError::not_found(format!("库存 ID {} 不存在", item.stock_id)))?;
            let expected_version = stock.version;
            let result = inventory_stock::Entity::update_many()
                .col_expr(
                    inventory_stock::Column::QuantityOnHand,
                    Expr::val(item.quantity_actual).into(),
                )
                .col_expr(
                    inventory_stock::Column::QuantityAvailable,
                    Expr::val(item.quantity_actual).into(),
                )
                .col_expr(
                    inventory_stock::Column::LastCountDate,
                    Expr::val(Utc::now()).into(),
                )
                .col_expr(
                    inventory_stock::Column::Version,
                    Expr::col(inventory_stock::Column::Version).add(1),
                )
                .col_expr(
                    inventory_stock::Column::UpdatedAt,
                    Expr::val(Utc::now()).into(),
                )
                .filter(inventory_stock::Column::Id.eq(item.stock_id))
                .filter(inventory_stock::Column::Version.eq(expected_version))
                .exec(&txn)
                .await?;
            if result.rows_affected == 0 {
                return Err(AppError::business(format!(
                    "库存 ID {} 已被其他用户修改，请重试",
                    item.stock_id
                )));
            }
        }

        // 盘点单状态：approved → completed（审批后立即完成，简化流程）
        let mut active: inventory_count::ActiveModel = count_model.into();
        active.status = Set(count_status::COMPLETED.to_string());
        active.approved_by = Set(Some(approver_id));
        active.approved_at = Set(Some(Utc::now()));
        active.completed_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());
        let updated = AuditLogService::update_with_audit::<
            inventory_count::Entity,
            inventory_count::ActiveModel,
            _,
        >(&txn, "inventory_count", active, Some(approver_id))
        .await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 驳回审批，盘点单退回 pending 状态
    pub async fn reject_count(
        &self,
        count_id: i32,
    ) -> Result<inventory_count::Model, AppError> {
        let txn = (*self.db).begin().await?;
        let count_model = inventory_count::Entity::find_by_id(count_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("盘点单 {} 不存在", count_id)))?;
        if count_model.status != "in_review" {
            return Err(AppError::business("只有待审批状态的盘点单可以驳回".to_string()));
        }
        let mut active: inventory_count::ActiveModel = count_model.into();
        active.status = Set(count_status::PENDING.to_string());
        active.updated_at = Set(Utc::now());
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }
}

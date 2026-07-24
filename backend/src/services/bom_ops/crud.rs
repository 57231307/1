//! BOM Service CRUD 子模块（bom_ops/crud）
//!
//! 批次 D10 拆分：从原 `bom_service.rs` 迁移。
//! 包含 `BomService` 的 BOM 主表记录 CRUD + 版本/默认值管理方法：
//! - `create`：创建 BOM（含明细），事务包裹取消旧默认+建主表+批量插明细
//! - `get_by_id`：根据 ID 获取 BOM 详情（含明细）
//! - `list`：分页 + 过滤查询 BOM 列表
//! - `update`：更新 BOM（含明细替换），事务扩大到主表 update + 明细批量插入
//! - `delete`：软删除 BOM（状态置 INACTIVE）
//! - `get_versions`：获取 BOM 版本历史
//! - `copy`：复制 BOM（基于源 BOM 明细创建新版本）
//! - `get_next_version`：获取下一个版本号（私有 helper）
//! - `set_default`：设置默认 BOM（事务保护取消旧默认 + 设新默认）
//!
//! 跨模块调用：
//! - `create` 调用 facade 纯函数 `Self::cancel_existing_default_bom` / `Self::build_bom_item_models`（`pub(crate)`）
//! - `set_default` 内联取消旧默认逻辑（保持原实现，未复用 cancel_existing_default_bom）
//! - `update` 内联构建明细 ActiveModel（保持原逻辑，未复用 build_bom_item_models）

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};

use crate::models::bom::{
    ActiveModel, BomStatus, Column as BomColumn, Entity as BomEntity, Model as BomModel,
};
use crate::models::bom_item::{
    ActiveModel as BomItemActiveModel, Column as BomItemColumn, Entity as BomItemEntity,
};
use crate::services::bom_service::{
    BomDetail, BomQuery, BomService, CreateBomItemRequest, CreateBomRequest, UpdateBomRequest,
};
use crate::utils::error::AppError;

impl BomService {
    /// 创建BOM（含明细）
    ///
    /// 批次 203 P1-4 修复：原实现存在两个缺陷——整个方法无事务保护（主表插入、
    /// 默认取消、明细插入分散执行，若明细插入失败会留下无明细的脏 BOM）；
    /// 明细采用循环内逐条 `insert(&*self.db)`，N 条明细 = N 次 INSERT（N+1 写）。
    /// 现用事务包裹"取消旧默认 + 创建主表 + 批量插入明细"，明细改用 `insert_many`
    /// 单次 INSERT，并在事务内回查明细以构造 BomDetail 返回。
    pub async fn create(&self, req: CreateBomRequest) -> Result<BomDetail, AppError> {
        let txn = self.db.begin().await?;

        let version = if let Some(v) = req.version {
            v
        } else {
            self.get_next_version(req.product_id).await?
        };

        let is_default = req.is_default.unwrap_or(false);

        if is_default {
            Self::cancel_existing_default_bom(&txn, req.product_id).await?;
        }

        let bom_active_model = ActiveModel {
            product_id: Set(req.product_id),
            version: Set(version),
            is_default: Set(is_default),
            status: Set(BomStatus::Active.to_string()),
            remarks: Set(req.remarks),
            created_by: Set(req.created_by),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };

        let bom_model = bom_active_model.insert(&txn).await?;

        let item_active_models = Self::build_bom_item_models(bom_model.id, &req.items);
        if !item_active_models.is_empty() {
            BomItemEntity::insert_many(item_active_models)
                .exec(&txn)
                .await?;
        }

        // insert_many 不返回每条 Model，需在事务内回查明细以构造 BomDetail
        let items = BomItemEntity::find()
            .filter(BomItemColumn::BomId.eq(bom_model.id))
            .order_by_asc(BomItemColumn::SortOrder)
            .all(&txn)
            .await?;

        txn.commit().await?;

        Ok(BomDetail {
            bom: bom_model,
            items,
        })
    }

    /// 根据ID获取BOM详情
    pub async fn get_by_id(&self, id: i32) -> Result<Option<BomDetail>, AppError> {
        let bom_model = BomEntity::find_by_id(id).one(&*self.db).await?;

        match bom_model {
            Some(bom) => {
                let items = BomItemEntity::find()
                    .filter(BomItemColumn::BomId.eq(id))
                    .order_by_asc(BomItemColumn::SortOrder)
                    .all(&*self.db)
                    .await?;

                Ok(Some(BomDetail { bom, items }))
            }
            None => Ok(None),
        }
    }

    /// 获取BOM列表
    pub async fn list(&self, query: BomQuery) -> Result<(Vec<BomModel>, u64), AppError> {
        let mut select = BomEntity::find();

        if let Some(product_id) = query.product_id {
            select = select.filter(BomColumn::ProductId.eq(product_id));
        }

        if let Some(status) = query.status {
            select = select.filter(BomColumn::Status.eq(status));
        }

        if let Some(is_default) = query.is_default {
            select = select.filter(BomColumn::IsDefault.eq(is_default));
        }

        let total = select.clone().count(&*self.db).await?;

        let boms = select
            .order_by_desc(BomColumn::CreatedAt)
            .offset(query.page.saturating_sub(1) * query.page_size)
            .limit(query.page_size)
            .all(&*self.db)
            .await?;

        Ok((boms, total))
    }

    /// 更新BOM
    ///
    /// 批次 203 P1-4 修复：原实现存在两个缺陷——事务仅包裹"删除旧明细 + 创建新明细"
    /// （主表 update 在事务外，若明细插入失败，主表 update 不会回滚）；明细采用循环内
    /// 逐条 `insert(&txn)`，N 条明细 = N 次 INSERT（N+1 写）。现将事务范围扩大到包含
    /// 主表 update，明细改用 `insert_many` 单次 INSERT。
    pub async fn update(&self, id: i32, req: UpdateBomRequest) -> Result<BomDetail, AppError> {
        let txn = self.db.begin().await?;

        let bom_model = BomEntity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("BOM不存在"))?;

        let mut bom_active: ActiveModel = bom_model.into();

        if let Some(is_default) = req.is_default {
            bom_active.is_default = Set(is_default);
        }
        if let Some(status) = req.status {
            bom_active.status = Set(status);
        }
        if let Some(remarks) = req.remarks {
            bom_active.remarks = Set(Some(remarks));
        }
        bom_active.updated_at = Set(Utc::now());

        bom_active.update(&txn).await?;

        // 如果提供了新的明细，替换所有明细：改用 insert_many 批量 INSERT（原为循环内逐条 insert）
        if let Some(items_req) = req.items {
            // 删除旧明细
            BomItemEntity::delete_many()
                .filter(BomItemColumn::BomId.eq(id))
                .exec(&txn)
                .await?;

            // 创建新明细：批量插入
            let mut item_active_models: Vec<BomItemActiveModel> =
                Vec::with_capacity(items_req.len());
            for (index, item_req) in items_req.iter().enumerate() {
                item_active_models.push(BomItemActiveModel {
                    bom_id: Set(id),
                    material_id: Set(item_req.material_id),
                    quantity: Set(item_req.quantity),
                    unit: Set(item_req.unit.clone()),
                    scrap_rate: Set(item_req.scrap_rate),
                    sort_order: Set(Some(item_req.sort_order.unwrap_or(index as i32))),
                    created_at: Set(Utc::now()),
                    updated_at: Set(Utc::now()),
                    ..Default::default()
                });
            }

            if !item_active_models.is_empty() {
                BomItemEntity::insert_many(item_active_models)
                    .exec(&txn)
                    .await?;
            }
        }

        txn.commit().await?;

        // 返回更新后的详情
        self.get_by_id(id)
            .await?
            .ok_or_else(|| AppError::not_found("BOM不存在"))
    }

    /// 删除BOM（软删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let bom_model = BomEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("BOM不存在"))?;

        let mut bom_active: ActiveModel = bom_model.into();
        bom_active.status = Set(BomStatus::Inactive.to_string());
        bom_active.updated_at = Set(Utc::now());

        bom_active.update(&*self.db).await?;

        Ok(())
    }

    /// 获取BOM版本历史
    pub async fn get_versions(&self, product_id: i32) -> Result<Vec<BomModel>, AppError> {
        let boms = BomEntity::find()
            .filter(BomColumn::ProductId.eq(product_id))
            .order_by_desc(BomColumn::Version)
            .all(&*self.db)
            .await?;

        Ok(boms)
    }

    /// 复制BOM
    pub async fn copy(&self, id: i32, created_by: i32) -> Result<BomDetail, AppError> {
        let source = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| AppError::not_found("源BOM不存在"))?;

        let new_version = self.get_next_version(source.bom.product_id).await?;

        let items: Vec<CreateBomItemRequest> = source
            .items
            .iter()
            .map(|item| CreateBomItemRequest {
                material_id: item.material_id,
                quantity: item.quantity,
                unit: item.unit.clone(),
                scrap_rate: item.scrap_rate,
                sort_order: item.sort_order,
            })
            .collect();

        self.create(CreateBomRequest {
            product_id: source.bom.product_id,
            version: Some(new_version),
            is_default: Some(false),
            remarks: source.bom.remarks,
            created_by,
            items,
        })
        .await
    }

    /// 获取下一个版本号
    async fn get_next_version(&self, product_id: i32) -> Result<i32, AppError> {
        let latest = BomEntity::find()
            .filter(BomColumn::ProductId.eq(product_id))
            .order_by_desc(BomColumn::Version)
            .one(&*self.db)
            .await?;

        Ok(match latest {
            Some(bom) => bom.version + 1,
            None => 1,
        })
    }

    /// 设置默认BOM
    pub async fn set_default(&self, id: i32) -> Result<BomModel, AppError> {
        let bom_model = BomEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("BOM不存在"))?;

        // 使用事务保护取消旧默认和设置新默认操作
        let txn = self.db.begin().await?;

        // 取消同产品其他默认BOM
        BomEntity::update_many()
            .filter(BomColumn::ProductId.eq(bom_model.product_id))
            .filter(BomColumn::IsDefault.eq(true))
            .set(ActiveModel {
                is_default: Set(false),
                updated_at: Set(Utc::now()),
                ..Default::default()
            })
            .exec(&txn)
            .await?;

        // 设置当前BOM为默认
        let mut bom_active: ActiveModel = bom_model.into();
        bom_active.is_default = Set(true);
        bom_active.updated_at = Set(Utc::now());

        let updated_bom = bom_active.update(&txn).await?;

        txn.commit().await?;

        Ok(updated_bom)
    }
}

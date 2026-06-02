//! BOM物料清单 Service
//!
//! 提供BOM的CRUD操作、版本管理和树形结构查询

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;

use crate::models::bom::{
    ActiveModel, BomStatus, Column as BomColumn, Entity as BomEntity, Model as BomModel,
};
use crate::models::bom_item::{
    ActiveModel as BomItemActiveModel, Column as BomItemColumn, Entity as BomItemEntity,
    Model as BomItemModel,
};
use crate::utils::error::AppError;

/// 创建BOM请求
#[derive(Debug, Clone)]
pub struct CreateBomRequest {
    pub product_id: i32,
    pub version: Option<i32>,
    pub is_default: Option<bool>,
    pub remarks: Option<String>,
    pub created_by: i32,
    pub items: Vec<CreateBomItemRequest>,
}

/// 创建BOM明细请求
#[derive(Debug, Clone)]
pub struct CreateBomItemRequest {
    pub material_id: i32,
    pub quantity: Decimal,
    pub unit: Option<String>,
    pub scrap_rate: Option<Decimal>,
    pub sort_order: Option<i32>,
}

/// 更新BOM请求
#[derive(Debug, Clone)]
pub struct UpdateBomRequest {
    pub is_default: Option<bool>,
    pub status: Option<String>,
    pub remarks: Option<String>,
    pub items: Option<Vec<CreateBomItemRequest>>,
}

/// BOM查询参数
#[derive(Debug, Clone)]
pub struct BomQuery {
    pub product_id: Option<i32>,
    pub status: Option<String>,
    pub is_default: Option<bool>,
    pub page: u64,
    pub page_size: u64,
}

/// BOM详情（含明细）
#[derive(Debug, Clone, serde::Serialize)]
pub struct BomDetail {
    pub bom: BomModel,
    pub items: Vec<BomItemModel>,
}

/// BOM Service
pub struct BomService {
    db: Arc<DatabaseConnection>,
}

impl BomService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建BOM（含明细）
    pub async fn create(&self, req: CreateBomRequest) -> Result<BomDetail, AppError> {
        // 计算版本号
        let version = if let Some(v) = req.version {
            v
        } else {
            self.get_next_version(req.product_id).await?
        };

        let is_default = req.is_default.unwrap_or(false);

        // 如果设置为默认，先取消同产品其他默认BOM
        if is_default {
            BomEntity::update_many()
                .filter(BomColumn::ProductId.eq(req.product_id))
                .filter(BomColumn::IsDefault.eq(true))
                .set(ActiveModel {
                    is_default: Set(false),
                    updated_at: Set(Utc::now()),
                    ..Default::default()
                })
                .exec(&*self.db)
                .await?;
        }

        // 创建BOM主记录
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

        let bom_model = bom_active_model.insert(&*self.db).await?;

        // 创建BOM明细
        let mut items = Vec::new();
        for (index, item_req) in req.items.iter().enumerate() {
            let item_active_model = BomItemActiveModel {
                bom_id: Set(bom_model.id),
                material_id: Set(item_req.material_id),
                quantity: Set(item_req.quantity),
                unit: Set(item_req.unit.clone()),
                scrap_rate: Set(item_req.scrap_rate),
                sort_order: Set(Some(item_req.sort_order.unwrap_or(index as i32))),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
                ..Default::default()
            };

            let item_model = item_active_model.insert(&*self.db).await?;

            items.push(item_model);
        }

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
            .offset((query.page - 1) * query.page_size)
            .limit(query.page_size)
            .all(&*self.db)
            .await?;

        Ok((boms, total))
    }

    /// 更新BOM
    pub async fn update(&self, id: i32, req: UpdateBomRequest) -> Result<BomDetail, AppError> {
        let bom_model = BomEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound("BOM不存在".to_string()))?;

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

        let _updated_bom = bom_active.update(&*self.db).await?;

        // 如果提供了新的明细，替换所有明细
        if let Some(items_req) = req.items {
            // 使用事务保护删除和创建操作
            let txn = self.db.begin().await?;

            // 删除旧明细
            BomItemEntity::delete_many()
                .filter(BomItemColumn::BomId.eq(id))
                .exec(&txn)
                .await?;

            // 创建新明细
            for (index, item_req) in items_req.iter().enumerate() {
                let item_active_model = BomItemActiveModel {
                    bom_id: Set(id),
                    material_id: Set(item_req.material_id),
                    quantity: Set(item_req.quantity),
                    unit: Set(item_req.unit.clone()),
                    scrap_rate: Set(item_req.scrap_rate),
                    sort_order: Set(Some(item_req.sort_order.unwrap_or(index as i32))),
                    created_at: Set(Utc::now()),
                    updated_at: Set(Utc::now()),
                    ..Default::default()
                };

                item_active_model.insert(&txn).await?;
            }

            txn.commit().await?;
        }

        // 返回更新后的详情
        self.get_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("BOM不存在".to_string()))
    }

    /// 删除BOM（软删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let bom_model = BomEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::NotFound("BOM不存在".to_string()))?;

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
            .ok_or_else(|| AppError::NotFound("源BOM不存在".to_string()))?;

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
            .ok_or_else(|| AppError::NotFound("BOM不存在".to_string()))?;

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

    /// 获取BOM树形结构（递归展开）
    pub fn get_bom_tree(
        &self,
        bom_id: i32,
        max_depth: Option<i32>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<BomTreeNode, AppError>> + Send + '_>,
    > {
        Box::pin(async move {
            let bom = BomEntity::find_by_id(bom_id)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::NotFound("BOM不存在".to_string()))?;

            let items = BomItemEntity::find()
                .filter(BomItemColumn::BomId.eq(bom_id))
                .order_by_asc(BomItemColumn::SortOrder)
                .all(&*self.db)
                .await?;

            let mut children = Vec::new();
            let depth = max_depth.unwrap_or(10);

            if depth > 0 {
                for item in &items {
                    // 递归查找子物料的BOM
                    let child_bom = BomEntity::find()
                        .filter(BomColumn::ProductId.eq(item.material_id))
                        .filter(BomColumn::IsDefault.eq(true))
                        .filter(BomColumn::Status.eq("ACTIVE"))
                        .one(&*self.db)
                        .await?;

                    let child_node = if let Some(child_bom) = child_bom {
                        // 递归展开子BOM
                        self.get_bom_tree(child_bom.id, Some(depth - 1)).await?
                    } else {
                        // 叶子节点
                        BomTreeNode {
                            id: format!("item-{}", item.id),
                            product_id: item.material_id,
                            product_name: format!("物料 #{}", item.material_id),
                            quantity: item.quantity,
                            unit: item.unit.clone(),
                            scrap_rate: item.scrap_rate,
                            children: vec![],
                        }
                    };

                    children.push(child_node);
                }
            }

            Ok(BomTreeNode {
                id: format!("bom-{}", bom.id),
                product_id: bom.product_id,
                product_name: format!("产品 #{}", bom.product_id),
                quantity: Decimal::ONE,
                unit: None,
                scrap_rate: None,
                children,
            })
        })
    }

    /// 获取BOM用量计算（多层级）
    pub async fn calculate_bom_requirements(
        &self,
        bom_id: i32,
        quantity: Decimal,
    ) -> Result<Vec<BomRequirement>, AppError> {
        let tree = self.get_bom_tree(bom_id, Some(10)).await?;
        let mut requirements = Vec::new();
        self.collect_requirements(&tree, quantity, &mut requirements);
        Ok(requirements)
    }

    /// 递归收集BOM需求
    fn collect_requirements(
        &self,
        node: &BomTreeNode,
        parent_quantity: Decimal,
        requirements: &mut Vec<BomRequirement>,
    ) {
        let required_quantity = parent_quantity * node.quantity;
        let scrap_multiplier = match node.scrap_rate {
            Some(rate) if rate > Decimal::ZERO => Decimal::ONE + (rate / Decimal::from(100)),
            _ => Decimal::ONE,
        };
        let actual_quantity = required_quantity * scrap_multiplier;

        if node.children.is_empty() {
            // 叶子节点，添加到需求列表
            requirements.push(BomRequirement {
                product_id: node.product_id,
                product_name: node.product_name.clone(),
                required_quantity: actual_quantity,
                unit: node.unit.clone(),
            });
        } else {
            // 递归处理子节点
            for child in &node.children {
                self.collect_requirements(child, actual_quantity, requirements);
            }
        }
    }
}

/// BOM树节点
#[derive(Debug, Clone, serde::Serialize)]
pub struct BomTreeNode {
    pub id: String,
    pub product_id: i32,
    pub product_name: String,
    pub quantity: Decimal,
    pub unit: Option<String>,
    pub scrap_rate: Option<Decimal>,
    pub children: Vec<BomTreeNode>,
}

/// BOM需求项
#[derive(Debug, Clone, serde::Serialize)]
pub struct BomRequirement {
    pub product_id: i32,
    pub product_name: String,
    pub required_quantity: Decimal,
    pub unit: Option<String>,
}

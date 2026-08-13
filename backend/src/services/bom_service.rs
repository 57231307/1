//! BOM物料清单 Service（facade）
//!
//! 提供BOM的CRUD操作、版本管理、状态流转和树形结构查询。
//!
//! # 模块拆分说明
//! 本文件为 facade，仅保留：
//! - DTO struct（CreateBomRequest / CreateBomItemRequest / UpdateBomRequest / BomQuery / BomDetail）
//! - BOM 树与需求结果 struct（BomTreeNode / BomRequirement）
//! - `BomService` struct 定义与 `new` 构造函数
//! - 纯函数（无 `&self` / 无 db 访问）：`cancel_existing_default_bom` / `build_bom_item_models` / `build_leaf_bom_node`
//! - 测试模块
//!
//! 业务 impl 块已按职责拆分到 [`crate::services::bom_ops`] 子模块：
//! - `crud`：BOM 主表 CRUD + 版本/默认值管理
//! - `state`：状态机流转（submit/approve，lock_exclusive 串行化并发状态变更）
//! - `tree`：树形结构查询与多层级用量计算
//!
//! `db` 字段声明为 `pub(crate)` 以便 `bom_ops` 子模块的 `impl BomService` 块直接访问；
//! 纯函数声明为 `pub(crate)` 供 `bom_ops` 子模块跨模块调用（`Self::xxx`）。

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use std::sync::Arc;

use crate::models::bom::{
    ActiveModel, Column as BomColumn, Entity as BomEntity, Model as BomModel,
};
use crate::models::bom_item::{ActiveModel as BomItemActiveModel, Model as BomItemModel};
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

/// BOM Service（字段声明为 `pub(crate)` 以便 `bom_ops` 子模块的 `impl BomService` 块直接访问；（业务方法已迁移至 `bom_ops::{crud,state,tree}`）。）
pub struct BomService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl BomService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 取消同产品其他默认 BOM（事务内执行）
    /// 纯函数（无 `&self`）：在事务内将同产品已存在的默认 BOM 的 `is_default` 置 false，；供 `bom_ops::crud` 的 create 调用（set_default 内联实现，未复用本函数）。
    pub(crate) async fn cancel_existing_default_bom(
        txn: &sea_orm::DatabaseTransaction,
        product_id: i32,
    ) -> Result<(), AppError> {
        BomEntity::update_many()
            .filter(BomColumn::ProductId.eq(product_id))
            .filter(BomColumn::IsDefault.eq(true))
            .set(ActiveModel {
                is_default: Set(false),
                updated_at: Set(Utc::now()),
                ..Default::default()
            })
            .exec(txn)
            .await?;
        Ok(())
    }

    /// 构建 BOM 明细 ActiveModel 列表（批量插入用）
    /// 纯函数（无 `&self`）：按 `CreateBomItemRequest` 列表构造 `BomItemActiveModel` 列表，；sort_order 缺省时取索引下标。供 `bom_ops::crud` 的 create 调用。
    pub fn build_bom_item_models(
        bom_id: i32,
        items: &[CreateBomItemRequest],
    ) -> Vec<BomItemActiveModel> {
        items
            .iter()
            .enumerate()
            .map(|(index, item_req)| BomItemActiveModel {
                bom_id: Set(bom_id),
                material_id: Set(item_req.material_id),
                quantity: Set(item_req.quantity),
                unit: Set(item_req.unit.clone()),
                scrap_rate: Set(item_req.scrap_rate),
                sort_order: Set(Some(item_req.sort_order.unwrap_or(index as i32))),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
                ..Default::default()
            })
            .collect()
    }

    /// 构建叶子节点 BomTreeNode
    /// 纯函数（无 `&self`）：由 BOM 明细行构造无子节点的 `BomTreeNode`，；供 `bom_ops::tree` 的 get_bom_tree 在子物料无默认 BOM 时调用。
    pub fn build_leaf_bom_node(item: &BomItemModel) -> BomTreeNode {
        BomTreeNode {
            id: format!("item-{}", item.id),
            product_id: item.material_id,
            product_name: format!("物料 #{}", item.material_id),
            quantity: item.quantity,
            unit: item.unit.clone(),
            scrap_rate: item.scrap_rate,
            children: vec![],
        }
    }
}

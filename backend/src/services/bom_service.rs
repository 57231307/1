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
    pub items: Vec<CreateBomItemRequest>,
}

/// 创建BOM明细请求（scrap_rate 为**存储口径**十进制裁损率 0–1，
/// 由 handler 边界经 [`BomService::scrap_percent_to_ratio`] 从 API 百分比数值换算而来；
/// 服务层内部（含 `copy`）流转的本结构一律保持存储口径，禁止二次换算）
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
    /// 产品名称（来自 products.name 批量查询；查不到为 None，前端渲染空白）
    pub product_name: Option<String>,
    pub quantity: Decimal,
    pub unit: Option<String>,
    /// 结构体字段 = 存储口径 0–1 比率（`collect_requirements` 按 (1+rate) 消费）；
    /// 仅序列化输出经 `scrap_ratio_to_percent` 转 API 百分比口径（与写边界对称）
    #[serde(serialize_with = "serialize_scrap_rate_as_percent")]
    pub scrap_rate: Option<Decimal>,
    pub children: Vec<BomTreeNode>,
}

/// `BomTreeNode.scrap_rate` 的读边界序列化：存储比率 → 百分比数值（None 保持 None）
fn serialize_scrap_rate_as_percent<S: serde::Serializer>(
    value: &Option<Decimal>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use serde::Serialize;
    BomService::scrap_ratio_to_percent(*value).serialize(serializer)
}

/// BOM需求项
#[derive(Debug, Clone, serde::Serialize)]
pub struct BomRequirement {
    pub product_id: i32,
    /// 产品名称（随树节点透传；查不到为 None）
    pub product_name: Option<String>,
    pub required_quantity: Decimal,
    pub unit: Option<String>,
}

/// BOM导出视图对象（LEFT JOIN products 富化产品编码/名称）
#[derive(Debug, Clone, sea_orm::FromQueryResult)]
pub struct BomExportDto {
    pub id: i32,
    pub product_id: i32,
    pub product_code: Option<String>,
    pub product_name: Option<String>,
    pub version: i32,
    pub is_default: bool,
    pub status: String,
    pub remarks: Option<String>,
    pub created_by: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
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

    /// 损耗率口径换算（写边界）：API 入参百分比数值 → 存储口径十进制比率。
    ///
    /// DDL 证据：`bom_items.scrap_rate DECIMAL(5, 4) DEFAULT 0`
    /// （`backend/migration/src/domain/business/m0007_add_mrp_production_bom.rs:41`），
    /// 存储口径为 0–1 十进制比率（`models/bom_item.rs` 字段注释 `(0-1)` 同源；
    /// p=5、s=4 ⇒ 整数位仅 1 位，≥10 的值直插必触发 PG `numeric field overflow`，
    /// CI e2e/mrp/01:82 即写入侧未换算所致）。
    /// API/业务口径为百分比数值（10 表示 10%，见 frontend/e2e/mrp/01-calculation.spec.ts
    /// 入参与损耗乘数语义），写入前必须经本函数换算，禁止把百分比数值直接落库。
    /// 越界（<0 或 >100%）与超精度（百分比小数位 >2，即 0.01% 粒度）一律
    /// fail-closed 抛 VALIDATION_ERROR 点名实值，禁止 clamp/截断/静默舍入。
    pub fn scrap_percent_to_ratio(
        scrap_percent: Option<Decimal>,
    ) -> Result<Option<Decimal>, AppError> {
        let Some(percent) = scrap_percent else {
            return Ok(None);
        };
        if percent < Decimal::ZERO || percent > Decimal::from(100) {
            return Err(AppError::validation_displayable(format!(
                "BOM 损耗率（百分比数值）必须在 0–100 范围内，实际: {percent}"
            )));
        }
        let ratio = percent / Decimal::from(100);
        let storage = ratio.round_dp(4);
        if storage != ratio {
            return Err(AppError::validation_displayable(format!(
                "BOM 损耗率最多保留两位小数（0.01% 粒度），实际: {percent}"
            )));
        }
        Ok(Some(storage))
    }

    /// 损耗率口径换算（读边界/回显）：存储口径十进制比率 → API 百分比数值。
    ///
    /// [`Self::scrap_percent_to_ratio`] 的精确逆函数：合法存储值由 DDL
    /// `DECIMAL(5,4)` 保证为 0.0001 的整数倍，×100 后必为 0.01 的整数倍，
    /// `round_dp(2)` 无损失（如 0.1000 → 10.00、0.0333 → 3.33）。
    /// API 对外口径 = 百分比数值（写入/回显同一口径，同一字段名禁止两种口径；
    /// 依据：e2e `frontend/e2e/mrp/01-calculation.spec.ts:95`「scrap_rate=10 ⇒
    /// 乘数 1+10/100」），故 handler 响应与树端点序列化输出必须经本函数换算，
    /// 而服务内部计算链（`collect_requirements`、MRP 展开、缺料测算）
    /// 一律消费存储比率，不换算。
    pub fn scrap_ratio_to_percent(ratio: Option<Decimal>) -> Option<Decimal> {
        ratio.map(|r| (r * Decimal::from(100)).round_dp(2))
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
                // 显式初始化：bom_items.is_deleted 列无 DB 默认值，NotSet 会报 Missing value
                is_deleted: Set(false),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
                ..Default::default()
            })
            .collect()
    }

    /// 构建叶子节点 BomTreeNode
    /// 纯函数（无 `&self`）：由 BOM 明细行构造无子节点的 `BomTreeNode`，名称由调用方从
    /// products 批量查询结果传入（查不到时为 `None`）；供 `bom_ops::tree` 的 get_bom_tree
    /// 在子物料无默认 BOM 时调用。
    pub fn build_leaf_bom_node(item: &BomItemModel, product_name: Option<String>) -> BomTreeNode {
        BomTreeNode {
            id: format!("item-{}", item.id),
            product_id: item.material_id,
            product_name,
            quantity: item.quantity,
            unit: item.unit.clone(),
            scrap_rate: item.scrap_rate,
            children: vec![],
        }
    }
}

//! 库存服务模块（inv = inventory）
//!
//! 由原 `services/inventory_transfer_service.rs`（1202 行）按业务子领域拆分而来。
//! 子模块：
//! - `move`   调拨单主流程（CRUD / 审核 / 状态机 / 单据号生成）
//! - `batch`  调拨单明细行管理 + 发出/接收批次处理（库存扣减/增加）
//! - `stock`  库存检查辅助逻辑
//! - `adjust` 库存调整（占位模块，详见 `services/inventory_adjustment_service.rs`）
//! - `count`  库存盘点（占位模块，详见 `services/inventory_count_service.rs`）
//! - `hold`   库存预留（占位模块，详见 `services/inventory_reservation_service.rs`）
//! - `fabric_class` 纺织白坯/染色判定与追溯字段校验（全仓唯一实现）
//!
//! 兼容说明：原 `crate::services::inv::*` 路径需要由上层
//! `services/mod.rs` 通过 `pub use super::inv::*;` 重新导出以保持向后兼容。
//!
//! 注意：`move` 与 `return` 同为 Rust 关键字，不能直接作为模块名。
//! 实际文件名为 `inventory_move.rs`（参考 `return_rs.rs` 的命名约定），通过 `as` 别名对外暴露。

use sea_orm::{DatabaseConnection, FromQueryResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod adjust;
pub mod batch;
pub mod count;
pub mod fabric_class;
pub mod hold;
pub mod inventory_move;
pub mod stock;

// =====================================================
// 共享 DTO（与原 inventory_transfer_service.rs 保持一致）
// =====================================================

/// 库存调拨详情响应
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InventoryTransferDetail {
    pub id: i32,
    pub transfer_no: String,
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub transfer_date: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub total_quantity: rust_decimal::Decimal,
    /// 调拨总金额（对应 `inventory_transfer.total_amount`，实体 NOT NULL，默认 0）
    pub total_amount: rust_decimal::Decimal,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub shipped_at: Option<chrono::DateTime<chrono::Utc>>,
    pub received_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// 调出仓库名（LEFT JOIN warehouses 于 from_warehouse_id 取得）
    pub from_warehouse_name: Option<String>,
    /// 调入仓库名（LEFT JOIN warehouses 于 to_warehouse_id 取得）
    pub to_warehouse_name: Option<String>,
    /// 创建人姓名（LEFT JOIN users 于 created_by 取得 `real_name`）
    pub created_by_name: Option<String>,
    pub items: Vec<InventoryTransferItemDetail>,
}

/// 库存调拨表头读模型（单次 JOIN 富化的查询结果）。
///
/// 与 [`InventoryTransferDetail`] 同构但不含 `items`：明细在详情端点单独查询，
/// 列表端点 `items` 恒为空数组，因此表头查询用本视图 `into_model` 落地，
/// 再组装为 [`InventoryTransferDetail`]。JOIN 派生列一律 `Option<String>`。
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct InventoryTransferView {
    pub id: i32,
    pub transfer_no: String,
    pub from_warehouse_id: i32,
    pub to_warehouse_id: i32,
    pub transfer_date: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub total_quantity: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub notes: Option<String>,
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub shipped_at: Option<chrono::DateTime<chrono::Utc>>,
    pub received_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub from_warehouse_name: Option<String>,
    pub to_warehouse_name: Option<String>,
    pub created_by_name: Option<String>,
}

/// 库存调拨明细项详情
#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
pub struct InventoryTransferItemDetail {
    pub id: i32,
    pub transfer_id: i32,
    pub product_id: i32,
    pub quantity: rust_decimal::Decimal,
    pub shipped_quantity: rust_decimal::Decimal,
    pub received_quantity: rust_decimal::Decimal,
    pub unit_cost: Option<rust_decimal::Decimal>,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    // 面料行业追溯字段（v14 批次 417 T-P0-1）：入参已收、库中已存，出参必须如实回传
    pub color_no: String,
    pub dye_lot_no: Option<String>,
    pub batch_no: String,
    /// 匹号（出库四维第四维，用户 2026-10-02 纠正口径；白坯布合法为 NULL，m0066 补列）
    pub piece_no: Option<String>,
    // 由 LEFT JOIN products 富化（实体仅存 product_id）；JOIN 派生列一律 Option<String>
    pub product_code: Option<String>,
    pub product_name: Option<String>,
    /// 产品等级（products.product_grade）
    pub grade: Option<String>,
    /// 计量单位（products.unit）
    pub unit: Option<String>,
}

/// 创建库存调拨请求
#[derive(Debug, Deserialize)]
pub struct CreateInventoryTransferRequest {
    pub from_warehouse_id: Option<i32>,
    pub to_warehouse_id: Option<i32>,
    pub transfer_date: Option<chrono::DateTime<chrono::Utc>>,
    /// 状态：只接受权威词表内的**初始状态** pending（键缺席由服务落缺省 pending）。
    /// 其它词表内取值（含 approved/shipped/completed）在建单口一律 400 拒绝——
    /// 状态推进归审批/发货/收货三个权威操作，见
    /// `inventory_move.rs::validate_transfer_initial_status`。
    pub status: Option<String>,
    pub notes: Option<String>,
    pub items: Option<Vec<InventoryTransferItemRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct InventoryTransferItemRequest {
    pub product_id: Option<i32>,
    pub quantity: Option<rust_decimal::Decimal>,
    pub notes: Option<String>,
    // P1 batch-18 缺陷 6.2：调拨明细强制缸号追溯字段
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub batch_no: Option<String>,
    // 出库四维补强（用户 2026-10-02 纠正口径：染色布出库强制 缸号/色号/批次/匹号）：
    // 匹号为第四维，染色布必填（fabric_class::normalize_outbound_piece_no 唯一判定），
    // 白坯布免填归一 NULL；是否命中真实可用库存匹由 piece_domain_service 校验（BUSINESS 族）
    pub piece_no: Option<String>,
    pub unit_cost: Option<rust_decimal::Decimal>,
}

/// JSON 三态反序列化适配器（RFC 7386 JSON Merge Patch 的"键缺席 ≠ 显式 null"语义所需）。
///
/// 为何需要：serde_json 对 `Option<Option<T>>` 的默认反序列化在遇到 JSON null 时
/// 直接调 visit_none()，把"显式 null"塌成外层 `None`，与"键缺席"不可区分。
/// 本适配器把字段先按内层 `Option<T>` 反序列化再包一层：
/// 键缺席（配合 `#[serde(default)]`）= `None`、显式 null = `Some(None)`、有值 = `Some(Some(v))`。
/// 与 handlers/department_handler.rs 中同名适配器形状一致（跨域合并到共享工具需动
/// utils，超出本批授权范围；本 DTO 为 wire 直连，适配器落位服务侧同文件）。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// 更新库存调拨请求
///
/// 字段语义 = 显式三态部分更新（对齐 RFC 7386 JSON Merge Patch）：
/// 键缺席=保持原值、显式 `null`=清空为 NULL（仅 DB 可空列）、有值=覆盖。
/// status 对应实体 Model 非 Option 列（置 NULL 该行按模型不可读）：显式 null 由 service 拒绝。
/// 且 status 是**状态机位不是可编辑字段**：本端点的合法流转集合为空——只有与当前状态
/// 逐字符相同的幂等写入放行，任何异值（词表内或词表外）都 400 BUSINESS_ERROR
/// （`inventory_move.rs::validate_transfer_status_write`：审批/发货/收货各自还要落
/// 审批人、扣减库存、写流水与事件，只改一列状态会造出账实分裂）。
/// items 为明细整表替换数组：不开放"显式 null 清全表"，清空明细须传空数组 `[]`
/// （与销售合同 UpdateSalesContractDto.items 同口径）。
#[derive(Debug, Deserialize)]
pub struct UpdateInventoryTransferRequest {
    /// 状态（映射非 Option 列 status）——显式 null 被 service 拒绝；
    /// 异值（=试图经编辑口流转状态）同样被拒绝，只放行与当前值的幂等写入
    #[serde(default, deserialize_with = "double_option")]
    pub status: Option<Option<String>>,
    /// 备注：DB 可空列 notes TEXT（m0001 DDL）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
    pub items: Option<Vec<InventoryTransferItemRequest>>,
}

/// 更新调拨单明细请求（PUT /inventory/transfers/items/{item_id}）
///
/// 字段语义 = 显式三态部分更新（对齐 RFC 7386 JSON Merge Patch）：
/// 键缺席=保持原值、显式 `null`=清空为 NULL（仅 DB 可空列）、有值=覆盖。
/// 与创建口径的 [`InventoryTransferItemRequest`]（单 Option，缺席=未提供）分型：
/// 更新端点必须能区分"没送"与"送了 null"，故独立 DTO。
/// NOT NULL 列（inventory_transfer_item.product_id/quantity/color_no/batch_no，
/// models/inventory_transfer_item.rs 模型为非 Option 列）不开 null 清空，
/// 显式 null 由 service 在任何 DB 访问前拒绝；可空列 notes/unit_cost 开放置 NULL。
/// `dye_lot_no` 模型虽是 Option，但列 DDL 是 `NOT NULL DEFAULT ''`
/// （migration/src/domain/system/mod.rs:292，同迁移已把历史 NULL 回填 ''），
/// 故它的"清空"落空串（DB 里"无缸号"的合法表示即 ''），绝不能落 NULL。
#[derive(Debug, Deserialize)]
pub struct UpdateInventoryTransferItemRequest {
    /// 产品 ID：NOT NULL 列——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub product_id: Option<Option<i32>>,
    /// 数量：NOT NULL 列——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub quantity: Option<Option<rust_decimal::Decimal>>,
    /// 备注：DB 可空列 notes——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
    /// 单位成本：DB 可空列 unit_cost——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub unit_cost: Option<Option<rust_decimal::Decimal>>,
    /// 色号：NOT NULL 列（空串=白坯合法值）——显式 null 被 service 拒绝，"改回白坯"提交空串
    #[serde(default, deserialize_with = "double_option")]
    pub color_no: Option<Option<String>>,
    /// 缸号：DB 可空列 dye_lot_no（白坯布为合法 NULL）——显式 null 清空；
    /// 染色布（生效色号非空）行清空缸号违反四维追溯不变量，由 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub dye_lot_no: Option<Option<String>>,
    /// 批次：NOT NULL 列——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub batch_no: Option<Option<String>>,
    /// 匹号：DB 可空列 piece_no（白坯布为合法 NULL，m0066）——显式 null 清空仅对白坯行合法；
    /// 染色布（生效色号非空）行清空匹号违反出库四维不变量，由 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub piece_no: Option<Option<String>>,
}

// =====================================================
// 共享 Service 结构体（子模块均通过 impl InventoryTransferService 扩展）
// =====================================================

/// 库存调拨服务
pub struct InventoryTransferService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl InventoryTransferService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

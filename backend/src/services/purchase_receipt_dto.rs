//! 采购入库服务请求/响应 DTO 结构体
//!
//! 拆分自 purchase_receipt_service.rs：原 4 个 DTO 独立成文件。

use rust_decimal::Decimal;
use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 采购入库单视图对象（列表/详情出参）
///
/// 本表自身字段 + LEFT JOIN 富化字段（supplier_name / warehouse_name /
/// purchase_order_no / created_by_name），参照 `PurchaseOrderDto` 范式实现单次查询无 N+1。
#[derive(Debug, Clone, FromQueryResult, Serialize)]
pub struct PurchaseReceiptDto {
    pub id: i32,
    pub receipt_no: String,
    pub order_id: Option<i32>,
    pub supplier_id: i32,
    pub receipt_date: chrono::NaiveDate,
    pub warehouse_id: i32,
    pub department_id: Option<i32>,
    pub receiver_id: Option<i32>,
    pub inspector_id: Option<i32>,
    pub inspection_status: String,
    pub receipt_status: String,
    pub total_quantity: Decimal,
    pub total_quantity_alt: Decimal,
    pub total_amount: Decimal,
    pub notes: Option<String>,
    pub attachment_urls: Option<Vec<String>>,
    pub created_by: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_by: Option<i32>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub confirmed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub confirmed_by: Option<i32>,

    /// 供应商名称：supplier_id -> suppliers.supplier_name（LEFT JOIN）
    pub supplier_name: Option<String>,
    /// 仓库名称：warehouse_id -> warehouses.name（LEFT JOIN）
    pub warehouse_name: Option<String>,
    /// 采购订单号：order_id -> purchase_orders.order_no（LEFT JOIN，无关联订单时为 NULL）
    pub purchase_order_no: Option<String>,
    /// 创建人姓名：created_by -> users.real_name（LEFT JOIN）
    pub created_by_name: Option<String>,
}

/// 创建采购入库单请求
#[derive(Debug, Validate, Deserialize)]
pub struct CreatePurchaseReceiptRequest {
    /// 采购订单 ID
    pub order_id: Option<i32>,

    /// 供应商 ID
    pub supplier_id: i32,

    /// 入库日期
    pub receipt_date: chrono::NaiveDate,

    /// 仓库 ID
    pub warehouse_id: i32,

    /// 部门 ID
    pub department_id: Option<i32>,

    /// 质检员 ID
    pub inspector_id: Option<i32>,

    /// 备注
    pub notes: Option<String>,

    /// 附件 URL 列表
    pub attachment_urls: Option<Vec<String>>,

    /// 入库明细
    #[validate(length(min = 1, message = "入库单至少需要一行明细"))]
    pub items: Vec<CreateReceiptItemRequest>,
}

/// JSON 三态反序列化适配器（RFC 7386 JSON Merge Patch 的"键缺席 ≠ 显式 null"语义所需）。
///
/// 为何需要：serde_json 对 `Option<Option<T>>` 的默认反序列化在遇到 JSON null 时
/// 直接调 visit_none()，把"显式 null"塌成外层 `None`，与"键缺席"不可区分。
/// 本适配器把字段先按内层 `Option<T>` 反序列化再包一层：
/// 键缺席（配合 `#[serde(default)]`）= `None`、显式 null = `Some(None)`、有值 = `Some(Some(v))`。
/// 与 handlers/department_handler.rs 中同名适配器形状一致（跨域合并到共享工具需动
/// utils，超出本批授权范围；本文件 DTO 为 wire 直连，适配器落位 DTO 同文件）。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// 更新采购入库单请求
///
/// 字段语义 = 显式三态部分更新（对齐 RFC 7386 JSON Merge Patch）：
/// 键缺席=保持原值、显式 `null`=清空为 NULL（仅 DB 可空列）、有值=覆盖。
/// NOT NULL 列（purchase_receipt.supplier_id/receipt_date，m0009 DDL）不开 null 清空，
/// 显式 null 由 service 拒绝。
#[derive(Debug, Default, Deserialize)]
pub struct UpdatePurchaseReceiptRequest {
    /// NOT NULL 列 supplier_id（m0009 DDL）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub supplier_id: Option<Option<i32>>,
    /// NOT NULL 列 receipt_date（m0009 DDL）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub receipt_date: Option<Option<chrono::NaiveDate>>,
    /// DB 可空列 department_id（m0009 DDL）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub department_id: Option<Option<i32>>,
    /// DB 可空列 inspector_id（m0009 DDL）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub inspector_id: Option<Option<i32>>,
    /// DB 可空列 notes TEXT（m0009 DDL）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
    /// DB 可空列 attachment_urls（m0009 DDL，TEXT 序列化列表）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub attachment_urls: Option<Option<Vec<String>>>,
}

/// 创建入库明细请求
#[derive(Debug, Validate, Deserialize, Serialize)]
pub struct CreateReceiptItemRequest {
    /// 订单明细 ID
    pub order_item_id: Option<i32>,

    /// 行号
    pub line_no: i32,

    /// 物料 ID
    pub material_id: i32,

    /// 物料编码
    pub material_code: String,

    /// 物料名称
    pub material_name: String,

    /// 批次号
    pub batch_no: Option<String>,

    /// 色号
    pub color_code: Option<String>,

    /// 缸号
    pub lot_no: Option<String>,

    /// 染色匹号（匹号领域：入库使用染色匹号）
    pub piece_no: Option<String>,

    /// 等级
    pub grade: Option<String>,

    /// 克重
    pub gram_weight: Option<Decimal>,

    /// 幅宽
    pub width: Option<Decimal>,

    /// 入库数量（主单位）
    pub quantity: Decimal,

    /// 入库数量（辅助单位）
    pub quantity_alt: Decimal,

    /// 主单位
    pub unit_master: String,

    /// 辅助单位
    pub unit_alt: Option<String>,

    /// 单价
    pub unit_price: Option<Decimal>,

    /// 库位编码
    pub location_code: Option<String>,

    /// 包号
    pub package_no: Option<String>,

    /// 生产日期
    pub production_date: Option<chrono::NaiveDate>,

    /// 保质期（天）
    pub shelf_life: Option<i32>,

    /// 备注
    pub notes: Option<String>,
}

/// 更新入库明细请求
///
/// 字段语义 = 显式三态部分更新（对齐 RFC 7386 JSON Merge Patch）：
/// 键缺席=保持原值、显式 `null`=清空为 NULL（仅 DB 可空列）、有值=覆盖。
/// NOT NULL 列（purchase_receipt_item.line_no/product_id/material_code/material_name/
/// quantity，m0009 DDL）不开 null 清空，显式 null 由 service 拒绝。
#[derive(Debug, Default, Deserialize)]
pub struct UpdateReceiptItemRequest {
    /// NOT NULL 列 line_no（m0009 DDL）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub line_no: Option<Option<i32>>,
    /// NOT NULL 列 product_id（m0009 DDL）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub material_id: Option<Option<i32>>,
    /// NOT NULL 列 material_code（m0009 DDL）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub material_code: Option<Option<String>>,
    /// NOT NULL 列 material_name（m0009 DDL）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub material_name: Option<Option<String>>,
    /// DB 可空列 batch_no——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub batch_no: Option<Option<String>>,
    /// DB 可空列 color_code——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub color_code: Option<Option<String>>,
    /// DB 可空列 lot_no——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub lot_no: Option<Option<String>>,
    /// DB 可空列 grade——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub grade: Option<Option<String>>,
    /// DB 可空列 gram_weight——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub gram_weight: Option<Option<Decimal>>,
    /// DB 可空列 width——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub width: Option<Option<Decimal>>,
    /// NOT NULL 列 quantity（m0009 DDL）——显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub quantity: Option<Option<Decimal>>,
    /// DB 可空列 quantity_alt——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub quantity_alt: Option<Option<Decimal>>,
    /// DB 可空列 unit_price——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub unit_price: Option<Option<Decimal>>,
    /// DB 可空列 location_code——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub location_code: Option<Option<String>>,
    /// DB 可空列 notes——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub notes: Option<Option<String>>,
    /// 染色匹号：DB 可空列 piece_no——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub piece_no: Option<Option<String>>,
}

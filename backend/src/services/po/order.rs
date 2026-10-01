//! 采购订单核心服务 facade（po/order）
//!
//! 本文件为 facade：仅保留响应 DTO、`PurchaseOrderService` 结构体与 `new` 构造器、单元测试。
//! 业务实现（CRUD / 生命周期 / 查询导出）已拆分至 `po/order_ops/` 子模块，各子模块以独立
//! `impl PurchaseOrderService` 块形式挂载方法（Rust 允许同 crate 多文件多 impl 块）。
//! `db` 字段声明为 `pub(crate)` 供各 ops 子模块直接访问 `self.db`。
//! 拆分自原 `purchase_order_service.rs`。

use sea_orm::{DatabaseConnection, FromQueryResult};
use serde::Serialize;
use std::sync::Arc;

// =====================================================
// 响应 DTO
// =====================================================

/// 采购订单视图对象
#[derive(Debug, Clone, FromQueryResult, Serialize)]
pub struct PurchaseOrderDto {
    pub id: i32,
    pub order_no: String,
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub order_date: chrono::NaiveDate,
    pub expected_delivery_date: Option<chrono::NaiveDate>,
    pub actual_delivery_date: Option<chrono::NaiveDate>,
    pub warehouse_id: i32,
    pub warehouse_name: Option<String>,
    pub department_id: i32,
    pub department_name: Option<String>,
    pub purchaser_id: i32,
    pub currency: String,
    pub exchange_rate: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub total_amount_foreign: rust_decimal::Decimal,
    pub total_quantity: rust_decimal::Decimal,
    pub total_quantity_alt: rust_decimal::Decimal,
    #[serde(rename = "status")]
    pub order_status: String,
    pub payment_terms: Option<String>,
    pub shipping_terms: Option<String>,
    pub notes: Option<String>,
    /// 附件 URL 列表：purchase_orders.attachment_urls 真实列（TEXT[]，
    /// models/purchase_order.rs:87，DDL migration/src/domain/system/mod.rs:326）。
    /// 键名与模型列同源 snake_case；list_orders/get_order 走 Entity::find()
    /// 全列 SELECT + into_model::<Dto>() 按列名直映（本仓读取富化唯一范式，
    /// 参照 crud.rs:657 column_as+LeftJoin 链路），禁止二次查询拼 N+1。
    /// 写入侧同源：crud.rs:268（创建）/ :591-592（更新）。
    pub attachment_urls: Option<Vec<String>>,
    pub created_by: i32,
    /// 创建人姓名：created_by -> users.real_name（LEFT JOIN，可空）
    pub creator_name: Option<String>,
    /// 已入库金额：该订单下所有 purchase_receipt.total_amount 之和（关联标量子查询聚合，无入库单时为 NULL）
    pub received_amount: Option<rust_decimal::Decimal>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 采购订单明细视图对象
#[derive(Debug, Clone, FromQueryResult, Serialize)]
pub struct PurchaseOrderItemDto {
    pub id: i32,
    pub order_id: i32,
    pub line_no: i32,
    #[serde(rename = "material_id")]
    pub product_id: i32,
    #[serde(rename = "material_code")]
    pub material_code: Option<String>,
    #[serde(rename = "material_name")]
    pub material_name: Option<String>,
    #[serde(rename = "quantity_ordered")]
    pub quantity: rust_decimal::Decimal,
    /// 换算后辅助单位数量（面料行业米/公斤双计量：主单位 quantity 对应的辅单位已存列
    /// purchase_order_item.quantity_alt，由转采购/创建链路按产品 UOM 换算结果写入；
    /// list_order_items 走 `Entity::find()` 全列 SELECT，此字段按列名直映真实列，非现算/非默认。
    /// 语义：主单位为匹/码/卷/张等时本列为米数；主单位为米时本列为公斤数（克重×幅宽换算）。
    pub quantity_alt: rust_decimal::Decimal,
    /// 交货数量允收容差（百分比，可空）：NULL = 未行级指定，按「品类默认 > 全局默认」解析。
    /// purchase_order_item 真实列，经 list_order_items 全列 SELECT 按列名直映。
    pub quantity_tolerance_pct: Option<rust_decimal::Decimal>,
    pub unit_price: rust_decimal::Decimal,
    #[serde(rename = "tax_rate")]
    pub tax_percent: rust_decimal::Decimal,
    /// 折扣率（百分比）：purchase_order_item.discount_percent 真实列（NOT NULL）。
    pub discount_percent: rust_decimal::Decimal,
    pub amount: rust_decimal::Decimal,
    pub tax_amount: rust_decimal::Decimal,
    /// 折扣金额（本位币）：purchase_order_item.discount_amount 真实列（NOT NULL）。
    pub discount_amount: rust_decimal::Decimal,
    pub total_amount: rust_decimal::Decimal,
    pub received_quantity: rust_decimal::Decimal,
    pub returned_quantity: rust_decimal::Decimal,
    // 转采购翻译回填的供应商侧快照编码（采购域可见，用于实际下单/收货核对；
    // 属 purchase_order_item 真实列，经 list_order_items 全列 SELECT 自动映射）。
    // 保密：销售域响应不含这两列——此 DTO 仅由 /purchase/orders/{id}/items 返回。
    pub supplier_product_code: Option<String>,
    pub supplier_color_no: Option<String>,
    /// 色号（面料行业追溯字段，DB 列名 color_code）：purchase_order_item 真实列，可空。
    pub color_code: Option<String>,
    pub notes: Option<String>,
}

// =====================================================
// 采购订单服务
// =====================================================

/// 采购订单服务（核心）
/// 业务方法分布于 `po/order_ops/` 子模块的各 `impl` 块中：CRUD / 列表 / 详情：`order_ops::crud`；生命周期（关闭）：`order_ops::lifecycle`；明细查询 / CSV 导出：`order_ops::query`
pub struct PurchaseOrderService {
    pub(crate) db: Arc<DatabaseConnection>,
}

impl PurchaseOrderService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

// =====================================================
// 单元测试模块（模式 B：内嵌 #[cfg(test)] mod tests）
// =====================================================
// 测试策略：create_order_items / create_order_header / validate_order_request 中的
// 纯算法逻辑（金额、税额、折扣、总额、行号默认值、货币/汇率默认值、日期校验、CSV 表头）
// 通过复现其计算公式进行回归保护；依赖真实数据库 schema 的方法标注 #[ignore]。

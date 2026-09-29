//! 生产订单服务的内部类型定义（production_order_ops/types）
//!
//! 批次 488 D10-2 拆分：从原 `production_order_service.rs` L33-84 迁移。
//! - `CreateProductionOrderRequest` / `UpdateProductionOrderRequest` / `ProductionOrderQuery`：外部 handler 引用，公开导出
//! - `ProductionOutputRecord`：completion 子模块内部使用，`pub(super)` 可见性

use rust_decimal::Decimal;
use sea_orm::FromQueryResult;

/// 生产订单列表/详情出参：实体 `production_order::Model` 的对外字段 + 经 LEFT JOIN
/// 富化的 `product_name`（products.name 列别名）。
/// 严格对齐前端 `api/production.ts::ProductionOrder`：可空列用 `Option`，NOT NULL 列不用。
/// 唯一富化方式（§5 范式）：`column_as(product.name, "product_name")` + `LeftJoin` +
/// `into_model`，单次查询、无 N+1；禁止 `format!` 造假名或逐行再查。
#[derive(Debug, Clone, FromQueryResult)]
pub struct ProductionOrderDto {
    pub id: i32,
    pub order_no: String,
    pub sales_order_id: Option<i32>,
    pub product_id: i32,
    pub planned_quantity: Decimal,
    pub actual_quantity: Option<Decimal>,
    pub planned_start_date: Option<chrono::NaiveDate>,
    pub planned_end_date: Option<chrono::NaiveDate>,
    pub actual_start_date: Option<chrono::NaiveDate>,
    pub actual_end_date: Option<chrono::NaiveDate>,
    pub status: String,
    pub priority: i32,
    pub work_center_id: Option<i32>,
    pub remarks: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// 创建人（仅用于服务侧 IDOR 数据范围校验，不进对外响应体）
    pub created_by: i32,
    pub product_name: Option<String>,
}

/// 审计日志中「生产订单」这类资源的 resource_type（写入侧与读取侧必须同源）。
/// 曾用值 `auto_audit` 不带实体信息，读取侧只能按 resource_id 匹配，会把其他模块同号记录的快照一并带出。
pub const AUDIT_RESOURCE_TYPE: &str = "production_order";

/// 创建生产订单请求
#[derive(Debug, Clone)]
pub struct CreateProductionOrderRequest {
    /// 订单编号（None 时自动生成 PO-时间戳-随机数）
    pub order_no: Option<String>,
    /// 关联销售订单 ID（可选）
    pub sales_order_id: Option<i32>,
    /// 产品 ID
    pub product_id: i32,
    /// 计划数量（None 取默认 0）
    pub planned_quantity: Option<Decimal>,
    /// 计划开始日期
    pub planned_start_date: Option<chrono::NaiveDate>,
    /// 计划结束日期
    pub planned_end_date: Option<chrono::NaiveDate>,
    /// 优先级（None 取 0）
    pub priority: Option<i32>,
    /// 工作中心 ID
    pub work_center_id: Option<i32>,
    /// 备注
    pub remarks: Option<String>,
    /// 创建人 ID
    pub created_by: i32,
}

/// 更新生产订单请求
#[derive(Debug, Clone)]
pub struct UpdateProductionOrderRequest {
    /// 计划数量
    pub planned_quantity: Option<Decimal>,
    /// 计划开始日期
    pub planned_start_date: Option<chrono::NaiveDate>,
    /// 计划结束日期
    pub planned_end_date: Option<chrono::NaiveDate>,
    /// 优先级
    pub priority: Option<i32>,
    /// 工作中心 ID
    pub work_center_id: Option<i32>,
    /// 备注
    pub remarks: Option<String>,
}

/// 生产订单查询参数
#[derive(Debug, Clone)]
pub struct ProductionOrderQuery {
    /// 订单编号模糊过滤
    pub order_no: Option<String>,
    /// 状态过滤
    pub status: Option<String>,
    /// 产品 ID 过滤
    pub product_id: Option<i32>,
    /// 页码（1-based）
    pub page: u64,
    /// 每页大小
    pub page_size: u64,
}

/// increase_finished_goods_txn 内部共享的成品入库流水上下文
/// 批次 488 D10-2 拆分：从原 `production_order_service.rs` L74-84 迁移。；`pub(super)` 可见性：completion 子模块需直接构造此结构体。
pub(super) struct ProductionOutputRecord {
    /// 批号
    pub batch_no: String,
    /// 色号
    pub color_no: String,
    /// 染色批号（lot 概念，防色差混批）
    pub dye_lot_no: Option<String>,
    /// 等级
    pub grade: String,
    /// 入库增量公斤数
    pub added_kg: Decimal,
    /// 入库前米数
    pub qty_before_meters: Decimal,
    /// 入库前公斤数
    pub qty_before_kg: Decimal,
    /// 入库后米数
    pub qty_after_meters: Decimal,
    /// 入库后公斤数
    pub qty_after_kg: Decimal,
}

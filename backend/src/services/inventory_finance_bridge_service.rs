//! 库存财务桥接 Service（facade）
//!
//! 负责监听库存变动事件并自动生成相应的会计凭证。
//! 本文件作为 facade，保留 Service struct + new 构造函数 + 参数对象 DTOs。
//! 业务 impl 块迁移至 inventory_finance_bridge_ops 子模块：
//! - listener：事件监听器启动/关闭 + 事件分发处理
//! - voucher：7 类库存交易凭证生成（采购入库/销售出库/库存调整/生产入库/生产领料/采购退货/销售退货）
//!
//! db 字段为 pub(crate) 供 ops 子模块访问，外部引用路径不变。

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

/// 库存财务桥接服务
/// 负责监听库存变动事件并自动生成相应的会计凭证
pub struct InventoryFinanceBridgeService {
    pub(crate) db: Arc<DatabaseConnection>,
}

/// 凭证分录构造参数对象
///
/// 批次 334 v10 复审 P3 修复：引入参数对象消除 make_voucher_item 的 too_many_arguments 警告。
/// 聚合凭证分录所需的全部字段，避免函数签名携带 9 个参数。
/// 使用生命周期 `'_` 借用 subject_code / subject_name，避免调用方不必要的 to_string()。
pub struct VoucherItemArgs<'a> {
    /// 行号
    pub line_no: i32,
    /// 科目编码
    pub subject_code: &'a str,
    /// 科目名称
    pub subject_name: &'a str,
    /// 借方金额
    pub debit: Decimal,
    /// 贷方金额
    pub credit: Decimal,
    /// 摘要
    pub summary: Option<String>,
    /// 数量（米）
    pub quantity_meters: Option<Decimal>,
    /// 数量（公斤）
    pub quantity_kg: Option<Decimal>,
    /// 单价
    pub unit_price: Option<Decimal>,
}

/// 库存事件生成凭证参数对象
///
/// 批次 337 v10 复审 P3 修复：引入参数对象消除 5 个 create_*_voucher 私有函数的 too_many_arguments 警告。
/// 5 个函数（create_purchase_receipt_voucher / create_sales_delivery_voucher /
/// create_inventory_adjustment_voucher / create_production_receipt_voucher /
/// create_production_issue_voucher）参数完全一致，统一聚合为单一参数对象。
/// 使用生命周期 `'_` 借用 source_bill_type / source_bill_no / batch_no / color_no，
/// 避免调用方不必要的 to_string()。
pub struct VoucherCreateArgs<'a> {
    /// 产品 ID
    pub product_id: i32,
    /// 仓库 ID
    pub warehouse_id: i32,
    /// 数量（米）
    pub quantity_meters: Decimal,
    /// 数量（公斤）
    pub quantity_kg: Decimal,
    /// 来源单据类型（可选）
    pub source_bill_type: Option<&'a str>,
    /// 来源单据号（可选）
    pub source_bill_no: Option<&'a str>,
    /// 来源单据 ID（可选）
    pub source_bill_id: Option<i32>,
    /// 批次号
    pub batch_no: &'a str,
    /// 色号
    pub color_no: &'a str,
    /// 创建人 ID（可选，系统自动生成时为 None）
    pub created_by: Option<i32>,
}

/// 库存盘盈盘亏凭证构造参数对象
///
/// D08 第三梯队修复：引入参数对象消除 build_overage_voucher_request /
/// build_shortage_voucher_request 两个函数的 too_many_arguments 警告。
/// 两函数签名一致，统一聚合为单一参数对象，避免函数签名携带 9 个参数。
pub struct BridgeVoucherArgs<'a> {
    /// 来源单据类型（可选）
    pub source_bill_type: Option<&'a str>,
    /// 来源单据号（可选）
    pub source_bill_no: Option<&'a str>,
    /// 来源单据 ID（可选）
    pub source_bill_id: Option<i32>,
    /// 批次号
    pub batch_no: &'a str,
    /// 色号
    pub color_no: &'a str,
    /// 摘要
    pub summary: &'a str,
    /// 金额
    pub amount: Decimal,
    /// 数量（米）
    pub quantity_meters: Decimal,
    /// 数量（公斤）
    pub quantity_kg: Decimal,
}

impl InventoryFinanceBridgeService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}

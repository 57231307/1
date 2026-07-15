//! 委外加工订单主表模型（outsourcing_order 表）
//!
//! v14 批次 430：委托加工物资贯通
//! 依据：面料行业真实业务调研文档 §5.4 委托加工物资核算 + §5.5 委外织布场景
//!       + §5.7 损耗率标准 + §6.5 委托加工模式
//! 真实业务：委外加工订单贯穿发料→加工费→入库三步分录，记录三种凭证号
//! 损耗处理：正常损耗摊入成本，非正常损耗计入营业外支出（§5.4+§5.7）

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 委外加工订单主表模型
///
/// 真实业务要点：
/// - 三步分录：发料（借 委托加工物资 / 贷 自制半成品-胚布）
///            加工费（借 委托加工物资+应交税费-进项税额 / 贷 银行存款）
///            入库（借 库存商品-成品布 / 贷 委托加工物资）
/// - 损耗处理：正常损耗摊入成本（不单独做分录），非正常损耗计入营业外支出
/// - 行业损耗率标准（§5.7）：dyeing=0.05 / weaving=0.035 / spinning=0.055
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "outsourcing_order")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 委外订单号（唯一）
    pub order_no: String,
    /// 委外类型：dyeing(染色) / printing(印花) / weaving(织布) / finishing(后整理) / other(其他)
    pub order_type: String,
    /// 委外加工厂 ID（外键 → suppliers）
    pub supplier_id: i32,
    /// 关联生产订单 ID（外键 → production_orders，可空）
    pub production_order_id: Option<i32>,
    /// 关联缸号 ID（外键 → dye_batch，可空）
    pub dye_batch_id: Option<i32>,
    /// 色号（面料行业追溯）
    pub color_no: Option<String>,
    /// 缸号（面料行业追溯）
    pub dye_lot_no: Option<String>,
    /// 发料日期
    pub issue_date: chrono::NaiveDate,
    /// 预计收回日期
    pub expected_return_date: Option<chrono::NaiveDate>,
    /// 实际收回日期
    pub actual_return_date: Option<chrono::NaiveDate>,
    /// 发出数量
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub issue_quantity: Decimal,
    /// 发出单位：kg/m/匹
    pub issue_unit: String,
    /// 收回数量
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub return_quantity: Decimal,
    /// 损耗数量 = 发出 - 收回
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub loss_quantity: Decimal,
    /// 损耗类型：normal(正常) / abnormal(非正常) / NULL(无损耗)
    pub loss_type: Option<String>,
    /// 实际损耗率 = loss_quantity / issue_quantity
    #[sea_orm(column_type = "Decimal(Some((8, 4)))")]
    pub loss_rate: Option<Decimal>,
    /// 标准损耗率（按工序：dyeing=0.05 / weaving=0.035 / spinning=0.055）
    #[sea_orm(column_type = "Decimal(Some((8, 4)))")]
    pub standard_loss_rate: Option<Decimal>,
    /// 发出材料成本
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub material_cost: Decimal,
    /// 加工费
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub processing_fee: Decimal,
    /// 运费
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub freight_fee: Decimal,
    /// 进项税额
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub tax_amount: Decimal,
    /// 非正常损耗金额（计入营业外支出）
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub abnormal_loss_amount: Decimal,
    /// 总成本 = 材料成本 + 加工费 + 运费 - 非正常损耗金额
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub total_cost: Decimal,
    /// 单位成本 = total_cost / return_quantity
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub unit_cost: Decimal,
    /// 状态：draft/issued/processing/received/settled/closed/cancelled
    pub status: String,
    /// 发料凭证号
    pub voucher_no_issue: Option<String>,
    /// 加工费凭证号
    pub voucher_no_fee: Option<String>,
    /// 入库凭证号
    pub voucher_no_receipt: Option<String>,
    /// 备注
    pub remarks: Option<String>,

    // 软删除与审计
    pub is_deleted: bool,
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联委外加工厂
    #[sea_orm(
        belongs_to = "super::supplier::Entity",
        from = "Column::SupplierId",
        to = "super::supplier::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Supplier,
    /// 关联生产订单
    #[sea_orm(
        belongs_to = "super::production_order::Entity",
        from = "Column::ProductionOrderId",
        to = "super::production_order::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    ProductionOrder,
    /// 关联缸号
    #[sea_orm(
        belongs_to = "super::dye_batch::Entity",
        from = "Column::DyeBatchId",
        to = "super::dye_batch::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    DyeBatch,
    /// 一对多：委外发料明细
    #[sea_orm(has_many = "super::outsourcing_order_item::Entity")]
    Items,
    /// 一对多：委外收回入库单
    #[sea_orm(has_many = "super::outsourcing_receipt::Entity")]
    Receipts,
    /// 一对多：委外会计分录凭证
    #[sea_orm(has_many = "super::outsourcing_voucher::Entity")]
    Vouchers,
}

impl Related<super::supplier::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Supplier.def()
    }
}

impl Related<super::production_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductionOrder.def()
    }
}

impl Related<super::dye_batch::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DyeBatch.def()
    }
}

impl Related<super::outsourcing_order_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

impl Related<super::outsourcing_receipt::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Receipts.def()
    }
}

impl Related<super::outsourcing_voucher::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Vouchers.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

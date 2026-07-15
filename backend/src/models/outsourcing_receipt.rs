//! 委外收回入库单模型（outsourcing_receipt 表）
//!
//! v14 批次 430：委托加工物资贯通
//! 依据：面料行业真实业务调研文档 §5.4 委托加工物资核算 + §5.7 损耗率标准
//! 真实业务：委外加工完成后收回成品入库，含损耗分类与质量等级
//! 状态机：draft(草稿) → confirmed(已确认) → cancelled(已取消)

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 委外收回入库单模型
///
/// 真实业务要点：
/// - 收回成品入库，按实际收回数量结转成本
/// - 正常损耗摊入成本（不影响总成本，只影响单位成本）
/// - 非正常损耗计入营业外支出，不进成本
/// - 质量状态：pending(待检) / passed(合格) / failed(不合格)
/// - 等级：A/B/C
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "outsourcing_receipt")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 收回单号（唯一）
    pub receipt_no: String,
    /// 委外订单 ID（外键 → outsourcing_order）
    pub outsourcing_order_id: i32,
    /// 收回日期
    pub receipt_date: chrono::NaiveDate,
    /// 收回的成品 ID（外键 → products）
    pub product_id: i32,
    /// 色号
    pub color_no: Option<String>,
    /// 缸号
    pub dye_lot_no: Option<String>,
    /// 匹号
    pub batch_no: Option<String>,
    /// 入库仓库 ID（外键 → warehouses）
    pub warehouse_id: Option<i32>,
    /// 收回数量
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub return_quantity: Decimal,
    /// 损耗数量
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub loss_quantity: Decimal,
    /// 损耗类型：normal/abnormal/NULL
    pub loss_type: Option<String>,
    /// 损耗率
    #[sea_orm(column_type = "Decimal(Some((8, 4)))")]
    pub loss_rate: Option<Decimal>,
    /// 是否正常损耗
    pub is_loss_normal: bool,
    /// 单位成本 = total_cost / return_quantity
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub unit_cost: Decimal,
    /// 入库总成本
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub total_cost: Decimal,
    /// 非正常损耗金额
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub abnormal_loss_amount: Decimal,
    /// 质量状态：pending(待检) / passed(合格) / failed(不合格)
    pub quality_status: Option<String>,
    /// 等级：A/B/C
    pub grade: Option<String>,
    /// 关联库存流水 ID
    pub inventory_transaction_id: Option<i32>,
    /// 状态：draft/confirmed/cancelled
    pub status: String,
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
    /// 关联委外订单
    #[sea_orm(
        belongs_to = "super::outsourcing_order::Entity",
        from = "Column::OutsourcingOrderId",
        to = "super::outsourcing_order::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    OutsourcingOrder,
    /// 关联成品
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Product,
    /// 关联入库仓库
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::WarehouseId",
        to = "super::warehouse::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Warehouse,
}

impl Related<super::outsourcing_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OutsourcingOrder.def()
    }
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl Related<super::warehouse::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Warehouse.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

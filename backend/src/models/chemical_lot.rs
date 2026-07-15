//! 染化料批次模型（chemical_lot 表）
//!
//! v14 批次 429：染化料主数据完善
//! 依据：面料行业真实业务调研文档 §4.3 染化料管理
//! 真实业务：每批染化料的批号/供应商批号/生产日期/失效日期/来料检验状态
//! 检验状态机：pending(待检) → passed(合格) / failed(不合格) / quarantine(隔离)
//! 批次状态机：active(可用) → consumed(已耗尽) / expired(已过期) / scrapped(已报废)

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 染化料批次模型
///
/// 真实业务要点：
/// - 每批染化料独立管理批号、效期、检验状态
/// - 危化品批次应存放于 hazard 存储区
/// - 来料检验合格后方可领用
/// - 接近失效日期触发预警
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "chemical_lot")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 批次号（唯一）
    pub lot_no: String,
    /// 染化料 ID（外键 → chemical_master）
    pub chemical_id: i32,
    /// 供应商 ID（外键 → suppliers）
    pub supplier_id: Option<i32>,
    /// 供应商批号
    pub supplier_lot_no: Option<String>,
    /// 生产日期
    pub production_date: Option<chrono::NaiveDate>,
    /// 失效日期
    pub expiry_date: Option<chrono::NaiveDate>,
    /// 接收日期
    pub received_date: Option<chrono::NaiveDate>,
    /// 接收数量
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub quantity_received: Decimal,
    /// 可用数量
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub quantity_available: Decimal,
    /// 已预留数量
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub quantity_reserved: Decimal,
    /// 来料检验状态：pending(待检) / passed(合格) / failed(不合格) / quarantine(隔离)
    pub inspection_status: String,
    /// 检验报告 URL
    pub inspection_report_url: Option<String>,
    /// 单位成本
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub unit_cost: Decimal,
    /// 总成本
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub total_cost: Decimal,
    /// 仓库 ID（外键 → warehouses）
    pub warehouse_id: Option<i32>,
    /// 存储区：hazard(危险品区) / safe(普通区)
    pub storage_zone: Option<String>,
    /// 批次状态：active(可用) / consumed(已耗尽) / expired(已过期) / scrapped(已报废)
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
    /// 关联染化料主数据
    #[sea_orm(
        belongs_to = "super::chemical_master::Entity",
        from = "Column::ChemicalId",
        to = "super::chemical_master::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    Chemical,
    /// 关联供应商
    #[sea_orm(
        belongs_to = "super::supplier::Entity",
        from = "Column::SupplierId",
        to = "super::supplier::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Supplier,
    /// 关联仓库
    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::WarehouseId",
        to = "super::warehouse::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Warehouse,
}

impl Related<super::chemical_master::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Chemical.def()
    }
}

impl Related<super::supplier::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Supplier.def()
    }
}

impl Related<super::warehouse::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Warehouse.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

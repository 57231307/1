//! 染化料领用单模型（chemical_requisition 表）
//!
//! v14 批次 429：染化料主数据完善
//! 依据：面料行业真实业务调研文档 §4.3 染化料管理
//! 真实业务：生产/化验室/研发领用染化料，关联染色缸号 dye_batch_id
//! 状态机：draft(草稿) → approved(已审批) → issued(已发料) → partial_returned(部分退回) → closed(已关闭)
//! 任意非 closed 状态 → cancelled(已取消)

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 染化料领用单模型
///
/// 真实业务要点：
/// - 三种领用类型：生产领用 / 化验室领用 / 研发领用
/// - 生产领用必须关联染色缸号 dye_batch_id
/// - 审批后才能发料，发料后可部分退回
/// - 全部退回或正常结案后状态变为 closed
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "chemical_requisition")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 领用单号（唯一）
    pub requisition_no: String,
    /// 领用类型：production(生产领用) / lab(化验室领用) / rd(研发领用)
    pub requisition_type: String,
    /// 部门 ID（外键 → departments）
    pub department_id: Option<i32>,
    /// 领用日期
    pub requisition_date: chrono::NaiveDate,
    /// 需求日期
    pub required_date: Option<chrono::NaiveDate>,
    /// 关联染色缸号 ID（外键 → dye_batch，可空）
    pub dye_batch_id: Option<i32>,
    /// 关联生产订单 ID（外键 → production_orders，可空）
    pub production_order_id: Option<i32>,
    /// 状态：draft/approved/issued/partial_returned/closed/cancelled
    pub status: String,
    /// 总金额
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub total_amount: Decimal,
    /// 备注
    pub remarks: Option<String>,
    /// 软删除
    pub is_deleted: bool,
    /// 审计
    pub created_by: Option<i32>,
    pub approved_by: Option<i32>,
    pub issued_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联部门
    #[sea_orm(
        belongs_to = "super::department::Entity",
        from = "Column::DepartmentId",
        to = "super::department::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Department,
    /// 关联染色缸号
    #[sea_orm(
        belongs_to = "super::dye_batch::Entity",
        from = "Column::DyeBatchId",
        to = "super::dye_batch::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    DyeBatch,
    /// 关联生产订单
    #[sea_orm(
        belongs_to = "super::production_order::Entity",
        from = "Column::ProductionOrderId",
        to = "super::production_order::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    ProductionOrder,
}

impl Related<super::department::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Department.def()
    }
}

impl Related<super::dye_batch::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DyeBatch.def()
    }
}

impl Related<super::production_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductionOrder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

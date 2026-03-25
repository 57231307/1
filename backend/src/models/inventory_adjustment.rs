//! 库存调整模型
//! 用于记录库存调整操作（非盘点调整）

use sea_orm::entity::prelude::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 库存调整单主表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "inventory_adjustments")]
pub struct Model {
    /// 调整单 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,
    
    /// 调整单号（唯一）
    #[sea_orm(unique)]
    pub adjustment_no: String,
    
    /// 仓库 ID
    pub warehouse_id: i32,
    
    /// 调整日期
    pub adjustment_date: DateTime<Utc>,
    
    /// 调整类型：increase-增加，decrease-减少
    pub adjustment_type: String,
    
    /// 调整原因：damage-损坏，sample-样品，correction-修正，other-其他
    pub reason_type: String,
    
    /// 调整原因说明
    pub reason_description: Option<String>,
    
    /// 总数量（调整项数量总和）
    pub total_quantity: Decimal,
    
    /// 备注
    pub notes: Option<String>,
    
    /// 创建人 ID
    pub created_by: Option<i32>,
    
    /// 审核人 ID
    pub approved_by: Option<i32>,
    
    /// 审核时间
    pub approved_at: Option<DateTime<Utc>>,
    
    /// 状态：pending-待审核，approved-已审核，rejected-已驳回
    pub status: String,
    
    /// 创建时间
    pub created_at: DateTime<Utc>,
    
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 库存调整单关联表（一对多：调整单 -> 调整明细）
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::inventory_adjustment_item::Entity")]
    Item,

    #[sea_orm(
        belongs_to = "super::warehouse::Entity",
        from = "Column::WarehouseId",
        to = "super::warehouse::Column::Id"
    )]
    Warehouse,
}

impl Related<super::inventory_adjustment_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Item.def()
    }
}

impl Related<super::warehouse::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Warehouse.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

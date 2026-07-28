//! 固定资产盘点单 Model
//!
//! V15 P1 17.8-D4：资产盘点闭环（盘点计划-盘点执行-差异处理-凭证生成）

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 固定资产盘点单 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "fixed_asset_counts")]
pub struct Model {
    /// 盘点单 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 盘点单号
    #[sea_orm(unique)]
    pub count_no: String,

    /// 盘点计划名称
    pub plan_name: String,

    /// 盘点日期
    pub count_date: NaiveDate,

    /// 资产类别（筛选条件）
    pub asset_category: Option<String>,

    /// 存放地点（筛选条件）
    pub use_location: Option<String>,

    /// 盘点状态：DRAFT=草稿，COUNTING=盘点中，COMPLETED=已完成
    pub status: String,

    /// 总项数
    pub total_items: i32,

    /// 已盘点项数
    pub counted_items: i32,

    /// 盘盈项数
    pub surplus_items: i32,

    /// 盘亏项数（含毁损）
    pub shortage_items: i32,

    /// 备注
    pub notes: Option<String>,

    /// 创建人 ID
    pub created_by: i32,

    /// 审批人/完成人 ID
    pub approved_by: Option<i32>,

    /// 完成时间
    pub completed_at: Option<DateTime<Utc>>,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    Creator,
    #[sea_orm(has_many = "super::fixed_asset_count_item::Entity")]
    Items,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Creator.def()
    }
}

impl Related<super::fixed_asset_count_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

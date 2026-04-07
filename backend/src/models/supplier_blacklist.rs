#![allow(dead_code, unused_imports, unused_variables)]
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "supplier_blacklists")]
pub struct Model {
    /// 黑名单 ID
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 供应商 ID
    pub supplier_id: i32,
    /// 列入日期
    pub blacklist_date: Date,
    /// 列入原因
    pub blacklist_reason: String,
    /// 详细说明
    pub detail_description: String,
    /// 证据材料
    pub evidence: Option<String>,
    /// 审批人 ID
    pub approver_id: i32,
    /// 审批日期
    pub approval_date: Date,
    /// 是否永久
    pub is_permanent: bool,
    /// 解禁日期
    pub release_date: Option<Date>,
    /// 解禁条件
    pub release_condition: Option<String>,
    /// 解禁状态
    pub release_status: String,
    /// 实际解禁日期
    pub release_date_actual: Option<Date>,
    /// 备注
    pub remarks: Option<String>,
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
    /// 创建人 ID
    pub created_by: Option<i32>,
    /// 更新人 ID
    pub updated_by: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 黑名单 - 供应商（多对一）
    #[sea_orm(
        belongs_to = "super::supplier::Entity",
        from = "Column::SupplierId",
        to = "super::supplier::Column::Id"
    )]
    Supplier,
}

impl Related<super::supplier::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Supplier.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

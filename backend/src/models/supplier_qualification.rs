#![allow(dead_code, unused_imports, unused_variables)]
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "supplier_qualifications")]
pub struct Model {
    /// 资质 ID
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 供应商 ID
    pub supplier_id: i32,
    /// 资质名称
    pub qualification_name: String,
    /// 资质类型
    pub qualification_type: String,
    /// 资质编号
    pub qualification_no: String,
    /// 发证机构
    pub issuing_authority: String,
    /// 发证日期
    pub issue_date: Date,
    /// 有效期至
    pub valid_until: Date,
    /// 附件路径
    pub attachment_path: Option<String>,
    /// 是否年检
    pub need_annual_check: bool,
    /// 年检记录
    pub annual_check_record: Option<String>,
    /// 是否过期
    pub is_expired: bool,
    /// 备注
    pub remarks: Option<String>,
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 资质 - 供应商（多对一）
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

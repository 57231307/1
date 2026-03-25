use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "supplier_contacts")]
pub struct Model {
    /// 联系人 ID
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 供应商 ID
    pub supplier_id: i32,
    /// 联系人姓名
    pub contact_name: String,
    /// 所属部门
    pub department: Option<String>,
    /// 职位
    pub position: Option<String>,
    /// 手机号码
    pub mobile_phone: String,
    /// 联系电话
    pub tel_phone: Option<String>,
    /// 联系邮箱
    pub email: Option<String>,
    /// 微信
    pub wechat: Option<String>,
    /// QQ
    pub qq: Option<String>,
    /// 是否主要联系人
    pub is_primary: bool,
    /// 备注
    pub remarks: Option<String>,
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 联系人 - 供应商（多对一）
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

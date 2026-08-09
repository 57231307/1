use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 客户收货地址实体 - batch-13 P3: 客户多地址
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "customer_addresses")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 客户 ID
    pub customer_id: i32,
    /// 收货人
    pub contact_name: String,
    /// 联系电话
    pub contact_phone: String,
    /// 省份
    pub province: Option<String>,
    /// 城市
    pub city: Option<String>,
    /// 区县
    pub district: Option<String>,
    /// 详细地址
    pub address: String,
    /// 邮编
    pub postal_code: Option<String>,
    /// 是否默认地址
    pub is_default: bool,
    /// 备注
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// 创建客户地址 DTO
#[derive(Deserialize)]
pub struct CreateCustomerAddressDto {
    pub contact_name: String,
    pub contact_phone: String,
    pub province: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub address: String,
    pub postal_code: Option<String>,
    pub is_default: Option<bool>,
    pub remark: Option<String>,
}

/// 更新客户地址 DTO
#[derive(Deserialize)]
pub struct UpdateCustomerAddressDto {
    pub contact_name: Option<String>,
    pub contact_phone: Option<String>,
    pub province: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub address: Option<String>,
    pub postal_code: Option<String>,
    pub is_default: Option<bool>,
    pub remark: Option<String>,
}

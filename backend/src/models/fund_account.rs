#![allow(dead_code, unused_imports, unused_variables)]
//! 资金账户 Model
//!
//! 资金账户模块

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 资金账户 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "fund_accounts")]
pub struct Model {
    /// 账户 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 账户编号
    #[sea_orm(unique)]
    pub account_no: String,

    /// 账户名称
    pub account_name: String,

    /// 账户类型：CASH=现金，BANK=银行存款，OTHER=其他
    pub account_type: String,

    /// 开户银行
    pub bank_name: Option<String>,

    /// 银行账号
    pub bank_account: Option<String>,

    /// 账户余额
    pub balance: Decimal,

    /// 币种
    pub currency: String,

    /// 备注
    pub remarks: Option<String>,

    /// 是否启用
    pub is_active: bool,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 资金账户关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 账户 - 用户（创建人，多对一）
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    Creator,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Creator.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

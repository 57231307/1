//! 资金转账记录 Model
//!
//! 资金转账记录模块

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 资金转账记录 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "fund_transfer_records")]
pub struct Model {
    /// 转账记录 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 转账单号
    #[sea_orm(unique)]
    pub transfer_no: String,

    /// 转出账户 ID（外键）
    pub from_account_id: i32,

    /// 转入账户 ID（外键）
    pub to_account_id: i32,

    /// 转账日期
    pub transfer_date: NaiveDate,

    /// 转账金额
    pub amount: Decimal,

    /// 转账状态：DRAFT=草稿，CONFIRMED=已确认，COMPLETED=已完成，CANCELLED=已取消
    pub status: String,

    /// 备注
    pub remarks: Option<String>,

    /// 创建人 ID
    pub created_by: i32,

    /// 创建时间
    pub created_at: DateTime<Utc>,

    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

/// 资金转账记录关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 转账 - 转出账户（多对一）
    #[sea_orm(
        belongs_to = "super::fund_account::Entity",
        from = "Column::FromAccountId",
        to = "super::fund_account::Column::Id"
    )]
    FromAccount,

    /// 转账 - 转入账户（多对一）
    #[sea_orm(
        belongs_to = "super::fund_account::Entity",
        from = "Column::ToAccountId",
        to = "super::fund_account::Column::Id"
    )]
    ToAccount,

    /// 转账 - 用户（创建人，多对一）
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    Creator,
}

impl Related<super::fund_account::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::FromAccount.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Creator.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

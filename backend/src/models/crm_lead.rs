#![allow(dead_code, unused_imports, unused_variables)]
//! CRM 线索 Model
//!
//! CRM 线索模块

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// CRM 线索 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "crm_lead")]
pub struct Model {
    /// 线索 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 线索编号
    #[sea_orm(unique)]
    pub lead_no: String,

    /// 线索名称
    pub name: String,

    /// 客户名称
    pub customer_name: Option<String>,

    /// 联系人
    pub contact_person: Option<String>,

    /// 联系电话
    pub contact_phone: Option<String>,

    /// 邮箱
    pub email: Option<String>,

    /// 地址
    pub address: Option<String>,

    /// 来源：MARKETING=营销活动，REFERRAL=转介绍，DIRECT=直接联系，OTHER=其他
    pub source: String,

    /// 状态：NEW=新建，CONTACTED=已联系，QUALIFIED=已合格，CONVERTED=已转化，LOST=已流失
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

/// CRM 线索关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

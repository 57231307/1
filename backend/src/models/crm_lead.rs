#![allow(dead_code)]

//! CRM 线索 Model
//!
//! CRM 线索模块

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
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

    /// 线索来源
    pub lead_source: String,

    /// 线索状态：new/contacted/qualified/converted/lost
    pub lead_status: Option<String>,

    /// 公司名称
    pub company_name: Option<String>,

    /// 联系人姓名
    pub contact_name: String,

    /// 联系人职位
    pub contact_title: Option<String>,

    /// 手机号
    pub mobile_phone: Option<String>,

    /// 座机
    pub tel_phone: Option<String>,

    /// 邮箱
    pub email: Option<String>,

    /// 微信
    pub wechat: Option<String>,

    /// QQ
    pub qq: Option<String>,

    /// 地址
    pub address: Option<String>,

    /// 感兴趣的产品
    pub product_interest: Option<String>,

    /// 预估数量
    pub estimated_quantity: Option<Decimal>,

    /// 预估金额
    pub estimated_amount: Option<Decimal>,

    /// 预期交付日期
    pub expected_delivery_date: Option<NaiveDate>,

    /// 需求描述
    pub requirement_desc: Option<String>,

    /// 负责人 ID
    pub owner_id: i32,

    /// 负责人姓名
    pub owner_name: String,

    /// 最近跟进日期
    pub last_follow_up_date: Option<NaiveDate>,

    /// 下次跟进日期
    pub next_follow_up_date: Option<NaiveDate>,

    /// 跟进计划
    pub follow_up_plan: Option<String>,

    /// 转化时间
    pub converted_at: Option<DateTime<Utc>>,

    /// 转化后的客户 ID
    pub converted_customer_id: Option<i32>,

    /// 转化后的商机 ID
    pub converted_opportunity_id: Option<i32>,

    /// 流失原因
    pub lost_reason: Option<String>,

    /// 优先级：low/medium/high/urgent
    pub priority: Option<String>,

    /// 评分
    pub rating: Option<i32>,

    /// 标签
    pub tags: Option<Vec<String>>,

    /// 创建时间
    pub created_at: Option<DateTime<Utc>>,

    /// 更新时间
    pub updated_at: Option<DateTime<Utc>>,

    /// 创建人 ID
    pub created_by: Option<i32>,

    /// 更新人 ID
    pub updated_by: Option<i32>,
}

/// CRM 线索关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::ConvertedCustomerId",
        to = "super::customer::Column::Id"
    )]
    ConvertedCustomer,
}

impl ActiveModelBehavior for ActiveModel {}

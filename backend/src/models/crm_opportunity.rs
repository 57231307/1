#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! CRM 商机 Model
//!
//! CRM 商机模块

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// CRM 商机 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "crm_opportunity")]
pub struct Model {
    /// 商机 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 商机编号
    #[sea_orm(unique)]
    pub opportunity_no: String,

    /// 商机名称
    pub opportunity_name: String,

    /// 客户 ID（外键）
    pub customer_id: i32,

    /// 线索 ID（外键）
    pub lead_id: Option<i32>,

    /// 商机类型
    pub opportunity_type: Option<String>,

    /// 商机阶段
    pub opportunity_stage: Option<String>,

    /// 成交概率
    pub win_probability: Option<Decimal>,

    /// 预估金额
    pub estimated_amount: Option<Decimal>,

    /// 实际金额
    pub actual_amount: Option<Decimal>,

    /// 币种
    pub currency: Option<String>,

    /// 预计成交日期
    pub expected_close_date: Option<NaiveDate>,

    /// 实际成交日期
    pub actual_close_date: Option<NaiveDate>,

    /// 产品 IDs
    pub product_ids: Option<Vec<i32>>,

    /// 产品名称
    pub product_names: Option<Vec<String>>,

    /// 产品描述
    pub product_desc: Option<String>,

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

    /// 竞争对手
    pub competitor_names: Option<Vec<String>>,

    /// 竞争优势
    pub competitive_advantage: Option<String>,

    /// 商机状态
    pub opportunity_status: Option<String>,

    /// 成交原因
    pub won_reason: Option<String>,

    /// 流失原因
    pub lost_reason: Option<String>,

    /// 优先级
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

/// CRM 商机关联关系
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
    #[sea_orm(
        belongs_to = "super::crm_lead::Entity",
        from = "Column::LeadId",
        to = "super::crm_lead::Column::Id"
    )]
    Lead,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::OwnerId",
        to = "super::user::Column::Id"
    )]
    Owner,
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::crm_lead::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Lead.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Owner.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

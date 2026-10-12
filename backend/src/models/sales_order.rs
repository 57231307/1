#![allow(dead_code)]
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Default, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_orders")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub order_no: String,
    pub customer_id: i32,
    pub opportunity_id: Option<i32>,
    pub order_date: DateTime<Utc>,
    /// 要求交期：生效 DDL 为可空 TIMESTAMPTZ（migration system/mod.rs:420），
    /// 声明成非 Option 会让任一 NULL 行在读取时 ColumnNull 报错（列表/详情整链 500）
    pub required_date: Option<DateTime<Utc>>,
    pub ship_date: Option<DateTime<Utc>>,
    pub status: String,
    pub subtotal: Decimal,
    pub tax_amount: Decimal,
    pub discount_amount: Decimal,
    pub shipping_cost: Decimal,
    pub total_amount: Decimal,
    pub paid_amount: Decimal,
    pub balance_amount: Decimal,
    pub shipping_address: Option<String>,
    /// 收货联系人快照（下单时从客户档案带出，可编辑）
    pub contact_person: Option<String>,
    /// 收货联系电话快照
    pub contact_phone: Option<String>,
    pub billing_address: Option<String>,
    pub notes: Option<String>,
    pub batch_no: Option<String>,
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
    pub grade: Option<String>,
    pub packaging_requirement: Option<String>,
    pub quality_standard: Option<String>,
    pub created_by: Option<i32>,
    /// 数据部门 ID（RLS dept 语义，由 trg_sales_orders_dept 触发器自动维护 =
    /// created_by 指向用户的 department_id；NULL 历史数据保留 NULL）
    /// m_rls_dept_domain 迁移补列
    pub department_id: Option<i32>,
    pub approved_by: Option<i32>,
    pub approved_at: Option<DateTime<Utc>>,
    /// 审批通过理由（m0079 加列，TEXT 可空；流程放行类，选填可留空=NULL）
    pub approval_reason: Option<String>,
    /// 审批拒绝理由（m0079 加列，TEXT 可空；自本列起 reject 不再挪用 notes）
    pub rejected_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::sales_order_item::Entity")]
    Items,
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    Creator,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ApprovedBy",
        to = "super::user::Column::Id"
    )]
    Approver,
    #[sea_orm(
        belongs_to = "super::crm_opportunity::Entity",
        from = "Column::OpportunityId",
        to = "super::crm_opportunity::Column::Id"
    )]
    Opportunity,
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::sales_order_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

impl Related<super::crm_opportunity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Opportunity.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

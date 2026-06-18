#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 销售报价单主表实体
//!
//! 销售模块核心，订单前序。涵盖 Incoterms 2020 + 多币种 + 状态机 + BPM 审批 + 报价转销售订单。
//! 关联计划：[2026-06-17-p12-batch1-quotation-port-plan.md](../../../../../docs/superpowers/plans/2026-06-17-p12-batch1-quotation-port-plan.md) PR-1
//!
//! 字段类型适配 main 风格：
//! - 所有主外键 ID 均为 i32（与 main 已有 sales_order / customer / user / product 一致）
//! - status 枚举：'DRAFT' / 'SUBMITTED' / 'APPROVED' / 'REJECTED' / 'CONVERTED' / 'CANCELLED' / 'EXPIRED'

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 销售报价单主表实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_quotations")]
pub struct Model {
    /// 报价单 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 报价单号（唯一）
    pub quotation_no: String,
    /// 客户 ID（外键 customers.id）
    pub customer_id: i32,
    /// 销售员 ID（外键 users.id）
    pub sales_user_id: i32,
    /// 报价日期
    pub quotation_date: NaiveDate,
    /// 有效期至
    pub valid_until: NaiveDate,

    /// 报价货币
    pub currency: String,
    /// 汇率
    pub exchange_rate: Decimal,
    /// 本位币
    pub base_currency: String,

    /// 价格条款（Incoterms 2020：FOB/CIF/EXW/DDP/DAP）
    pub price_terms: String,
    /// Incoterms 版本
    pub incoterms_version: Option<String>,
    /// Incoterms 地点
    pub incoterm_location: Option<String>,

    /// 是否含税
    pub tax_inclusive: bool,
    /// 税率（%）
    pub tax_rate: Decimal,

    /// 最小起订量
    pub moq: Option<Decimal>,
    /// 交货周期（天）
    pub lead_time_days: Option<i32>,
    /// 客户等级
    pub customer_level: Option<String>,

    /// 不含税小计
    pub subtotal: Decimal,
    /// 税额
    pub tax_amount: Decimal,
    /// 含税总额
    pub total_amount: Decimal,

    /// 状态
    pub status: String,

    /// BPM 审批实例 ID（外键待补建）
    pub approval_instance_id: Option<i32>,
    /// 审批人（外键 users.id）
    pub approved_by: Option<i32>,
    /// 审批时间
    pub approved_at: Option<DateTime<Utc>>,
    /// 拒绝原因
    pub rejection_reason: Option<String>,

    /// 转换后的销售订单 ID（外键 sales_orders.id）
    pub converted_sales_order_id: Option<i32>,
    /// 转换时间
    pub converted_at: Option<DateTime<Utc>>,

    /// 备注
    pub notes: Option<String>,
    /// 创建人（外键 users.id）
    pub created_by: i32,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::sales_quotation_item::Entity")]
    Items,
    #[sea_orm(has_many = "super::sales_quotation_term::Entity")]
    Terms,
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::SalesUserId",
        to = "super::user::Column::Id"
    )]
    SalesUser,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::CreatedBy",
        to = "super::user::Column::Id"
    )]
    CreatedByUser,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::ApprovedBy",
        to = "super::user::Column::Id"
    )]
    ApprovedByUser,
    #[sea_orm(
        belongs_to = "super::sales_order::Entity",
        from = "Column::ConvertedSalesOrderId",
        to = "super::sales_order::Column::Id"
    )]
    ConvertedSalesOrder,
}

impl Related<super::sales_quotation_item::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Items.def()
    }
}

impl Related<super::sales_quotation_term::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Terms.def()
    }
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SalesUser.def()
    }
}

impl Related<super::sales_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ConvertedSalesOrder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

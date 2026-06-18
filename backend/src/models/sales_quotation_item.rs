#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! 销售报价单明细实体
//!
//! 行项目表：每个产品/色号对应一行报价。包含色号编码、潘通号、CNCS 号、规格、计量单位、数量、单价、金额、阶梯价 JSON、折扣等。
//! 关联计划：[2026-06-17-p12-batch1-quotation-port-plan.md](../../../../../docs/superpowers/plans/2026-06-17-p12-batch1-quotation-port-plan.md) PR-1
//!
//! 字段类型适配 main 风格：i32 与主表保持一致。

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 销售报价单明细实体
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_quotation_items")]
pub struct Model {
    /// 明细 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 报价单 ID（外键 sales_quotations.id，级联删除）
    pub quotation_id: i32,

    /// 产品 ID（外键 products.id）
    pub product_id: i32,
    /// 色号 ID（外键 product_colors.id）
    pub color_id: Option<i32>,
    /// 色号编码
    pub color_code: Option<String>,
    /// 潘通色号
    pub pantone_code: Option<String>,
    /// CNCS 色号
    pub cncs_code: Option<String>,

    /// 规格
    pub specification: Option<String>,
    /// 计量单位
    pub unit: String,

    /// 数量
    pub quantity: Decimal,
    /// 不含税单价
    pub unit_price: Decimal,
    /// 含税单价
    pub unit_price_with_tax: Decimal,
    /// 不含税金额
    pub amount: Decimal,
    /// 含税金额
    pub amount_with_tax: Decimal,

    /// 阶梯价 JSON 数据
    pub tier_pricing: Option<Json>,
    /// 折扣率（%）
    pub discount_rate: Option<Decimal>,
    /// 折扣金额
    pub discount_amount: Option<Decimal>,

    /// 备注
    pub notes: Option<String>,
    /// 排序号
    pub sequence: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::sales_quotation::Entity",
        from = "Column::QuotationId",
        to = "super::sales_quotation::Column::Id"
    )]
    Quotation,
    #[sea_orm(
        belongs_to = "super::product::Entity",
        from = "Column::ProductId",
        to = "super::product::Column::Id"
    )]
    Product,
    #[sea_orm(
        belongs_to = "super::product_color::Entity",
        from = "Column::ColorId",
        to = "super::product_color::Column::Id"
    )]
    Color,
}

impl Related<super::sales_quotation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Quotation.def()
    }
}

impl Related<super::product::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Product.def()
    }
}

impl Related<super::product_color::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Color.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

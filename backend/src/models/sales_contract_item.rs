#![allow(dead_code)]
//! 销售合同明细行 Entity
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sales_contract_items")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 合同 ID
    pub contract_id: i32,
    /// 产品 ID
    pub product_id: Option<i32>,
    /// 产品名称
    pub product_name: String,
    /// 产品规格
    pub product_spec: Option<String>,
    /// 单位
    pub unit: String,
    /// 数量
    #[sea_orm(column_type = "Decimal(Some((15, 2)))")]
    pub quantity: Decimal,
    /// 交货数量允收容差（百分比，DECIMAL(5,2)，可空）。
    /// NULL = 未行级指定，按「品类默认 > 全局默认」解析（见 utils::delivery_tolerance）。
    /// 与 dual_unit_converter 的 0.5% 录入校验无关。
    #[sea_orm(column_type = "Decimal(Some((5, 2)))")]
    pub quantity_tolerance_pct: Option<Decimal>,
    /// 单价
    #[sea_orm(column_type = "Decimal(Some((15, 4)))")]
    pub unit_price: Decimal,
    /// 金额
    #[sea_orm(column_type = "Decimal(Some((15, 2)))")]
    pub amount: Decimal,
    /// 交货日期
    pub delivery_date: Option<NaiveDate>,
    /// 色号（m_sales_contract_source_order 迁移加列，VARCHAR 255 可空）。
    /// NULL 表示手工录入行未采集，空串表示白坯布，非空表示染色布，
    /// 语义与 sales_order_items 同名列一致。写入方是 sales_contract_service 转单路径
    pub color_no: Option<String>,
    /// 折扣率百分比（m_sales_contract_source_order 迁移加列，DECIMAL 18 位小数 4 位，可空）。
    /// NULL 表示未采集（手工录入行），非空由转单按来源订单行原值带入。
    /// 列宽与 sales_order_items 同名列一致，带入不截断
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub discount_percent: Option<Decimal>,
    /// 折扣金额（m_sales_contract_source_order 迁移加列，DECIMAL 18 位小数 4 位，可空）。
    /// 由服务端按数量、单价与折扣率算出，不采信客户端送来的值
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub discount_amount: Option<Decimal>,
    /// 税率百分比（m_sales_contract_source_order 迁移加列，DECIMAL 18 位小数 4 位，可空）。
    /// NULL 表示未采集（手工录入行），非空由转单按来源订单行原值带入，
    /// 入参边界校验 0 到 100（与订单侧同一口径）
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub tax_percent: Option<Decimal>,
    /// 税额（m_sales_contract_source_order 迁移加列，DECIMAL 18 位小数 4 位，可空）。
    /// 由服务端按折扣后金额乘税率算出，与订单行共用同一金额算法模块
    #[sea_orm(column_type = "Decimal(Some((18, 4)))")]
    pub tax_amount: Option<Decimal>,
    /// 来源销售订单明细行 ID（m_sales_contract_source_order 迁移加列，INTEGER 可空，
    /// 外键指向 sales_order_items 主键并在订单行删除时置空）。NULL 表示手工录入的合同行，
    /// 非空表示由该订单行带入的合同行。转单时服务端按本行回读订单行的折扣率与税率，
    /// 客户端不申报税率，金额算法因此只有一个来源
    pub source_order_item_id: Option<i32>,
    /// 备注
    pub remarks: Option<String>,
    /// 排序
    pub sort_order: i32,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 明细行 - 合同（多对一）
    #[sea_orm(
        belongs_to = "super::sales_contract::Entity",
        from = "Column::ContractId",
        to = "super::sales_contract::Column::Id"
    )]
    SalesContract,
}

impl Related<super::sales_contract::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SalesContract.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

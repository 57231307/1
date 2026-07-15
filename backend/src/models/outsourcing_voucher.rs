//! 委外加工会计分录凭证模型（outsourcing_voucher 表）
//!
//! v14 批次 430：委托加工物资贯通
//! 依据：面料行业真实业务调研文档 §5.4 委托加工物资核算三步分录
//! 真实业务：
//! - issue(发料)：借 委托加工物资 / 贷 自制半成品-胚布
//! - fee(加工费)：借 委托加工物资 + 应交税费-进项税额 / 贷 银行存款
//! - receipt(入库)：借 库存商品-成品布 / 贷 委托加工物资
//! - loss(损耗处理)：借 营业外支出 / 贷 委托加工物资（非正常损耗单独追责）

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 委外加工会计分录凭证模型
///
/// 真实业务要点：
/// - 凭证类型：issue(发料) / fee(加工费) / receipt(入库) / loss(损耗处理)
/// - 借方科目：委托加工物资 / 库存商品-成品布 / 应交税费-进项税额 / 营业外支出
/// - 贷方科目：自制半成品-胚布 / 库存商品-棉纱 / 委托加工物资 / 银行存款
/// - 仅加工费凭证有税额（tax_amount）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "outsourcing_voucher")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 凭证号（唯一）
    pub voucher_no: String,
    /// 委外订单 ID（外键 → outsourcing_order）
    pub outsourcing_order_id: i32,
    /// 凭证类型：issue(发料) / fee(加工费) / receipt(入库) / loss(损耗处理)
    pub voucher_type: String,
    /// 借方科目（如 委托加工物资 / 库存商品-成品布 / 应交税费-进项税额 / 营业外支出）
    pub debit_account: String,
    /// 贷方科目（如 自制半成品-胚布 / 库存商品-棉纱 / 委托加工物资 / 银行存款）
    pub credit_account: String,
    /// 金额
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub amount: Decimal,
    /// 税额（仅加工费凭证有）
    #[sea_orm(column_type = "Decimal(Some((14, 4)))")]
    pub tax_amount: Decimal,
    /// 凭证日期
    pub voucher_date: chrono::NaiveDate,
    /// 是否已过账
    pub is_posted: bool,
    /// 过账时间
    pub posted_at: Option<DateTimeWithTimeZone>,
    /// 备注
    pub remarks: Option<String>,

    // 审计
    pub created_by: Option<i32>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 关联委外订单
    #[sea_orm(
        belongs_to = "super::outsourcing_order::Entity",
        from = "Column::OutsourcingOrderId",
        to = "super::outsourcing_order::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    OutsourcingOrder,
}

impl Related<super::outsourcing_order::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::OutsourcingOrder.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

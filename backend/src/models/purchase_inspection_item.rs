//! 采购质检明细 Model
//!
//! 批次 131 v9 复审 P0 修复：替代 purchase_inspection_handler 4 个明细 CRUD 端点占位。
//! 原 list_inspection_items 返回硬编码空列表，create/update/delete 仅记日志不落库。
//! 现使用真实 purchase_inspection_items 表存储明细。

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 采购质检明细 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "purchase_inspection_items")]
pub struct Model {
    /// 明细 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 采购质检单 ID（外键 purchase_inspection.id）
    pub inspection_id: i32,
    /// 产品 ID（外键 products.id）
    pub product_id: i32,
    /// 检验项目名称
    pub item_name: String,
    /// 合格数量
    pub qualified_quantity: Decimal,
    /// 不合格数量
    pub unqualified_quantity: Decimal,
    /// 备注
    pub remark: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

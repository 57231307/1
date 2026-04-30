#![allow(dead_code, unused_imports, unused_variables)]
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "supplier_grades")]
pub struct Model {
    /// 等级 ID
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 等级编码（A/B/C/D）
    pub grade_code: String,
    /// 等级名称
    pub grade_name: String,
    /// 最低分数
    pub min_score: Decimal,
    /// 最高分数
    pub max_score: Decimal,
    /// 颜色标识
    pub color_code: Option<String>,
    /// 权限说明
    pub permission_desc: Option<String>,
    /// 是否启用
    pub is_enabled: bool,
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

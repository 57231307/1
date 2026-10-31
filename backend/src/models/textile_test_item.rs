#![allow(dead_code)]
//! 纺织品安全检测项目主数据模型（GB 18401 基本项目与 GB 31701 加考条款）
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "textile_test_item")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 项目稳定编码（词表与种子查找键，禁改值）
    pub item_code: String,
    pub item_name: String,
    /// 测量类型，取值域 `models::status::textile_item_kind`
    pub item_kind: String,
    /// 项目出身标准分组，取值域 `models::status::textile_basis_standard`
    pub basis_standard: String,
    /// 核查方式（measurable 或 manual），取值域 `models::status::textile_check_kind`
    pub check_kind: String,
    /// 检测方法标准号，来源不确定为 NULL
    pub method_standard: Option<String>,
    /// 计量单位（mg/kg、%、mm、级），等级与感官型可空
    pub unit: Option<String>,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}

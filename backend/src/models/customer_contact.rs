#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// 例外说明：本文件为批次 90b P2-12 新增模型，需待 v3 复审 v4 验证后接入业务方逐步移除本标注。

//! 客户联系人 Model
//!
//! 客户联系人模块，记录客户的多个联系人信息（含主联系人标识）。
//! 批次 90b P2-12：替代前端 crm/detail.vue "新增联系人功能待实现" 占位符。

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 客户联系人 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "customer_contacts")]
pub struct Model {
    /// 联系人 ID
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 客户 ID
    pub customer_id: i32,
    /// 联系人姓名
    pub name: String,
    /// 职务
    pub title: Option<String>,
    /// 联系电话
    pub phone: String,
    /// 联系邮箱
    pub email: Option<String>,
    /// 是否主要联系人
    pub is_primary: bool,
    /// 备注
    pub remarks: Option<String>,
    /// 创建人
    pub created_by: Option<i32>,
    /// 创建时间
    pub created_at: DateTimeWithTimeZone,
    /// 更新时间
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    /// 联系人 - 客户（多对一）
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

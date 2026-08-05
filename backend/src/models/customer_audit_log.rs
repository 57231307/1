use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// V15 P2 18.4-D6: 客户操作日志 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "customer_audit_log")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 客户ID
    pub customer_id: i32,

    /// 操作类型：create/update/delete/view/export
    pub operation: String,

    /// 变更字段
    pub field_name: Option<String>,

    /// 旧值
    pub old_value: Option<String>,

    /// 新值
    pub new_value: Option<String>,

    /// 操作人ID
    pub user_id: i32,

    /// 操作人姓名
    pub user_name: String,

    /// IP地址
    pub ip_address: Option<String>,

    /// 用户代理
    pub user_agent: Option<String>,

    /// 创建时间
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

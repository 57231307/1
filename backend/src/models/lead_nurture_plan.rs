use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// V15 P2 18.1-D6: 线索培育计划 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "lead_nurture_plan")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    /// 线索ID
    pub lead_id: i32,

    /// 培育计划名称
    pub plan_name: String,

    /// 培育类型：email/sms/visit/call
    pub nurture_type: String,

    /// 触发条件
    pub trigger_condition: Option<String>,

    /// 模板ID
    pub template_id: Option<String>,

    /// 计划执行时间
    pub scheduled_at: Option<DateTime<Utc>>,

    /// 实际执行时间
    pub executed_at: Option<DateTime<Utc>>,

    /// 状态：pending/executed/failed/cancelled
    pub status: Option<String>,

    /// 执行结果
    pub result: Option<String>,

    /// 创建人
    pub created_by: Option<i32>,

    /// 创建时间
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

#![allow(dead_code)]
// v11 批次 143 P1-2：user_behavior 模型用于用户行为追踪分析
// 保留文件级 dead_code 抑制以符合 models/ 目录例外规范（SeaORM 派生宏字段）

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 用户行为记录 Model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "user_behaviors")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 会话 ID（匿名用户标识）
    pub session_id: Option<String>,

    /// 用户 ID（登录用户）
    pub user_id: Option<i32>,

    /// 事件类型（click/scroll/submit 等）
    pub event_type: String,

    /// 事件目标（元素 ID/类名等）
    pub event_target: Option<String>,

    /// 事件附加数据（JSONB）
    #[sea_orm(column_type = "JsonBinary")]
    pub event_data: Option<serde_json::Value>,

    /// 页面路径
    pub path: Option<String>,

    /// IP 地址
    pub ip_address: Option<String>,

    /// 发生时间
    pub occurred_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

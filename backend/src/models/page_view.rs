#![allow(dead_code)]
// v11 批次 143 P1-2：page_view 模型用于用户行为追踪分析
// 保留文件级 dead_code 抑制以符合 models/ 目录例外规范（SeaORM 派生宏字段）

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 页面访问记录 Model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "page_views")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,

    /// 会话 ID（匿名用户标识）
    pub session_id: Option<String>,

    /// 用户 ID（登录用户）
    pub user_id: Option<i32>,

    /// 页面路径
    pub path: String,

    /// 来源页面
    pub referrer: Option<String>,

    /// 用户代理
    pub user_agent: Option<String>,

    /// IP 地址
    pub ip_address: Option<String>,

    /// 访问时间
    pub viewed_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

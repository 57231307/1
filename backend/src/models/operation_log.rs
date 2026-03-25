//! 操作日志模型
//! 用于记录用户的关键操作，便于审计追踪

use sea_orm::entity::prelude::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 操作日志表
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "operation_logs")]
pub struct Model {
    /// 日志 ID（主键）
    #[sea_orm(primary_key)]
    pub id: i32,
    
    /// 用户 ID
    pub user_id: Option<i32>,
    
    /// 用户名
    pub username: Option<String>,
    
    /// 操作模块：user-用户，role-角色，product-产品，inventory-库存，sales-销售，finance-财务
    pub module: String,
    
    /// 操作类型：create-创建，update-更新，delete-删除，approve-审核，reject-驳回，other-其他
    pub action: String,
    
    /// 操作描述
    pub description: Option<String>,
    
    /// 请求方法：GET, POST, PUT, DELETE
    pub request_method: Option<String>,
    
    /// 请求 URI
    pub request_uri: Option<String>,
    
    /// 请求 IP 地址
    pub request_ip: Option<String>,
    
    /// 用户代理（浏览器信息）
    pub user_agent: Option<String>,
    
    /// 操作状态：success-成功，failure-失败
    pub status: String,
    
    /// 错误信息（如果失败）
    pub error_message: Option<String>,
    
    /// 操作耗时（毫秒）
    pub duration_ms: Option<i64>,
    
    /// 额外数据（JSON 格式）
    pub extra_data: Option<Json>,
    
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

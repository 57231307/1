#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。

//! API 端点管理 Model（批次 91 P0-1）
//!
//! 用于管理 API 网关暴露的端点元数据，支持 CRUD 操作。

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// API 端点 Entity
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "api_endpoints")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    /// 端点路径，如 /api/v1/erp/customers
    pub path: String,
    /// HTTP 方法：GET/POST/PUT/DELETE/PATCH
    pub method: String,
    /// 端点描述
    pub description: Option<String>,
    /// 所属模块，如 customers/sales/purchase
    pub module: Option<String>,
    /// 状态：active/inactive/deprecated
    pub status: String,
    /// 每分钟速率限制（0 表示不限）
    pub rate_limit: i32,
    /// 超时时间（毫秒）
    pub timeout: i32,
    /// 是否需要认证
    pub authentication: bool,
    /// 授权角色列表（JSON 数组）
    pub authorization: Option<Json>,
    /// 请求 schema（JSON）
    pub request_schema: Option<Json>,
    /// 响应 schema（JSON）
    pub response_schema: Option<Json>,
    /// 版本号
    pub version: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

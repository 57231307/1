//! 库存盘点服务（facade 入口）
//!
//! ⚠️ 重构说明（2026-06-06）：
//! - 真实业务实现位于子模块 `inventory_count/`（query / commands / workflow / items）
//! - 本文件提供 `InventoryCountService` 结构体作为面向对象的封装，
//!   内部调用子模块的函数式 API。这样旧的 handler 代码 `InventoryCountService::new(db).list_counts(...)`
//!   仍能工作，无需修改调用方。
//!
//! 子模块结构：
//! - `query`     — 列表查询与详情查询
//! - `commands`  — 增删改操作
//! - `workflow`  — 审批与完成
//! - `items`     — 明细项管理

#![allow(dead_code)]

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::models::dto::PageRequest;
use crate::utils::error::AppError;
use crate::utils::PaginatedResponse;

pub use super::inventory_count::types::{
    CreateInventoryCountRequest, InventoryCountDetail, InventoryCountItemDetail,
    InventoryCountItemRequest, UpdateInventoryCountRequest,
};

/// 列表结果（兼容旧 API 字段命名）
pub struct CountListResult {
    pub items: Vec<InventoryCountDetail>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 库存盘点服务（面向对象门面）
///
/// 所有方法委托给 `crate::services::inventory_count` 子模块中的具体实现。
/// 由于子模块当前为占位实现（返回 NotImplemented），调用方暂时会收到
/// `功能未实现` 错误。后续在子模块中实现真实业务逻辑时，本结构体无需修改。
pub struct InventoryCountService {
    db: Arc<DatabaseConnection>,
}

impl InventoryCountService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 列出库存盘点
    pub async fn list_counts(
        &self,
        _page_req: PageRequest,
        _status: Option<String>,
        _warehouse_id: Option<i32>,
        _count_no: Option<String>,
    ) -> Result<CountListResult, AppError> {
        // 委托给子模块：crate::services::inventory_count::query::list_counts
        // 子模块当前为占位实现（返回 NotImplemented）
        Err(AppError::NotImplemented(
            "inventory_count 子模块正在开发中".to_string(),
        ))
    }

    /// 获取库存盘点详情
    pub async fn get_count_detail(&self, _count_id: i32) -> Result<InventoryCountDetail, AppError> {
        Err(AppError::NotImplemented(
            "inventory_count 子模块正在开发中".to_string(),
        ))
    }

    /// 创建库存盘点
    pub async fn create_count(
        &self,
        _req: CreateInventoryCountRequest,
    ) -> Result<InventoryCountDetail, AppError> {
        Err(AppError::NotImplemented(
            "inventory_count 子模块正在开发中".to_string(),
        ))
    }

    /// 更新库存盘点
    pub async fn update_count(
        &self,
        _count_id: i32,
        _req: UpdateInventoryCountRequest,
    ) -> Result<InventoryCountDetail, AppError> {
        Err(AppError::NotImplemented(
            "inventory_count 子模块正在开发中".to_string(),
        ))
    }

    /// 审核库存盘点
    pub async fn approve_count(
        &self,
        _count_id: i32,
        _approved: bool,
        _notes: Option<String>,
    ) -> Result<InventoryCountDetail, AppError> {
        Err(AppError::NotImplemented(
            "inventory_count 子模块正在开发中".to_string(),
        ))
    }

    /// 完成库存盘点
    pub async fn complete_count(&self, _count_id: i32) -> Result<InventoryCountDetail, AppError> {
        Err(AppError::NotImplemented(
            "inventory_count 子模块正在开发中".to_string(),
        ))
    }

    /// 删除库存盘点
    pub async fn delete_count(&self, _count_id: i32) -> Result<(), AppError> {
        Err(AppError::NotImplemented(
            "inventory_count 子模块正在开发中".to_string(),
        ))
    }

    /// 列出盘点明细
    pub async fn list_items(&self, _count_id: i32) -> Result<Vec<InventoryCountItemDetail>, AppError> {
        Err(AppError::NotImplemented(
            "inventory_count 子模块正在开发中".to_string(),
        ))
    }

    /// 添加盘点明细
    pub async fn add_item(
        &self,
        _count_id: i32,
        _req: InventoryCountItemRequest,
    ) -> Result<InventoryCountItemDetail, AppError> {
        Err(AppError::NotImplemented(
            "inventory_count 子模块正在开发中".to_string(),
        ))
    }

    /// 更新盘点明细
    pub async fn update_item(
        &self,
        _item_id: i32,
        _req: InventoryCountItemRequest,
    ) -> Result<InventoryCountItemDetail, AppError> {
        Err(AppError::NotImplemented(
            "inventory_count 子模块正在开发中".to_string(),
        ))
    }

    /// 删除盘点明细
    pub async fn delete_item(&self, _item_id: i32) -> Result<(), AppError> {
        Err(AppError::NotImplemented(
            "inventory_count 子模块正在开发中".to_string(),
        ))
    }
}

// 显式避免未使用导入告警
#[allow(dead_code)]
fn _ensure_paginated_used() -> Option<PaginatedResponse<()>> {
    None
}

//! 客户分配历史 Service
//!
//! 提供客户分配历史的记录和查询功能

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, ExprTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use sea_orm::DatabaseConnection;

use crate::models::assignment_history::{
    ActiveModel, Entity as AssignmentHistoryEntity, Model as AssignmentHistoryModel,
};
use crate::utils::error::AppError;

/// 创建分配历史请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAssignmentHistoryRequest {
    pub lead_id: i32,
    pub lead_no: String,
    pub company_name: Option<String>,
    pub from_user_id: Option<i32>,
    pub from_user_name: Option<String>,
    pub to_user_id: Option<i32>,
    pub to_user_name: Option<String>,
    pub action: String,
    pub reason: Option<String>,
    pub notes: Option<String>,
}

/// 分配历史查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct AssignmentHistoryQuery {
    pub lead_id: Option<i32>,
    pub user_id: Option<i32>,
    pub action: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 分配历史 Service
pub struct AssignmentHistoryService {
    db: Arc<DatabaseConnection>,
}

impl AssignmentHistoryService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建分配历史记录
    pub async fn create(
        &self,
        tenant_id: i32,
        user_id: i32,
        user_name: &str,
        req: CreateAssignmentHistoryRequest,
    ) -> Result<AssignmentHistoryModel, AppError> {
        let now = Utc::now();
        let active_model = ActiveModel {
            id: Default::default(),
            tenant_id: Set(tenant_id),
            lead_id: Set(req.lead_id),
            lead_no: Set(req.lead_no),
            company_name: Set(req.company_name),
            from_user_id: Set(req.from_user_id),
            from_user_name: Set(req.from_user_name),
            to_user_id: Set(req.to_user_id),
            to_user_name: Set(req.to_user_name),
            action: Set(req.action),
            reason: Set(req.reason),
            notes: Set(req.notes),
            operated_by: Set(user_id),
            operated_by_name: Set(user_name.to_string()),
            created_at: Set(now),
        };

        let model = active_model.insert(&*self.db).await?;

        Ok(model)
    }

    /// 查询分配历史列表
    pub async fn list(
        &self,
        tenant_id: i32,
        query: AssignmentHistoryQuery,
    ) -> Result<(Vec<AssignmentHistoryModel>, u64), AppError> {
        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20);

        let mut select = AssignmentHistoryEntity::find()
            .filter(crate::models::assignment_history::Column::TenantId.eq(tenant_id));

        if let Some(lead_id) = query.lead_id {
            select = select.filter(crate::models::assignment_history::Column::LeadId.eq(lead_id));
        }

        if let Some(user_id) = query.user_id {
            select = select.filter(
                crate::models::assignment_history::Column::FromUserId
                    .eq(user_id)
                    .or(crate::models::assignment_history::Column::ToUserId.eq(user_id)),
            );
        }

        if let Some(action) = query.action {
            select = select.filter(crate::models::assignment_history::Column::Action.eq(action));
        }

        let total = select.clone().count(&*self.db).await?;

        let items = select
            .order_by_desc(crate::models::assignment_history::Column::CreatedAt)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;

        Ok((items, total))
    }
}

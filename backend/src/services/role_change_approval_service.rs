//! 角色变更审批 Service
//!
//! B12-P2-4：敏感角色变更双人审批

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::models::role_change_approval::{
    self, ActiveModel as ApprovalActiveModel, ApprovalStatus, ChangeType, Entity as ApprovalEntity,
    Model as ApprovalModel,
};
use crate::utils::error::AppError;

/// 敏感角色列表
const SENSITIVE_ROLES: &[&str] = &["admin", "super_admin", "finance", "finance_admin"];

/// 创建审批请求
#[derive(Debug, Deserialize)]
pub struct CreateRoleChangeApprovalRequest {
    pub change_type: String,
    pub target_user_id: Option<i32>,
    pub target_role_id: i32,
    pub target_role_code: String,
    pub proposed_permission_id: Option<i32>,
    pub proposed_resource_type: Option<String>,
    pub proposed_action: Option<String>,
    pub proposed_allowed: Option<bool>,
}

/// 审批操作
#[derive(Debug, Deserialize)]
pub struct ApproveRoleChangeRequest {
    pub comments: Option<String>,
}

/// 审批列表查询
#[derive(Debug, Deserialize)]
pub struct RoleChangeApprovalQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub change_type: Option<String>,
    pub applicant_id: Option<i32>,
}

/// 审批列表 VO
#[derive(Debug, Serialize)]
pub struct ApprovalListVo {
    pub items: Vec<ApprovalModel>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 角色变更审批 Service
pub struct RoleChangeApprovalService {
    db: Arc<DatabaseConnection>,
}

impl RoleChangeApprovalService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 判断是否为敏感角色
    pub fn is_sensitive_role(role_code: &str) -> bool {
        SENSITIVE_ROLES.contains(&role_code)
    }

    /// 创建审批请求
    pub async fn create_request(
        &self,
        applicant_id: i32,
        applicant_username: String,
        req: CreateRoleChangeApprovalRequest,
    ) -> Result<ApprovalModel, AppError> {
        // 验证变更类型
        let change_type = match req.change_type.as_str() {
            "assign_role" => ChangeType::AssignRole,
            "assign_permission" => ChangeType::AssignPermission,
            "remove_permission" => ChangeType::RemovePermission,
            _ => return Err(AppError::validation("无效的变更类型")),
        };

        // 生成审批单号
        let approval_no = format!(
            "RCA-{}-{:04}",
            Utc::now().format("%Y%m%d%H%M%S"),
            chrono::Utc::now().timestamp_subsec_millis() % 10000
        );

        let now = Utc::now();
        let active = ApprovalActiveModel {
            id: Default::default(),
            approval_no: Set(approval_no),
            change_type: Set(change_type.to_string()),
            target_user_id: Set(req.target_user_id),
            target_role_id: Set(req.target_role_id),
            target_role_code: Set(req.target_role_code),
            proposed_permission_id: Set(req.proposed_permission_id),
            proposed_resource_type: Set(req.proposed_resource_type),
            proposed_action: Set(req.proposed_action),
            proposed_allowed: Set(req.proposed_allowed),
            applicant_id: Set(applicant_id),
            applicant_username: Set(applicant_username),
            approver1_id: Set(None),
            approver1_comment: Set(None),
            approver1_at: Set(None),
            approver2_id: Set(None),
            approver2_comment: Set(None),
            approver2_at: Set(None),
            status: Set(ApprovalStatus::PendingL1.to_string()),
            current_level: Set(1),
            completed_at: Set(None),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
        };

        let model = active.insert(&*self.db).await?;
        Ok(model)
    }

    /// 一级审批
    pub async fn approve_l1(
        &self,
        approval_id: i32,
        approver_id: i32,
        req: ApproveRoleChangeRequest,
    ) -> Result<ApprovalModel, AppError> {
        let model = ApprovalEntity::find_by_id(approval_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("审批请求不存在"))?;

        // 验证状态
        if model.status != ApprovalStatus::PendingL1.to_string() {
            return Err(AppError::business("当前状态不允许一级审批"));
        }

        // 防自审批
        if model.applicant_id == approver_id {
            return Err(AppError::business("审批人不能是申请人"));
        }

        let now = Utc::now();
        let mut active: ApprovalActiveModel = model.into();
        active.approver1_id = Set(Some(approver_id));
        active.approver1_comment = Set(req.comments);
        active.approver1_at = Set(Some(now.into()));
        active.status = Set(ApprovalStatus::PendingL2.to_string());
        active.current_level = Set(2);
        active.updated_at = Set(now.into());

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 二级审批
    pub async fn approve_l2(
        &self,
        approval_id: i32,
        approver_id: i32,
        req: ApproveRoleChangeRequest,
    ) -> Result<ApprovalModel, AppError> {
        let model = ApprovalEntity::find_by_id(approval_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("审批请求不存在"))?;

        // 验证状态
        if model.status != ApprovalStatus::PendingL2.to_string() {
            return Err(AppError::business("当前状态不允许二级审批"));
        }

        // 防自审批
        if model.applicant_id == approver_id {
            return Err(AppError::business("审批人不能是申请人"));
        }

        // 双人约束：二级审批人不能是一级审批人
        if model.approver1_id == Some(approver_id) {
            return Err(AppError::business("二级审批人不能与一级审批人相同"));
        }

        let now = Utc::now();
        let mut active: ApprovalActiveModel = model.into();
        active.approver2_id = Set(Some(approver_id));
        active.approver2_comment = Set(req.comments);
        active.approver2_at = Set(Some(now.into()));
        active.status = Set(ApprovalStatus::Approved.to_string());
        active.completed_at = Set(Some(now.into()));
        active.updated_at = Set(now.into());

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 拒绝审批
    pub async fn reject(
        &self,
        approval_id: i32,
        approver_id: i32,
        req: ApproveRoleChangeRequest,
    ) -> Result<ApprovalModel, AppError> {
        let model = ApprovalEntity::find_by_id(approval_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("审批请求不存在"))?;

        // 验证状态
        if model.status != ApprovalStatus::PendingL1.to_string()
            && model.status != ApprovalStatus::PendingL2.to_string()
        {
            return Err(AppError::business("当前状态不允许拒绝"));
        }

        let now = Utc::now();
        let mut active: ApprovalActiveModel = model.into();

        // 根据当前层级设置审批人
        if active.current_level.clone().unwrap() == 1 {
            active.approver1_id = Set(Some(approver_id));
            active.approver1_comment = Set(req.comments);
            active.approver1_at = Set(Some(now.into()));
        } else {
            active.approver2_id = Set(Some(approver_id));
            active.approver2_comment = Set(req.comments);
            active.approver2_at = Set(Some(now.into()));
        }

        active.status = Set(ApprovalStatus::Rejected.to_string());
        active.completed_at = Set(Some(now.into()));
        active.updated_at = Set(now.into());

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 取消审批（仅申请人可取消）
    pub async fn cancel(
        &self,
        approval_id: i32,
        applicant_id: i32,
    ) -> Result<ApprovalModel, AppError> {
        let model = ApprovalEntity::find_by_id(approval_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("审批请求不存在"))?;

        // 验证申请人
        if model.applicant_id != applicant_id {
            return Err(AppError::business("只有申请人可以取消审批"));
        }

        // 验证状态
        if model.status == ApprovalStatus::Approved.to_string()
            || model.status == ApprovalStatus::Rejected.to_string()
        {
            return Err(AppError::business("已完成的审批不能取消"));
        }

        let now = Utc::now();
        let mut active: ApprovalActiveModel = model.into();
        active.status = Set(ApprovalStatus::Cancelled.to_string());
        active.completed_at = Set(Some(now.into()));
        active.updated_at = Set(now.into());

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 查询审批详情
    pub async fn get_by_id(&self, id: i32) -> Result<ApprovalModel, AppError> {
        ApprovalEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("审批请求不存在"))
    }

    /// 查询审批列表
    pub async fn list(
        &self,
        query: RoleChangeApprovalQuery,
    ) -> Result<ApprovalListVo, AppError> {
        let page = query.page.unwrap_or(1).clamp(1, 1000);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let mut finder = ApprovalEntity::find();

        if let Some(status) = &query.status {
            finder = finder.filter(role_change_approval::Column::Status.eq(status.as_str()));
        }
        if let Some(change_type) = &query.change_type {
            finder = finder.filter(role_change_approval::Column::ChangeType.eq(change_type.as_str()));
        }
        if let Some(applicant_id) = query.applicant_id {
            finder = finder.filter(role_change_approval::Column::ApplicantId.eq(applicant_id));
        }

        let paginator = finder
            .order_by_desc(role_change_approval::Column::CreatedAt)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page - 1).await?;

        Ok(ApprovalListVo {
            items,
            total,
            page,
            page_size,
        })
    }
}

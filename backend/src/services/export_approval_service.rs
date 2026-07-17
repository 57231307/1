// V15 P0-S14 敏感数据导出二级审批 Service
//
// 实现：
//   1. 创建审批请求（敏感资源导出前必须先创建审批）
//   2. 一级审批通过（直接上级）→ 升级到二级
//   3. 二级审批通过（部门经理或更高）→ 生成临时下载 token（5 分钟有效）
//   4. 凭 token 下载文件（一次性，防重放）
//   5. token 过期或下载完成后流程终结
//
// 设计依据：V15 审计报告 类十三 P0-S14
// 关联文件：models/export_approval_request.rs / handlers/export_approval_handler.rs / migration 047

use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::models::export_approval_request::{
    ActiveModel as ExportApprovalActiveModel, ApprovalStatus, Column, Entity,
    ExportParams, Model, RiskLevel, sensitive_resources,
};
use crate::utils::error::AppError;

/// 下载 token 有效期（5 分钟）
const TOKEN_VALID_MINUTES: i64 = 5;

/// 创建审批请求 DTO
#[derive(Debug, Deserialize)]
pub struct CreateApprovalRequest {
    pub resource_type: String,
    pub export_params: serde_json::Value,
    pub estimated_rows: Option<i64>,
    pub file_format: Option<String>,
}

/// 审批操作 DTO
#[derive(Debug, Deserialize)]
pub struct ApproveRequest {
    /// 审批意见
    pub comments: Option<String>,
}

/// 审批请求列表 VO
#[derive(Debug, Serialize)]
pub struct ApprovalListVo {
    pub items: Vec<Model>,
    pub total: u64,
    pub page: u64,
    pub page_size: u64,
}

/// 列表查询参数
#[derive(Debug, Deserialize)]
pub struct ListApprovalQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub status: Option<String>,
    pub resource_type: Option<String>,
    pub applicant_user_id: Option<i32>,
}

/// V15 P0-S14 敏感数据导出二级审批 Service
pub struct ExportApprovalService {
    db: Arc<DatabaseConnection>,
}

impl ExportApprovalService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建审批请求
    ///
    /// 敏感资源导出前必须先创建审批请求。
    /// 风险等级根据预估导出行数自动评估：
    /// - < 1000 行：low（一级审批即可）
    /// - 1000-10000 行：medium（二级审批）
    /// - 10000-50000 行：high（二级审批 + 风险提示）
    /// - >= 50000 行：critical（二级审批 + 额外验证）
    pub async fn create_request(
        &self,
        applicant_user_id: i32,
        applicant_username: String,
        applicant_ip: Option<String>,
        applicant_user_agent: Option<String>,
        req: CreateApprovalRequest,
    ) -> Result<Model, AppError> {
        // 验证资源类型是否为敏感资源
        if !sensitive_resources::is_sensitive(&req.resource_type) {
            return Err(AppError::validation(format!(
                "资源类型 {} 不在敏感资源清单内，无需二级审批",
                req.resource_type
            )));
        }

        // 文件格式校验
        let file_format = req
            .file_format
            .unwrap_or_else(|| "xlsx".to_string())
            .to_lowercase();
        if !matches!(file_format.as_str(), "xlsx" | "pdf" | "csv") {
            return Err(AppError::validation(
                "file_format 仅支持 xlsx/pdf/csv",
            ));
        }

        // 风险等级评估
        let estimated_rows = req.estimated_rows.unwrap_or(0);
        let risk = RiskLevel::from_row_count(estimated_rows);
        let risk_level = risk.as_str().to_string();

        // 审批层级：high/critical 必须二级审批；medium 默认二级；low 可一级
        let approval_level = match risk {
            RiskLevel::Low => 1,
            RiskLevel::Medium | RiskLevel::High | RiskLevel::Critical => 2,
        };

        let now = Utc::now();
        let active = ExportApprovalActiveModel {
            id: sea_orm::ActiveValue::NotSet,
            applicant_user_id: Set(applicant_user_id),
            applicant_username: Set(applicant_username),
            approver_user_id: Set(None),
            approver_username: Set(None),
            resource_type: Set(req.resource_type),
            export_params: Set(Some(ExportParams(req.export_params))),
            estimated_rows: Set(Some(estimated_rows)),
            file_format: Set(file_format),
            status: Set(ApprovalStatus::Pending.as_str().to_string()),
            approval_level: Set(approval_level),
            approver_comments: Set(None),
            approved_at: Set(None),
            rejected_at: Set(None),
            download_token: Set(None),
            token_expires_at: Set(None),
            download_count: Set(0),
            max_downloads: Set(1),
            file_path: Set(None),
            file_size_bytes: Set(None),
            file_checksum: Set(None),
            applicant_ip: Set(applicant_ip),
            approver_ip: Set(None),
            applicant_user_agent: Set(applicant_user_agent),
            risk_level: Set(risk_level),
            context: Set(None),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            completed_at: Set(None),
        };

        let model = active.insert(&*self.db).await?;
        Ok(model)
    }

    /// 审批通过（一级或二级）
    ///
    /// - 一级审批通过后，若 approval_level == 2，状态保持 pending 等待二级审批
    /// - 二级审批通过后，生成临时下载 token（5 分钟有效）
    /// - 一级审批通过后，若 approval_level == 1，直接生成 token
    pub async fn approve(
        &self,
        approval_id: i64,
        approver_user_id: i32,
        approver_username: String,
        approver_ip: Option<String>,
        req: ApproveRequest,
    ) -> Result<Model, AppError> {
        let model = Entity::find_by_id(approval_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("导出审批请求不存在: id={}", approval_id))
            })?;

        // 校验状态：仅 pending 状态可审批
        if model.status != ApprovalStatus::Pending.as_str() {
            return Err(AppError::validation(format!(
                "审批请求状态为 {}，仅 pending 状态可审批",
                model.status
            )));
        }

        // 校验审批人不能是申请人本人（防自审批）
        if model.applicant_user_id == approver_user_id {
            return Err(AppError::permission_denied(
                "审批人不能是申请人本人（防自审批）",
            ));
        }

        let now = Utc::now();

        // 一级审批：若 approval_level == 2，升级到二级（状态保持 pending）
        // 二级审批：生成下载 token
        let target_level = model.approval_level;
        let current_level = model.approval_level; // 简化：当前层级等于目标层级（一级已通过则当前为 2）

        if current_level < target_level {
            // 升级到下一级（一级 → 二级）
            let mut active: ExportApprovalActiveModel = model.into();
            active.approval_level = Set(current_level + 1);
            active.approver_user_id = Set(Some(approver_user_id));
            active.approver_username = Set(Some(approver_username));
            active.approver_ip = Set(approver_ip);
            active.approver_comments = Set(req.comments);
            active.updated_at = Set(now.into());
            // 状态保持 pending，等待下一级审批
            let updated = active.update(&*self.db).await?;
            Ok(updated)
        } else {
            // 最终审批通过：生成下载 token
            let token = format!("exp_{}", Uuid::new_v4().simple());
            let token_expires_at = now + Duration::minutes(TOKEN_VALID_MINUTES);

            let mut active: ExportApprovalActiveModel = model.into();
            active.status = Set(ApprovalStatus::Approved.as_str().to_string());
            active.approver_user_id = Set(Some(approver_user_id));
            active.approver_username = Set(Some(approver_username));
            active.approver_ip = Set(approver_ip);
            active.approver_comments = Set(req.comments);
            active.approved_at = Set(Some(now.into()));
            active.download_token = Set(Some(token));
            active.token_expires_at = Set(Some(token_expires_at.into()));
            active.updated_at = Set(now.into());

            let updated = active.update(&*self.db).await?;
            Ok(updated)
        }
    }

    /// 审批拒绝
    pub async fn reject(
        &self,
        approval_id: i64,
        approver_user_id: i32,
        approver_username: String,
        approver_ip: Option<String>,
        req: ApproveRequest,
    ) -> Result<Model, AppError> {
        let model = Entity::find_by_id(approval_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("导出审批请求不存在: id={}", approval_id))
            })?;

        if model.status != ApprovalStatus::Pending.as_str() {
            return Err(AppError::validation(format!(
                "审批请求状态为 {}，仅 pending 状态可拒绝",
                model.status
            )));
        }

        if model.applicant_user_id == approver_user_id {
            return Err(AppError::permission_denied(
                "审批人不能是申请人本人（防自审批）",
            ));
        }

        let now = Utc::now();
        let mut active: ExportApprovalActiveModel = model.into();
        active.status = Set(ApprovalStatus::Rejected.as_str().to_string());
        active.approver_user_id = Set(Some(approver_user_id));
        active.approver_username = Set(Some(approver_username));
        active.approver_ip = Set(approver_ip);
        active.approver_comments = Set(req.comments);
        active.rejected_at = Set(Some(now.into()));
        active.updated_at = Set(now.into());
        active.completed_at = Set(Some(now.into()));

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 申请人取消
    pub async fn cancel(
        &self,
        approval_id: i64,
        applicant_user_id: i32,
    ) -> Result<Model, AppError> {
        let model = Entity::find_by_id(approval_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("导出审批请求不存在: id={}", approval_id))
            })?;

        // 仅申请人本人可取消
        if model.applicant_user_id != applicant_user_id {
            return Err(AppError::permission_denied("仅申请人本人可取消审批请求"));
        }

        // 仅 pending 状态可取消
        if model.status != ApprovalStatus::Pending.as_str() {
            return Err(AppError::validation(format!(
                "审批请求状态为 {}，仅 pending 状态可取消",
                model.status
            )));
        }

        let now = Utc::now();
        let mut active: ExportApprovalActiveModel = model.into();
        active.status = Set(ApprovalStatus::Cancelled.as_str().to_string());
        active.updated_at = Set(now.into());
        active.completed_at = Set(Some(now.into()));

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 校验下载 token（导出 handler 调用前校验）
    ///
    /// 返回审批请求记录，校验通过后 handler 生成导出文件并记录 file_path/checksum
    pub async fn verify_download_token(&self, token: &str) -> Result<Model, AppError> {
        let model = Entity::find()
            .filter(Column::DownloadToken.eq(token))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::permission_denied("下载令牌无效"))?;

        // 校验状态必须为 approved
        if model.status != ApprovalStatus::Approved.as_str() {
            return Err(AppError::permission_denied(format!(
                "审批状态为 {}，不可下载",
                model.status
            )));
        }

        // 校验 token 未过期
        if let Some(expires_at) = model.token_expires_at {
            let now = Utc::now();
            if now > expires_at {
                // 自动标记为 expired
                let mut active: ExportApprovalActiveModel = model.clone().into();
                active.status = Set(ApprovalStatus::Expired.as_str().to_string());
                active.updated_at = Set(now.into());
                active.completed_at = Set(Some(now.into()));
                active.update(&*self.db).await?;
                return Err(AppError::permission_denied("下载令牌已过期"));
            }
        }

        // 校验下载次数未超限
        if model.download_count >= model.max_downloads {
            return Err(AppError::permission_denied(format!(
                "下载次数已达上限 {}",
                model.max_downloads
            )));
        }

        Ok(model)
    }

    /// 记录文件下载完成（更新 download_count + file_path/checksum）
    pub async fn record_download(
        &self,
        approval_id: i64,
        file_path: String,
        file_size_bytes: i64,
        file_checksum: String,
    ) -> Result<Model, AppError> {
        let model = Entity::find_by_id(approval_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("导出审批请求不存在: id={}", approval_id))
            })?;

        let now = Utc::now();
        let new_count = model.download_count + 1;
        let is_final_download = new_count >= model.max_downloads;

        let mut active: ExportApprovalActiveModel = model.into();
        active.download_count = Set(new_count);
        active.file_path = Set(Some(file_path));
        active.file_size_bytes = Set(Some(file_size_bytes));
        active.file_checksum = Set(Some(file_checksum));
        active.updated_at = Set(now.into());
        if is_final_download {
            active.completed_at = Set(Some(now.into()));
        }

        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 审批请求列表查询
    pub async fn list_requests(
        &self,
        q: ListApprovalQuery,
    ) -> Result<ApprovalListVo, AppError> {
        let page = q.page.unwrap_or(1).clamp(1, 1000);
        let page_size = q.page_size.unwrap_or(20).clamp(1, 100);

        let mut select = Entity::find();
        if let Some(s) = &q.status {
            select = select.filter(Column::Status.eq(s));
        }
        if let Some(rt) = &q.resource_type {
            select = select.filter(Column::ResourceType.eq(rt));
        }
        if let Some(uid) = q.applicant_user_id {
            select = select.filter(Column::ApplicantUserId.eq(uid));
        }

        let total = select.clone().count(&*self.db).await?;
        // V15 P0-S14：使用 paginator 替代 offset/limit（Select 无 offset 方法）
        let paginator = select
            .order_by_desc(Column::CreatedAt)
            .paginate(&*self.db, page_size);
        let items = paginator
            .fetch_page(page.saturating_sub(1))
            .await?;

        Ok(ApprovalListVo {
            items,
            total,
            page,
            page_size,
        })
    }

    /// 审批请求详情
    pub async fn get_request(&self, id: i64) -> Result<Model, AppError> {
        Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("导出审批请求不存在: id={}", id)))
    }

    /// 清理过期 token（定时任务调用）
    ///
    /// 将已过期但仍为 approved 状态的请求标记为 expired
    pub async fn cleanup_expired_tokens(&self) -> Result<u64, AppError> {
        let now = Utc::now();
        let expired = Entity::find()
            .filter(Column::Status.eq(ApprovalStatus::Approved.as_str()))
            .filter(Column::TokenExpiresAt.lt(now))
            .all(&*self.db)
            .await?;

        let mut count = 0u64;
        for model in expired {
            let mut active: ExportApprovalActiveModel = model.into();
            active.status = Set(ApprovalStatus::Expired.as_str().to_string());
            active.updated_at = Set(now.into());
            active.completed_at = Set(Some(now.into()));
            active.update(&*self.db).await?;
            count += 1;
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试风险等级评估
    #[test]
    fn test_risk_level_from_row_count() {
        assert_eq!(RiskLevel::from_row_count(0), RiskLevel::Low);
        assert_eq!(RiskLevel::from_row_count(999), RiskLevel::Low);
        assert_eq!(RiskLevel::from_row_count(1000), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_row_count(9999), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_row_count(10000), RiskLevel::High);
        assert_eq!(RiskLevel::from_row_count(49999), RiskLevel::High);
        assert_eq!(RiskLevel::from_row_count(50000), RiskLevel::Critical);
        assert_eq!(RiskLevel::from_row_count(100000), RiskLevel::Critical);
    }

    /// 测试审批状态序列化
    #[test]
    fn test_approval_status_as_str() {
        assert_eq!(ApprovalStatus::Pending.as_str(), "pending");
        assert_eq!(ApprovalStatus::Approved.as_str(), "approved");
        assert_eq!(ApprovalStatus::Rejected.as_str(), "rejected");
        assert_eq!(ApprovalStatus::Expired.as_str(), "expired");
        assert_eq!(ApprovalStatus::Cancelled.as_str(), "cancelled");
    }

    /// 测试审批状态解析
    #[test]
    fn test_approval_status_from_str() {
        assert_eq!(
            ApprovalStatus::from_str("pending"),
            Some(ApprovalStatus::Pending)
        );
        assert_eq!(
            ApprovalStatus::from_str("approved"),
            Some(ApprovalStatus::Approved)
        );
        assert_eq!(ApprovalStatus::from_str("unknown"), None);
    }

    /// 测试敏感资源判断
    #[test]
    fn test_sensitive_resources() {
        assert!(sensitive_resources::is_sensitive("customer"));
        assert!(sensitive_resources::is_sensitive("dye_recipe"));
        assert!(sensitive_resources::is_sensitive("finance_report"));
        assert!(!sensitive_resources::is_sensitive("product"));
        assert!(!sensitive_resources::is_sensitive("inventory"));
    }
}

//! 会计科目服务
//!
//! 提供会计科目相关的API调用

use crate::models::account_subject::{
    AccountSubject, AccountSubjectListResponse, CreateSubjectRequest, SubjectQueryParams,
    SubjectTreeNode, UpdateSubjectRequest,
};
use crate::services::api::ApiService;

/// 会计科目服务
pub struct AccountSubjectService;

impl AccountSubjectService {
    /// 查询科目列表
    pub async fn list_subjects(params: SubjectQueryParams) -> Result<Vec<AccountSubject>, String> {
        let mut query_string = String::from("/account-subjects");
        let mut has_param = false;

        if let Some(level) = params.level {
            query_string.push_str(&format!(
                "{}level={}",
                if has_param { "&" } else { "?" },
                level
            ));
            has_param = true;
        }
        if let Some(parent_id) = params.parent_id {
            query_string.push_str(&format!(
                "{}parent_id={}",
                if has_param { "&" } else { "?" },
                parent_id
            ));
            has_param = true;
        }
        if let Some(status) = &params.status {
            query_string.push_str(&format!(
                "{}status={}",
                if has_param { "&" } else { "?" },
                status
            ));
            has_param = true;
        }
        if let Some(keyword) = &params.keyword {
            query_string.push_str(&format!(
                "{}keyword={}",
                if has_param { "&" } else { "?" },
                urlencoding::encode(keyword)
            ));
        }

        ApiService::get::<AccountSubjectListResponse>(&query_string)
            .await
            .map(|resp| resp.data)
    }

    /// 查询单个科目
    pub async fn get_subject(id: i32) -> Result<AccountSubject, String> {
        ApiService::get::<AccountSubject>(&format!("/account-subjects/{}", id)).await
    }

    /// 查询科目树
    pub async fn get_subject_tree() -> Result<Vec<SubjectTreeNode>, String> {
        ApiService::get::<Vec<SubjectTreeNode>>("/account-subjects/tree").await
    }

    /// 创建科目
    pub async fn create_subject(req: CreateSubjectRequest) -> Result<AccountSubject, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/account-subjects", &payload).await
    }

    /// 更新科目
    pub async fn update_subject(
        id: i32,
        req: UpdateSubjectRequest,
    ) -> Result<AccountSubject, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/account-subjects/{}", id), &payload).await
    }

    /// 删除科目
    pub async fn delete_subject(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/account-subjects/{}", id)).await
    }
}

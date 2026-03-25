//! 会计科目服务
//!
//! 提供会计科目相关的API调用

use crate::services::api::ApiService;

/// 会计科目数据模型
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct AccountSubject {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub level: i32,
    pub parent_id: Option<i32>,
    pub balance_direction: Option<String>,
    pub assist_customer: bool,
    pub assist_supplier: bool,
    pub assist_batch: bool,
    pub assist_color_no: bool,
    pub enable_dual_unit: bool,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 科目树节点
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SubjectTreeNode {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub level: i32,
    pub children: Vec<SubjectTreeNode>,
}

/// 科目列表响应
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AccountSubjectListResponse {
    pub data: Vec<AccountSubject>,
}

/// 查询参数
#[derive(Debug, Clone, serde::Serialize)]
pub struct SubjectQueryParams {
    pub level: Option<i32>,
    pub parent_id: Option<i32>,
    pub status: Option<String>,
    pub keyword: Option<String>,
}

/// 创建科目请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct CreateSubjectRequest {
    pub code: String,
    pub name: String,
    pub level: i32,
    pub parent_id: Option<i32>,
    pub balance_direction: Option<String>,
    pub assist_customer: bool,
    pub assist_supplier: bool,
    pub assist_batch: bool,
    pub assist_color_no: bool,
    pub enable_dual_unit: bool,
}

/// 更新科目请求
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateSubjectRequest {
    pub name: Option<String>,
    pub balance_direction: Option<String>,
    pub assist_customer: bool,
    pub assist_supplier: bool,
    pub assist_batch: bool,
    pub assist_color_no: bool,
    pub enable_dual_unit: bool,
}

/// 会计科目服务
pub struct AccountSubjectService;

impl AccountSubjectService {
    /// 查询科目列表
    pub async fn list_subjects(params: SubjectQueryParams) -> Result<Vec<AccountSubject>, String> {
        let mut query_string = String::from("/api/v1/erp/account-subjects");
        let mut has_param = false;

        if let Some(level) = params.level {
            query_string.push_str(&format!("{}level={}", if has_param { "&" } else { "?" }, level));
            has_param = true;
        }
        if let Some(parent_id) = params.parent_id {
            query_string.push_str(&format!("{}parent_id={}", if has_param { "&" } else { "?" }, parent_id));
            has_param = true;
        }
        if let Some(status) = &params.status {
            query_string.push_str(&format!("{}status={}", if has_param { "&" } else { "?" }, status));
            has_param = true;
        }
        if let Some(keyword) = &params.keyword {
            query_string.push_str(&format!("{}keyword={}", if has_param { "&" } else { "?" }, keyword));
        }

        ApiService::get::<AccountSubjectListResponse>(&query_string)
            .await
            .map(|resp| resp.data)
    }

    /// 查询单个科目
    pub async fn get_subject(id: i32) -> Result<AccountSubject, String> {
        ApiService::get::<AccountSubject>(&format!("/api/v1/erp/account-subjects/{}", id)).await
    }

    /// 查询科目树
    pub async fn get_subject_tree() -> Result<Vec<SubjectTreeNode>, String> {
        ApiService::get::<Vec<SubjectTreeNode>>("/api/v1/erp/account-subjects/tree").await
    }

    /// 创建科目
    pub async fn create_subject(req: CreateSubjectRequest) -> Result<AccountSubject, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::post("/api/v1/erp/account-subjects", &payload).await
    }

    /// 更新科目
    pub async fn update_subject(id: i32, req: UpdateSubjectRequest) -> Result<AccountSubject, String> {
        let payload = serde_json::to_value(&req).map_err(|e| e.to_string())?;
        ApiService::put(&format!("/api/v1/erp/account-subjects/{}", id), &payload).await
    }

    /// 删除科目
    pub async fn delete_subject(id: i32) -> Result<(), String> {
        ApiService::delete(&format!("/api/v1/erp/account-subjects/{}", id)).await
    }
}

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

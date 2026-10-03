use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use axum::{Json, extract::State};
use serde::Deserialize;
use validator::Validate;

use crate::services::department_service::DepartmentService;
use crate::services::department_service::DepartmentTreeNode;
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 查询参数 - 部门列表
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct DepartmentListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub parent_id: Option<i32>,
    pub search: Option<String>,
}

/// 创建部门请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct CreateDepartmentRequest {
    #[validate(length(min = 1, max = 100, message = "部门名称不能为空且最长100字符"))]
    pub name: String,
    /// 部门编码（契约对齐：前端 DepartmentCreateRequest.code；不传时后端自动生成 DEPT_时间戳）
    #[validate(length(max = 50, message = "部门编码最长50字符"))]
    pub code: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    pub manager_id: Option<i32>,
    /// 排序号（契约对齐：前端 DepartmentCreateRequest.sort_order；不传时默认 0）
    pub sort_order: Option<i32>,
}

/// JSON 三态反序列化适配器（RFC 7386 JSON Merge Patch 的"键缺席 ≠ 显式 null"语义所需）。
///
/// 为何需要：serde_json 对 `Option<Option<T>>` 的默认反序列化在遇到 JSON null 时
/// 直接调 visit_none()，把"显式 null"塌成外层 `None`，与"键缺席"不可区分。
/// 本适配器把字段先按内层 `Option<T>` 反序列化再包一层：
/// 键缺席（配合 `#[serde(default)]`）= `None`、显式 null = `Some(None)`、有值 = `Some(Some(v))`。
/// 与 handlers/purchase_contract_handler.rs 中同名适配器形状一致（本批授权文件仅限
/// 合同/部门，各域 handler 内私有定义；跨域合并到共享工具需动 utils，超出本批授权范围）。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// 更新部门请求
///
/// 字段语义 = 显式三态部分更新（对齐 RFC 7386 JSON Merge Patch）：
/// 键缺席=保持原值、显式 `null`=清空为 NULL（仅 DB 可空列）、有值=覆盖。
/// NOT NULL 列（name/code，m0001 DDL；sort_order/is_active 实体 Model 为非 Option 列，
/// 置 NULL 该行将无法按模型读出）不开 null 清空，显式 null 由 service 层判业务错误拒绝。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateDepartmentRequest {
    /// 部门名称：NOT NULL 列——显式 null 被 service 拒绝（业务错误，非脱敏）
    #[serde(default, deserialize_with = "double_option")]
    #[validate(length(min = 1, max = 100, message = "部门名称不能为空且最长100字符"))]
    pub name: Option<Option<String>>,
    /// 部门编码：NOT NULL UNIQUE 列——显式 null 被 service 拒绝（P0 契约修复：前端编辑对话框可改 code，后端原缺该字段 ⇒ 静默丢失）
    #[serde(default, deserialize_with = "double_option")]
    #[validate(length(min = 1, max = 50, message = "部门编码不能为空且最长50字符"))]
    pub code: Option<Option<String>>,
    /// 描述：DB 可空列 description TEXT（m0001 DDL）——显式 null 清空
    #[serde(default, deserialize_with = "double_option")]
    pub description: Option<Option<String>>,
    /// 父部门：DB 可空列 parent_id（自引用外键）——显式 null = 脱离父级成为顶级部门
    #[serde(default, deserialize_with = "double_option")]
    pub parent_id: Option<Option<i32>>,
    /// 负责人用户 ID（真实列 departments.manager_id，可空——显式 null 清空负责人；manager_name 为后端回填的展示字段，前端不得提交）
    #[serde(default, deserialize_with = "double_option")]
    pub manager_id: Option<Option<i32>>,
    /// 排序号（实体 Model 列非 Option i32，置 NULL 该行无法按模型读出）：显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    pub sort_order: Option<Option<i32>>,
    /// 启用状态（实体 Model 列非 Option bool）：显式 null 被 service 拒绝
    #[serde(default, deserialize_with = "double_option")]
    #[serde(alias = "status", alias = "is_active")]
    pub is_active: Option<Option<bool>>,
}

// define_crud_handlers! 出参实参 = service 的真实返回类型（批次 476 契约收紧）：
// list -> PaginatedResponse<department::Model>，get/create/update -> department::Model
// （证据 services/department_service.rs:84/130/142/222）
crate::define_crud_handlers!(
    DepartmentService,
    CreateDepartmentRequest,
    UpdateDepartmentRequest,
    DepartmentListQuery,
    i32,
    crate::utils::response::PaginatedResponse<crate::models::department::Model>,
    crate::models::department::Model
);

/// 获取部门树形结构 (定制化额外路由)
pub async fn get_department_tree(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<Vec<DepartmentTreeNode>>>, AppError> {
    let department_service = DepartmentService::new(state.db.clone());
    let tree = department_service.get_department_tree().await?;
    Ok(Json(ApiResponse::success(tree)))
}

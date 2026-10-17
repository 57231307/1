//! 公司档案 Handler
//!
//! 对外端点(资源键 `company-profile`,URL 段 `/api/v1/erp/company-profile*` 派生):
//! - `GET  /company-profile`       [`get_company_profile`]        公开投影(仅 name/short_name/logo)
//! - `GET  /company-profile/full`  [`get_company_profile_full`]   全量档案(含敏感列,`company-profile:read`)
//! - `PUT  /company-profile/full`  [`update_company_profile_full`] 全量替换更新(`company-profile:update`)
//!
//! 安全设计(两硬约束的落点):
//! - `AUTH_ONLY_PATHS` 是**精确路径、方法无关**匹配(`is_auth_only_path` 在 RBAC/白名单前
//!   return),故公开面独占精确路径 `/company-profile`,且该路径只挂 GET;全量读写走另一
//!   精确路径 `/company-profile/full` 过正常 RBAC——若 GET/PUT 同路径,任意登录者都能借
//!   豁免面调用 PUT。
//! - 公开投影是**硬投影结构体** [`CompanyProfilePublic`]:类型层面只有 name/short_name/logo
//!   三字段,handler 逐字段手动构造,敏感列(credit_code/legal_representative/
//!   tax_registration_number/bank_name/bank_account/taxpayer_type)不经由本端点出入,
//!   杜绝"整行序列化顺手带出敏感列"的泄露形态。
//! - 出参键 = Model/投影结构体字段名(snake_case);失败只用 AppError 统一信封。

use chrono::NaiveDate;
use sea_orm::entity::prelude::Decimal;
use serde::{Deserialize, Serialize};
use tracing::warn;

use axum::Json;
use axum::extract::State;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::company_profile;
use crate::services::company_profile_service::{CompanyProfileDraft, CompanyProfileService};
use crate::utils::error::AppError;
use crate::utils::response::ApiResponse;

/// 公开投影:仅抬头展示三列。新增敏感列时**不得**扩充本结构——公开端点的类型即泄露边界。
#[allow(dead_code, reason = "序列化输出字段")]
#[derive(Debug, Clone, Serialize)]
pub struct CompanyProfilePublic {
    pub name: String,
    pub short_name: Option<String>,
    pub logo: Option<String>,
}

/// `PUT /company-profile/full` 请求体:全量替换语义——提交的字段即档案终态,
/// 未提交的可选字段清空为 NULL。长度上限逐列对齐建表 DDL,非法即 400 点名,
/// 不让超长串撞 DB 报错变成 500。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateCompanyProfileRequest {
    pub name: String,
    #[serde(default)]
    pub short_name: Option<String>,
    #[serde(default)]
    pub credit_code: Option<String>,
    #[serde(default)]
    pub legal_representative: Option<String>,
    #[serde(default)]
    pub tax_registration_number: Option<String>,
    #[serde(default)]
    pub bank_name: Option<String>,
    #[serde(default)]
    pub bank_account: Option<String>,
    #[serde(default)]
    pub taxpayer_type: Option<String>,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub fax: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub registered_capital: Option<Decimal>,
    #[serde(default)]
    pub establishment_date: Option<NaiveDate>,
    #[serde(default)]
    pub logo: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// 请求体各列长度上限校验(与 `company_profile` DDL 逐列一致;TEXT 列不设长度校验)。
/// 命中即 400 中文点名,失败日志由 [`warn`] 落痕,不吞不落库。
fn validate_lengths(req: &UpdateCompanyProfileRequest) -> Result<(), AppError> {
    let checks: [(&str, Option<&str>, usize); 11] = [
        ("简称", req.short_name.as_deref(), 100),
        ("统一社会信用代码", req.credit_code.as_deref(), 50),
        ("法定代表人", req.legal_representative.as_deref(), 100),
        ("税务登记号", req.tax_registration_number.as_deref(), 50),
        ("开户银行", req.bank_name.as_deref(), 200),
        ("银行账号", req.bank_account.as_deref(), 100),
        ("纳税人类型", req.taxpayer_type.as_deref(), 50),
        ("电话", req.phone.as_deref(), 50),
        ("传真", req.fax.as_deref(), 50),
        ("邮箱", req.email.as_deref(), 100),
        ("网址", req.website.as_deref(), 200),
    ];
    if req.name.chars().count() > 200 {
        warn!(len = req.name.chars().count(), "公司档案名称超长");
        return Err(AppError::validation_displayable(
            "公司名称长度不能超过 200 字符",
        ));
    }
    for (label, value, max) in checks {
        if let Some(v) = value {
            if v.chars().count() > max {
                warn!(
                    field = label,
                    len = v.chars().count(),
                    max,
                    "公司档案字段超长"
                );
                return Err(AppError::validation_displayable(format!(
                    "{label}长度不能超过 {max} 字符"
                )));
            }
        }
    }
    Ok(())
}

/// 将请求体收敛为服务层草稿(全字段覆盖语义在此一次性成型)。
fn to_draft(req: UpdateCompanyProfileRequest) -> CompanyProfileDraft {
    CompanyProfileDraft {
        name: req.name,
        short_name: req.short_name,
        credit_code: req.credit_code,
        legal_representative: req.legal_representative,
        tax_registration_number: req.tax_registration_number,
        bank_name: req.bank_name,
        bank_account: req.bank_account,
        taxpayer_type: req.taxpayer_type,
        phone: req.phone,
        fax: req.fax,
        email: req.email,
        website: req.website,
        address: req.address,
        registered_capital: req.registered_capital,
        establishment_date: req.establishment_date,
        logo: req.logo,
        description: req.description,
    }
}

/// `GET /company-profile`:公开投影,登录即可读(AUTH_ONLY 精确路径,免业务权限码)。
/// 只出 name/short_name/logo——由 [`CompanyProfilePublic`] 类型层保证,敏感列不经此路出入。
pub async fn get_company_profile(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<CompanyProfilePublic>>, AppError> {
    let service = CompanyProfileService::new(state.db.clone());
    let profile = service.get_profile().await?;
    let public = CompanyProfilePublic {
        name: profile.name,
        short_name: profile.short_name,
        logo: profile.logo,
    };
    Ok(Json(ApiResponse::success(public)))
}

/// `GET /company-profile/full`:全量档案(含敏感列),权限码 `company-profile:read`。
/// 出参键 = `company_profile` Model 字段名(snake_case)。
pub async fn get_company_profile_full(
    State(state): State<AppState>,
    _auth: AuthContext,
) -> Result<Json<ApiResponse<company_profile::Model>>, AppError> {
    let service = CompanyProfileService::new(state.db.clone());
    let profile = service.get_profile().await?;
    Ok(Json(ApiResponse::success(profile)))
}

/// `PUT /company-profile/full`:全量替换更新,权限码 `company-profile:update`。
/// 校验(name 非空、logo PNG 魔数与 512KB、列长)在服务层与本函数前置完成,
/// 失败走 AppError 统一信封;成功记录 `updated_by` = 当前 user_id。
pub async fn update_company_profile_full(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateCompanyProfileRequest>,
) -> Result<Json<ApiResponse<company_profile::Model>>, AppError> {
    validate_lengths(&req)?;
    let draft = to_draft(req);
    let service = CompanyProfileService::new(state.db.clone());
    let profile = service.update_profile(draft, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message(
        profile,
        "公司档案更新成功",
    )))
}

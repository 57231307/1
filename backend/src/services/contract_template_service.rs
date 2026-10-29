//! 合同模板读写腿服务
//!
//! 功能 合同模板主数据与模板条款项的读、新建、改版、法务复核、置启用、停用与删除，
//!   以及条款项的增改删与逐条复核。启用语义照本仓 print_template 的 ACTIVE 门范式：
//!   同一 (contract_type, party_side) 同时只允许一版 active（库侧部分唯一索引
//!   `uq_contract_templates_active_type_side` 兜底，本服务在事务内先锁竞位行做点名拒绝），
//!   置 active 必须已经带 approved_by（复核腿只在本服务单点写入）。非法跃迁一律点名拒绝，
//!   不静默改动任何其他行的状态。改版是新增一行 version 递增的 draft，并把条款项复制过去、
//!   逐条复核状态复位 pending（改版必须重新过复核，不留旧复核结论的复用通道）。
//! 调用方 `handlers/contract_template_handler.rs` 的八个端点与集成测试。
//! 入参 各请求结构体与登录用户主键；类别立场状态条款码适用条件一律以
//!   `models::status::contract_template` 权威词表逐字符校验，词表外点名拒绝。
//! 传给谁 读写经 SeaORM Entity 落 PostgreSQL `contract_templates` 与
//!   `contract_template_clauses`，条款库正文只读 `clause_library`。
//! 存什么 模板行（编码加版本组合唯一，生命周期 draft active retired）、条款项行
//!   （正文与条款库引用二选一、顺序、必填标记、条件出条款标记、逐条复核留痕）。
//! 存哪里 PostgreSQL 上述两表；本服务不写合同条款快照，快照属签约固化腿
//!   `services/contract_clause_service.rs`。
//!
//! 状态机（唯一合法出边表，表外跃迁点名拒绝）
//! - draft → active（activate，须已带 approved_by，且同 (contract_type, party_side) 无在位 active）
//! - draft → retired（retire，草稿主动下线）
//! - active → retired（retire，被新版取代或停用）
//! - retired 为终态，retired 之后的任何跃迁一律拒绝；active → draft 同样拒绝
//! - 删除只允许 draft，且未被合同条款快照引用

use crate::models::clause_library;
use crate::models::contract_template;
use crate::models::contract_template_clause;
use crate::models::status::contract_template::{
    contract_applies_when, contract_clause_code, contract_legal_review_status, contract_party_side,
    contract_template_status, contract_type,
};
use crate::utils::error::AppError;
use chrono::NaiveDate;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;
use tracing::warn;

/// 模板条款排序号的库侧约束域，超出即点名拒绝（与 m0116 的 chk_ctc_sort 同界）。
const SORT_MIN: i16 = 1;
const SORT_MAX: i16 = 999;

/// 列表查询参数，键全部可缺席；非空值逐词表校验，词表外点名拒绝而不是静默空结果。
#[derive(Debug, Clone, Default)]
pub struct TemplateListQuery {
    pub template_code: Option<String>,
    pub contract_type: Option<String>,
    pub party_side: Option<String>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

/// 新建模板请求，模板编码必填（改版沿用同码递增版本，编码在创建时即业务身份）。
#[derive(Debug, Clone)]
pub struct CreateTemplateRequest {
    pub template_code: String,
    pub contract_type: String,
    pub party_side: String,
    pub title: String,
    pub effective_from: Option<NaiveDate>,
    pub remark: Option<String>,
    /// 建模板时一并写入的条款项（可空，条款可后续经条款项端点维护）
    pub clauses: Option<Vec<UpsertClauseRequest>>,
}

/// 条款项写入请求。正文与条款库引用恰好给一个（与 chk_ctc_text_source 同判据，
/// 服务层先点名拒绝，不等数据库冒裸 CHECK 错）。
#[derive(Debug, Clone)]
pub struct UpsertClauseRequest {
    pub clause_code: String,
    pub title: String,
    pub body: Option<String>,
    pub library_clause_id: Option<i32>,
    pub required: Option<bool>,
    pub sort: i16,
    pub applies_when: Option<String>,
}

/// 模板表头更新请求（仅 draft 可改）。三态语义与合同头更新链路一致：
/// 键缺席保持、显式 null 清空（仅可空列）、有值覆盖。
#[derive(Debug, Clone, Default)]
pub struct UpdateTemplateHeaderRequest {
    pub title: Option<Option<String>>,
    pub effective_from: Option<Option<NaiveDate>>,
    pub remark: Option<Option<String>>,
}

/// 逐条法务复核请求，只允许 pending → approved 或 pending → rejected 两条出边。
#[derive(Debug, Clone)]
pub struct ReviewClauseRequest {
    pub legal_review_status: String,
    pub legal_review_remark: Option<String>,
}

/// 条款项出参，条款库引用行把库正文一并带出，读侧不必二次请求。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ClauseView {
    #[serde(flatten)]
    pub clause: contract_template_clause::Model,
    /// library_clause_id 引用行的正文，自带 body 的行此列为 None
    pub library_body: Option<String>,
}

pub struct ContractTemplateService {
    db: Arc<DatabaseConnection>,
}

impl ContractTemplateService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 词表校验的统一入口：token 不在权威 `ALL` 内即点名拒绝（可外显，token 是入参原值）。
    fn require_vocab(field: &str, token: &str, allowed: &[&str]) -> Result<(), AppError> {
        if allowed.contains(&token) {
            Ok(())
        } else {
            Err(AppError::validation_displayable(format!(
                "{field} 取值 {token} 不在允许范围内（允许：{}）",
                allowed.join("、")
            )))
        }
    }

    /// 分页列表。筛选值先过词表，页码钳位与合同列表链路同口径。
    pub async fn list(
        &self,
        params: TemplateListQuery,
    ) -> Result<(Vec<contract_template::Model>, u64), AppError> {
        if let Some(v) = params.contract_type.as_deref() {
            Self::require_vocab("contract_type", v, contract_type::ALL)?;
        }
        if let Some(v) = params.party_side.as_deref() {
            Self::require_vocab("party_side", v, contract_party_side::ALL)?;
        }
        if let Some(v) = params.status.as_deref() {
            Self::require_vocab("status", v, contract_template_status::ALL)?;
        }

        let mut query = contract_template::Entity::find();
        if let Some(code) = params.template_code.filter(|s| !s.is_empty()) {
            query = query.filter(contract_template::Column::TemplateCode.eq(code));
        }
        if let Some(v) = params.contract_type {
            query = query.filter(contract_template::Column::ContractType.eq(v));
        }
        if let Some(v) = params.party_side {
            query = query.filter(contract_template::Column::PartySide.eq(v));
        }
        if let Some(v) = params.status {
            query = query.filter(contract_template::Column::Status.eq(v));
        }

        let total = query.clone().count(&*self.db).await?;
        let rows = query
            .order_by(contract_template::Column::TemplateCode, Order::Asc)
            .order_by(contract_template::Column::Version, Order::Desc)
            .offset(((params.page.clamp(1, 1000) - 1) * params.page_size) as u64)
            .limit(params.page_size.clamp(1, 100) as u64)
            .all(&*self.db)
            .await?;
        Ok((rows, total))
    }

    /// 按主键取模板行，不存在即 404 点名（编码是用户可识别的业务身份，可外显）。
    pub async fn get_by_id(&self, id: i32) -> Result<contract_template::Model, AppError> {
        contract_template::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("合同模板不存在（id {id}）")))
    }

    /// 取模板的条款项出参列表（含条款库引用正文），按 sort 升序。
    pub async fn list_clause_views(&self, template_id: i32) -> Result<Vec<ClauseView>, AppError> {
        let clauses = contract_template_clause::Entity::find()
            .filter(contract_template_clause::Column::TemplateId.eq(template_id))
            .order_by(contract_template_clause::Column::Sort, Order::Asc)
            .all(&*self.db)
            .await?;
        let lib_ids: Vec<i32> = clauses.iter().filter_map(|c| c.library_clause_id).collect();
        let lib_map: std::collections::HashMap<i32, String> = if lib_ids.is_empty() {
            Default::default()
        } else {
            clause_library::Entity::find()
                .filter(clause_library::Column::Id.is_in(lib_ids))
                .all(&*self.db)
                .await?
                .into_iter()
                .map(|l| (l.id, l.body))
                .collect()
        };
        Ok(clauses
            .into_iter()
            .map(|c| {
                let library_body = c
                    .library_clause_id
                    .and_then(|lid| lib_map.get(&lid).cloned());
                ClauseView {
                    clause: c,
                    library_body,
                }
            })
            .collect())
    }

    /// 新建模板（v1，draft 起步）。同编码已有 v1 即点名拒绝（改版走 revise，不覆盖存量版本）。
    pub async fn create(
        &self,
        req: CreateTemplateRequest,
        user_id: i32,
    ) -> Result<contract_template::Model, AppError> {
        let code = req.template_code.trim().to_string();
        if code.is_empty() {
            return Err(AppError::validation_displayable("模板编码不能为空"));
        }
        let title = req.title.trim().to_string();
        if title.is_empty() {
            return Err(AppError::validation_displayable("模板标题不能为空"));
        }
        Self::require_vocab("contract_type", &req.contract_type, contract_type::ALL)?;
        Self::require_vocab("party_side", &req.party_side, contract_party_side::ALL)?;

        let txn = (*self.db).begin().await?;
        let dup = contract_template::Entity::find()
            .filter(contract_template::Column::TemplateCode.eq(code.clone()))
            .filter(contract_template::Column::Version.eq(1))
            .one(&txn)
            .await?;
        if dup.is_some() {
            return Err(AppError::business_displayable(format!(
                "模板编码 {code} 的第 1 版已存在，改版请走版本递增通道，不允许覆盖既有版本"
            )));
        }

        let created = contract_template::ActiveModel {
            template_code: Set(code.clone()),
            contract_type: Set(req.contract_type.clone()),
            party_side: Set(req.party_side.clone()),
            version: Set(1),
            title: Set(title),
            status: Set(contract_template_status::DRAFT.to_string()),
            effective_from: Set(req.effective_from),
            created_by: Set(user_id),
            approved_by: Set(None),
            approved_at: Set(None),
            remark: Set(req.remark.clone()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        for clause in req.clauses.as_deref().unwrap_or_default() {
            Self::insert_clause_txn(&txn, created.id, clause).await?;
        }

        txn.commit().await?;
        Ok(created)
    }

    /// 改版：以源行为底新增一行 version 递增的 draft，条款项整组复制且逐条复核复位 pending。
    /// 源行状态不动、不改码、不并行为 active（active 互斥由 activate 门钉）。
    pub async fn revise(
        &self,
        source_id: i32,
        user_id: i32,
    ) -> Result<contract_template::Model, AppError> {
        let txn = (*self.db).begin().await?;
        let source = contract_template::Entity::find_by_id(source_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("合同模板不存在（id {source_id}）")))?;

        let max_version: Option<i32> = contract_template::Entity::find()
            .filter(contract_template::Column::TemplateCode.eq(source.template_code.clone()))
            .order_by(contract_template::Column::Version, Order::Desc)
            .limit(1)
            .one(&txn)
            .await?
            .map(|r| r.version);
        let new_version = max_version.unwrap_or(source.version).saturating_add(1);

        let created = contract_template::ActiveModel {
            template_code: Set(source.template_code.clone()),
            contract_type: Set(source.contract_type.clone()),
            party_side: Set(source.party_side.clone()),
            version: Set(new_version),
            title: Set(source.title.clone()),
            status: Set(contract_template_status::DRAFT.to_string()),
            effective_from: Set(None),
            created_by: Set(user_id),
            approved_by: Set(None),
            approved_at: Set(None),
            remark: Set(source.remark.clone()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        let source_clauses = contract_template_clause::Entity::find()
            .filter(contract_template_clause::Column::TemplateId.eq(source_id))
            .order_by(contract_template_clause::Column::Sort, Order::Asc)
            .all(&txn)
            .await?;
        for c in source_clauses {
            contract_template_clause::ActiveModel {
                template_id: Set(created.id),
                clause_code: Set(c.clause_code),
                title: Set(c.title),
                body: Set(c.body),
                library_clause_id: Set(c.library_clause_id),
                required: Set(c.required),
                sort: Set(c.sort),
                applies_when: Set(c.applies_when),
                legal_review_status: Set(contract_legal_review_status::PENDING.to_string()),
                legal_reviewed_by: Set(None),
                legal_reviewed_at: Set(None),
                legal_review_remark: Set(None),
                ..Default::default()
            }
            .insert(&txn)
            .await?;
        }

        txn.commit().await?;
        Ok(created)
    }

    /// 更新表头（仅 draft）。title 是 NOT NULL 列，显式 null 点名拒绝；
    /// contract_type 与 party_side 与 status 与编码与版本不开改写口——
    /// 启用身份与分类归属以整版行为准，改分类等于换模板，走新版。
    pub async fn update_header(
        &self,
        id: i32,
        req: UpdateTemplateHeaderRequest,
        user_id: i32,
    ) -> Result<contract_template::Model, AppError> {
        if matches!(req.title, Some(None)) {
            return Err(AppError::business_displayable(
                "模板标题不能清空：该字段为必填项",
            ));
        }
        let txn = (*self.db).begin().await?;
        let existing = contract_template::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("合同模板不存在（id {id}）")))?;
        if existing.status != contract_template_status::DRAFT {
            let code = existing.template_code.clone();
            let version = existing.version;
            return Err(AppError::business_displayable(format!(
                "模板 {code} v{version} 不是草稿状态，表头不可修改；需要调整内容请改版出新草稿"
            )));
        }
        let mut active: contract_template::ActiveModel = existing.into();
        if let Some(Some(v)) = req.title {
            let v = v.trim().to_string();
            if v.is_empty() {
                return Err(AppError::validation_displayable("模板标题不能为空白"));
            }
            active.title = Set(v);
        }
        if let Some(v) = req.effective_from {
            active.effective_from = Set(v);
        }
        if let Some(v) = req.remark {
            active.remark = Set(v);
        }
        active.updated_at = Set(Utc::now());
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        tracing::info!("用户 {user_id} 更新合同模板表头 id={id}");
        Ok(updated)
    }

    /// 法务复核通过腿：单点写入 approved_by 与 approved_at。仅 draft 可复核；
    /// 已复核过的行再次复核点名拒绝（复核结论不得被静默覆盖或刷时间）。
    pub async fn approve(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<contract_template::Model, AppError> {
        let txn = (*self.db).begin().await?;
        let existing = contract_template::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("合同模板不存在（id {id}）")))?;
        if existing.status != contract_template_status::DRAFT {
            let code = existing.template_code.clone();
            let version = existing.version;
            let status = existing.status.clone();
            return Err(AppError::business_displayable(format!(
                "模板 {code} v{version} 当前状态为 {status}，只有草稿可以记录法务复核通过"
            )));
        }
        if existing.approved_by.is_some() {
            let code = existing.template_code.clone();
            let version = existing.version;
            return Err(AppError::business_displayable(format!(
                "模板 {code} v{version} 已记录过法务复核通过，复核结论不可重复覆盖"
            )));
        }
        let now = Utc::now();
        let mut active: contract_template::ActiveModel = existing.into();
        active.approved_by = Set(Some(user_id));
        active.approved_at = Set(Some(now));
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 置启用。三道门逐条点名：起点必须 draft；必须已带 approved_by；
    /// 同 (contract_type, party_side) 已有在位 active 即拒绝并点名在位版的编码与版本，
    /// 绝不自动降位旧版（先下线旧版再启用新版是显式两步）。
    pub async fn activate(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<contract_template::Model, AppError> {
        let txn = (*self.db).begin().await?;
        let existing = contract_template::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("合同模板不存在（id {id}）")))?;
        if existing.status != contract_template_status::DRAFT {
            let code = existing.template_code.clone();
            let version = existing.version;
            let status = existing.status.clone();
            return Err(AppError::business_displayable(format!(
                "模板 {code} v{version} 当前状态为 {status}，启用只能从草稿发起"
            )));
        }
        if existing.approved_by.is_none() {
            let code = existing.template_code.clone();
            let version = existing.version;
            return Err(AppError::business_displayable(format!(
                "模板 {code} v{version} 尚未记录法务复核通过，未过复核的模板不能启用"
            )));
        }
        let (contract_type_token, party_side_token) =
            (existing.contract_type.clone(), existing.party_side.clone());
        let competitor = contract_template::Entity::find()
            .filter(contract_template::Column::ContractType.eq(contract_type_token.clone()))
            .filter(contract_template::Column::PartySide.eq(party_side_token.clone()))
            .filter(contract_template::Column::Status.eq(contract_template_status::ACTIVE))
            .lock_exclusive()
            .one(&txn)
            .await?;
        if let Some(rival) = competitor {
            warn!(
                "用户 {user_id} 启用合同模板 id={id} 被拒：类别 {contract_type_token}/立场 {party_side_token} 已有在位版 {} v{}",
                rival.template_code, rival.version
            );
            return Err(AppError::business_displayable(format!(
                "同一合同类别同一立场只允许一版启用，{contract_type_token} 立场 {party_side_token} 当前在位版为 {} v{}，请先停用旧版再启用新版",
                rival.template_code, rival.version
            )));
        }
        let mut active: contract_template::ActiveModel = existing.into();
        active.status = Set(contract_template_status::ACTIVE.to_string());
        active.updated_at = Set(Utc::now());
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 置停用。draft 与 active 可下线；retired 是终态，重复停用与复活一律点名拒绝。
    pub async fn retire(
        &self,
        id: i32,
        user_id: i32,
    ) -> Result<contract_template::Model, AppError> {
        let txn = (*self.db).begin().await?;
        let existing = contract_template::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("合同模板不存在（id {id}）")))?;
        if existing.status == contract_template_status::RETIRED {
            let code = existing.template_code.clone();
            let version = existing.version;
            return Err(AppError::business_displayable(format!(
                "模板 {code} v{version} 已是停用终态，停用不可重复、已停用版本不可复活；恢复使用请改版出新草稿"
            )));
        }
        let mut active: contract_template::ActiveModel = existing.into();
        active.status = Set(contract_template_status::RETIRED.to_string());
        active.updated_at = Set(Utc::now());
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        tracing::info!("用户 {user_id} 停用合同模板 id={id}");
        Ok(updated)
    }

    /// 删除模板。只删 draft；被合同条款快照引用的行一律拒绝并点名引用条数
    /// （库侧外键 RESTRICT 是最后防线，这里给可读的业务拒绝）。
    pub async fn delete(&self, id: i32, user_id: i32) -> Result<(), AppError> {
        let txn = (*self.db).begin().await?;
        let existing = contract_template::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("合同模板不存在（id {id}）")))?;
        if existing.status != contract_template_status::DRAFT {
            let code = existing.template_code.clone();
            let version = existing.version;
            let status = existing.status.clone();
            return Err(AppError::business_displayable(format!(
                "模板 {code} v{version} 当前状态为 {status}，只有未启用的草稿可以删除；已启用过的版本请停用留存，保证已签合同的快照可追溯"
            )));
        }
        let referenced = crate::models::contract_clause::Entity::find()
            .filter(crate::models::contract_clause::Column::TemplateId.eq(id))
            .count(&txn)
            .await?;
        if referenced > 0 {
            let code = existing.template_code.clone();
            return Err(AppError::business_displayable(format!(
                "模板 {code} 已被 {referenced} 条合同条款快照引用，删除会切断已签合同的约定内容追溯，拒绝删除"
            )));
        }
        contract_template_clause::Entity::delete_many()
            .filter(contract_template_clause::Column::TemplateId.eq(id))
            .exec(&txn)
            .await?;
        contract_template::Entity::delete_by_id(id)
            .exec(&txn)
            .await?;
        txn.commit().await?;
        tracing::info!("用户 {user_id} 删除草稿合同模板 id={id}");
        Ok(())
    }

    // -----------------------------------------------------------------------
    // 条款项维护（仅 draft 模板可动）
    // -----------------------------------------------------------------------

    /// 条款项入参的字段级校验：词表、正文与库引用二选一、排序号界内、库引用存在性。
    /// 判据与 m0116 的 CHECK 逐条同界，先点名再落库，不让数据库冒裸错。
    async fn validate_clause_request(
        db: &impl sea_orm::ConnectionTrait,
        req: &UpsertClauseRequest,
    ) -> Result<(), AppError> {
        Self::require_vocab("clause_code", &req.clause_code, contract_clause_code::ALL)?;
        if let Some(v) = req.applies_when.as_deref() {
            Self::require_vocab("applies_when", v, contract_applies_when::ALL)?;
        }
        let title = req.title.trim();
        if title.is_empty() {
            return Err(AppError::validation_displayable("条款标题不能为空"));
        }
        let has_body = req
            .body
            .as_deref()
            .map(|b| !b.trim().is_empty())
            .unwrap_or(false);
        let body_blank_given = req
            .body
            .as_deref()
            .map(|b| b.trim().is_empty())
            .unwrap_or(false);
        if body_blank_given {
            return Err(AppError::validation_displayable(
                "条款正文给了空串：自带正文必须非空，不复用条款库就不要给 body",
            ));
        }
        match (has_body, req.library_clause_id) {
            (true, None) => {}
            (false, Some(_)) => {}
            (true, Some(_)) => {
                return Err(AppError::validation_displayable(
                    "条款正文与条款库引用只能给一个：两处都给等于两个真相源，必然漂移",
                ));
            }
            (false, None) => {
                return Err(AppError::validation_displayable(
                    "条款必须自带正文或引用条款库其一：两者都不给的条款没有内容可固化",
                ));
            }
        }
        if !(SORT_MIN..=SORT_MAX).contains(&req.sort) {
            return Err(AppError::validation_displayable(format!(
                "条款顺序号须在 {SORT_MIN} 到 {SORT_MAX} 之间，实得 {}",
                req.sort
            )));
        }
        if let Some(lid) = req.library_clause_id {
            let lib = clause_library::Entity::find_by_id(lid).one(db).await?;
            let lib = lib.ok_or_else(|| {
                AppError::business_displayable(format!(
                    "引用的条款库行 {lid} 不存在：条款库正文由模板内容域维护，引用必须指向已有条款"
                ))
            })?;
            if lib.clause_code != req.clause_code {
                return Err(AppError::business_displayable(format!(
                    "条款库行 {lid} 的条款码是 {}，与本条的 {} 不一致，禁止张冠李戴",
                    lib.clause_code, req.clause_code
                )));
            }
        }
        Ok(())
    }

    /// 位置唯一预检：同模板同条款码同适用条件（空条件折叠成空串，与库索引谓词同口径）
    /// 已有他行即点名拒绝。exclude 传本行 id 于更新场景。
    async fn require_position_free(
        txn: &sea_orm::DatabaseTransaction,
        template_id: i32,
        clause_code: &str,
        applies_when: Option<&str>,
        exclude_id: Option<i32>,
    ) -> Result<(), AppError> {
        let mut q = contract_template_clause::Entity::find()
            .filter(contract_template_clause::Column::TemplateId.eq(template_id))
            .filter(contract_template_clause::Column::ClauseCode.eq(clause_code));
        q = match applies_when {
            Some(v) => q.filter(contract_template_clause::Column::AppliesWhen.eq(v)),
            None => q.filter(contract_template_clause::Column::AppliesWhen.is_null()),
        };
        for row in q.all(txn).await? {
            if Some(row.id) != exclude_id {
                return Err(AppError::business_displayable(format!(
                    "模板内条款码 {clause_code} 在适用条件 {} 上已有条款行（条款项 {}），同码同条件不允许并行两行",
                    applies_when.unwrap_or("通用"),
                    row.id
                )));
            }
        }
        Ok(())
    }

    /// 事务内插入条款项（create 与 add_clause 共用），复核状态从 pending 起步。
    async fn insert_clause_txn(
        txn: &sea_orm::DatabaseTransaction,
        template_id: i32,
        req: &UpsertClauseRequest,
    ) -> Result<contract_template_clause::Model, AppError> {
        Self::validate_clause_request(txn, req).await?;
        Self::require_position_free(
            txn,
            template_id,
            &req.clause_code,
            req.applies_when.as_deref(),
            None,
        )
        .await?;
        contract_template_clause::ActiveModel {
            template_id: Set(template_id),
            clause_code: Set(req.clause_code.clone()),
            title: Set(req.title.trim().to_string()),
            body: Set(req.body.clone()),
            library_clause_id: Set(req.library_clause_id),
            required: Set(req.required.unwrap_or(false)),
            sort: Set(req.sort),
            applies_when: Set(req.applies_when.clone()),
            legal_review_status: Set(contract_legal_review_status::PENDING.to_string()),
            legal_reviewed_by: Set(None),
            legal_reviewed_at: Set(None),
            legal_review_remark: Set(None),
            ..Default::default()
        }
        .insert(txn)
        .await
        .map_err(Into::into)
    }

    /// 加条款项（仅 draft 模板）。
    pub async fn add_clause(
        &self,
        template_id: i32,
        req: UpsertClauseRequest,
        user_id: i32,
    ) -> Result<contract_template_clause::Model, AppError> {
        let txn = (*self.db).begin().await?;
        Self::require_draft_template(&txn, template_id).await?;
        let created = Self::insert_clause_txn(&txn, template_id, &req).await?;
        txn.commit().await?;
        tracing::info!(
            "用户 {user_id} 为合同模板 {template_id} 新增条款项 {}",
            created.id
        );
        Ok(created)
    }

    /// 改条款项（仅 draft 模板）。文本来源任一键变化即复位复核 pending 并清空复核留痕
    /// （改文必须重审，旧结论不跨文本版本存活）；复核终态的改写只能走 pending 出边。
    pub async fn update_clause(
        &self,
        template_id: i32,
        clause_id: i32,
        req: UpsertClauseRequest,
        user_id: i32,
    ) -> Result<contract_template_clause::Model, AppError> {
        let txn = (*self.db).begin().await?;
        Self::require_draft_template(&txn, template_id).await?;
        let existing = Self::load_clause_of_template(&txn, template_id, clause_id).await?;
        Self::validate_clause_request(&txn, &req).await?;
        Self::require_position_free(
            &txn,
            template_id,
            &req.clause_code,
            req.applies_when.as_deref(),
            Some(clause_id),
        )
        .await?;
        let text_changed = existing.body != req.body
            || existing.library_clause_id != req.library_clause_id
            || existing.title != req.title.trim();
        let mut active: contract_template_clause::ActiveModel = existing.into();
        active.clause_code = Set(req.clause_code.clone());
        active.title = Set(req.title.trim().to_string());
        active.body = Set(req.body.clone());
        active.library_clause_id = Set(req.library_clause_id);
        if let Some(v) = req.required {
            active.required = Set(v);
        }
        active.sort = Set(req.sort);
        active.applies_when = Set(req.applies_when.clone());
        if text_changed {
            active.legal_review_status = Set(contract_legal_review_status::PENDING.to_string());
            active.legal_reviewed_by = Set(None);
            active.legal_reviewed_at = Set(None);
            active.legal_review_remark = Set(None);
        }
        active.updated_at = Set(Utc::now());
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        tracing::info!("用户 {user_id} 更新合同模板 {template_id} 的条款项 {clause_id}");
        Ok(updated)
    }

    /// 删条款项（仅 draft 模板）。
    pub async fn delete_clause(
        &self,
        template_id: i32,
        clause_id: i32,
        user_id: i32,
    ) -> Result<(), AppError> {
        let txn = (*self.db).begin().await?;
        Self::require_draft_template(&txn, template_id).await?;
        Self::load_clause_of_template(&txn, template_id, clause_id).await?;
        contract_template_clause::Entity::delete_by_id(clause_id)
            .exec(&txn)
            .await?;
        txn.commit().await?;
        tracing::info!("用户 {user_id} 删除合同模板 {template_id} 的条款项 {clause_id}");
        Ok(())
    }

    /// 逐条法务复核：只允许 pending → approved / rejected。非 pending 起点点名拒绝；
    /// rejected 之后要转好，必须先改文复位 pending 再重审，不给覆盖终态结论的通道。
    pub async fn review_clause(
        &self,
        template_id: i32,
        clause_id: i32,
        req: ReviewClauseRequest,
        user_id: i32,
    ) -> Result<contract_template_clause::Model, AppError> {
        if req.legal_review_status != contract_legal_review_status::APPROVED
            && req.legal_review_status != contract_legal_review_status::REJECTED
        {
            return Err(AppError::validation_displayable(
                "复核结论只能是 approved 或 rejected，pending 是初始态不是复核出口",
            ));
        }
        let txn = (*self.db).begin().await?;
        Self::require_draft_template(&txn, template_id).await?;
        let existing = Self::load_clause_of_template(&txn, template_id, clause_id).await?;
        if existing.legal_review_status != contract_legal_review_status::PENDING {
            let got = existing.legal_review_status.clone();
            return Err(AppError::business_displayable(format!(
                "条款项 {clause_id} 复核状态已是 {got}，复核结论不可覆盖；需要重审请先修改条款文本使其回到待复核"
            )));
        }
        let now = Utc::now();
        let mut active: contract_template_clause::ActiveModel = existing.into();
        active.legal_review_status = Set(req.legal_review_status);
        active.legal_reviewed_by = Set(Some(user_id));
        active.legal_reviewed_at = Set(Some(now));
        active.legal_review_remark = Set(req.legal_review_remark);
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// 条款项操作前的模板状态门：非 draft 点名拒绝（active 与 retired 行是历史真相，冻结可读卡）。
    async fn require_draft_template(
        txn: &sea_orm::DatabaseTransaction,
        template_id: i32,
    ) -> Result<contract_template::Model, AppError> {
        let tpl = contract_template::Entity::find_by_id(template_id)
            .lock_exclusive()
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("合同模板不存在（id {template_id}）")))?;
        if tpl.status != contract_template_status::DRAFT {
            let code = tpl.template_code.clone();
            let version = tpl.version;
            let status = tpl.status.clone();
            return Err(AppError::business_displayable(format!(
                "模板 {code} v{version} 当前状态为 {status}，条款项只在草稿阶段可维护；已启用或已停用版本的条款是历史真相，不可改写"
            )));
        }
        Ok(tpl)
    }

    /// 取模板下的条款项并校验归属，条款不 belong 该模板即点名（防跨模板越位改行）。
    async fn load_clause_of_template(
        txn: &sea_orm::DatabaseTransaction,
        template_id: i32,
        clause_id: i32,
    ) -> Result<contract_template_clause::Model, AppError> {
        let row = contract_template_clause::Entity::find_by_id(clause_id)
            .one(txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("条款项不存在（id {clause_id}）")))?;
        if row.template_id != template_id {
            return Err(AppError::business_displayable(format!(
                "条款项 {clause_id} 不属于模板 {template_id}，拒绝跨模板操作"
            )));
        }
        Ok(row)
    }
}

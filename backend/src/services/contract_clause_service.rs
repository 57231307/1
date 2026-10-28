//! 签约条款渲染与固化腿服务
//!
//! 功能 三件事。一 渲染 按所选模板（必须处于启用态）的条款项出条款集合，条件出条款标记
//!   只认合同侧可证实的来源（内外销取 contract_market、含税取 tax_inclusive；订做与框架两枚
//!   条件在合同侧没有采集来源，命中必填条款时拒绝落库并点名，非必填则不出条款），
//!   条款库引用行取固化当时的库文，库行缺席或已停用点名拒绝。二 占位符 填充值必须逐键给齐，
//!   缺任一键拒绝落库并点名条款码与缺键；空串值按未填处理；给值未用到也点名（防值表与模板错位）；
//!   成品文本残留 `{{` 或 `}}` 一律拒绝，库侧 CHECK `chk_cc_body_fully_rendered` 是最后防线。
//!   三 固化 把成品文本与当时的标题、条款码、顺序、来源条款项行指针、模板主键与模板版本号
//!   写进 `contract_clauses` 快照行；模板此后任何改版与停用都不回读、不影响已固化行。
//!   读侧两出口 合同详情腿的条款数组（顺序 标题 正文 必填标记 固化时模板版本，必填标记按
//!   来源指针单义回查，历史空指针行走 (模板,条款码) 唯一行回落，多义如实给空值不伪造），
//!   与打印腿的条款摘要文本（快照为空给“模板前签约，条款未固化”，不回填伪造内容）。
//! 调用方 `services/sales_contract_service.rs` 与 `services/purchase_contract_service.rs`
//!   的创建腿（事务内调用，与合同行同事务同生死）、合同详情 handler、`services/print_service.rs`
//!   的合同打印数据装配。
//! 入参 事务或库连接、合同侧别（取 `contract_side_kind` 词表）、合同主键、模板主键、
//!   内外销与含税两枚上下文、占位符填充值表。
//! 传给谁 写 `contract_clauses`；读 `contract_templates`、`contract_template_clauses`、
//!   `clause_library`。
//! 存什么 见功能段。存哪里 PostgreSQL `contract_clauses`。
//!
//! 状态口径 只有启用态模板可被选用；草稿模板未过 ACTIVE 门，停用模板不再对新签生效，
//!   两者都在服务层点名拒绝，不透传到数据库约束冒裸错。

use crate::models::clause_library;
use crate::models::contract_clause;
use crate::models::contract_template;
use crate::models::contract_template_clause;
use crate::models::status::contract_template::{
    contract_applies_when, contract_market as market_vocab, contract_side_kind,
    contract_template_status,
};
use crate::utils::error::AppError;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, Order,
    QueryFilter, QueryOrder, Set,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// 占位符起始与结束标记（与条款正文及库侧 CHECK 的判据字符一致）。
const PH_OPEN: &str = "{{";
const PH_CLOSE: &str = "}}";

/// 提取正文里的全部占位符键（按出现顺序去重）。
/// 形态失配（`{{` 未闭合、`}}` 无开头、键为空或键内再嵌套 `{{`）不静默跳过，
/// 返回 Err 点名，渲染腿据此拒绝，不把半成品形态带进判定。
pub fn extract_placeholder_keys(body: &str) -> Result<Vec<String>, String> {
    let mut keys: Vec<String> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut rest = body;
    while let Some(open) = rest.find(PH_OPEN) {
        if open > 0 && rest[..open].contains(PH_CLOSE) {
            return Err(format!(
                "正文存在未配对的占位符结束标记（位置 {open} 前出现 }} 配对段）"
            ));
        }
        let after = &rest[open + PH_OPEN.len()..];
        let Some(close) = after.find(PH_CLOSE) else {
            return Err("正文存在未闭合的占位符起始标记".to_string());
        };
        let key = after[..close].trim().to_string();
        if key.is_empty() || key.contains(PH_OPEN) || key.contains(PH_CLOSE) {
            return Err(format!(
                "占位符键不合法（实得 \"{key}\"，键不得为空或嵌套）"
            ));
        }
        if seen.insert(key.clone()) {
            keys.push(key);
        }
        rest = &after[close + PH_CLOSE.len()..];
    }
    if rest.contains(PH_CLOSE) {
        return Err("正文存在未配对的占位符结束标记".to_string());
    }
    Ok(keys)
}

/// 用值表填充正文。缺键与空串值都算未填，返回缺键清单交由调用方点名拒绝；
/// 填充后再次扫描成品，仍残留占位符形态的（值本身带 `{{`）同样拒绝，
/// 保证进入快照的文本是完整成品。
pub fn fill_placeholders(
    body: &str,
    values: &HashMap<String, String>,
) -> Result<String, Vec<String>> {
    let keys = match extract_placeholder_keys(body) {
        Ok(k) => k,
        Err(_) => return Err(vec!["<占位符形态不合法>".to_string()]),
    };
    let mut missing: Vec<String> = Vec::new();
    let mut out = body.to_string();
    for key in &keys {
        match values.get(key) {
            Some(v) if !v.trim().is_empty() => {
                out = out.replace(&format!("{PH_OPEN}{key}{PH_CLOSE}"), v.trim());
            }
            _ => missing.push(key.clone()),
        }
    }
    if !missing.is_empty() {
        return Err(missing);
    }
    if out.contains(PH_OPEN) || out.contains(PH_CLOSE) {
        return Err(vec!["<填充值引入残留占位符形态>".to_string()]);
    }
    Ok(out)
}

/// 条件出条款判定（纯函数，取值域与 contract_applies_when 词表一致）。
/// 返回 该变体是否应出现在本合同。订做与框架两枚条件合同侧无采集来源，返回 false，
/// 是否因此拒绝由调用方按必填与否分流。词表外 token 返回 Err（旁路写库脏值点名，不静默）。
pub fn variant_matches(
    applies_when: Option<&str>,
    contract_market: Option<&str>,
    tax_inclusive: Option<bool>,
) -> Result<bool, String> {
    let Some(token) = applies_when else {
        return Ok(true);
    };
    if !contract_applies_when::is_valid(token) {
        return Err(format!(
            "条款适用条件 {token} 不在词表内，模板行被旁路写入脏值，拒绝渲染"
        ));
    }
    Ok(match token {
        contract_applies_when::EXPORT => contract_market == Some(market_vocab::EXPORT),
        contract_applies_when::DOMESTIC => contract_market == Some(market_vocab::DOMESTIC),
        contract_applies_when::TAX_INCLUSIVE => tax_inclusive == Some(true),
        contract_applies_when::TAX_EXCLUSIVE => tax_inclusive == Some(false),
        // 订做与框架在合同侧没有采集来源（销售与采购主表均无对应列），
        // 不可判定的条件不出变体；必填时由调用方点名拒绝。
        contract_applies_when::CUSTOM_MADE | contract_applies_when::FRAMEWORK => false,
        _ => false,
    })
}

/// 一条待固化的条款成品。字段即快照行内容。
#[derive(Debug, Clone)]
pub struct PlannedClause {
    pub template_clause_id: i32,
    pub clause_code: String,
    pub title: String,
    pub body_rendered: String,
    pub sort: i16,
}

/// 渲染计划 模板行加固化行集合。plan 为空即模板没有可用条款，调用方拒绝。
#[derive(Debug, Clone)]
pub struct RenderPlan {
    pub template: contract_template::Model,
    pub rows: Vec<PlannedClause>,
}

/// 按模板出渲染计划。三道点名门 模板必须启用态；同一条款码命中多个适用变体即模板配置
/// 歧义，拒绝并点名；占位符缺值逐条款点名缺键。used_keys 收集模板实际用到的占位符键，
/// 供调用方反查值表里多余的键。
pub async fn plan_clauses_for_signing<C: ConnectionTrait + Sync>(
    conn: &C,
    template_id: i32,
    contract_market: Option<&str>,
    tax_inclusive: Option<bool>,
    values: &HashMap<String, String>,
) -> Result<RenderPlan, AppError> {
    let template = contract_template::Entity::find_by_id(template_id)
        .one(conn)
        .await?
        .ok_or_else(|| AppError::not_found(format!("合同模板不存在（id {template_id}）")))?;
    if template.status != contract_template_status::ACTIVE {
        return Err(AppError::business_displayable(format!(
            "模板 {} v{} 当前状态为 {}，只有启用中的模板可以被选用签约",
            template.template_code, template.version, template.status
        )));
    }

    let clauses = contract_template_clause::Entity::find()
        .filter(contract_template_clause::Column::TemplateId.eq(template_id))
        .order_by(contract_template_clause::Column::Sort, Order::Asc)
        .all(conn)
        .await?;
    if clauses.is_empty() {
        return Err(AppError::business_displayable(format!(
            "模板 {} v{} 还没有条款项，签约没有可固化的约定内容，请先在模板内落条款",
            template.template_code, template.version
        )));
    }

    let mut selected: Vec<&contract_template_clause::Model> = Vec::new();
    let mut by_code: BTreeMap<String, Vec<i32>> = BTreeMap::new();
    for c in &clauses {
        let matched = variant_matches(c.applies_when.as_deref(), contract_market, tax_inclusive)
            .map_err(AppError::business_displayable)?;
        if matched {
            by_code.entry(c.clause_code.clone()).or_default().push(c.id);
            selected.push(c);
        } else if c.required {
            return Err(AppError::business_displayable(format!(
                "模板 {} 的必填条款 {}（条款项 {}）要求适用条件 {}，而本合同侧没有采集该条件的来源，\
                 无法判定必填条款是否生效，拒绝签约",
                template.template_code,
                c.clause_code,
                c.id,
                c.applies_when.as_deref().unwrap_or("未填")
            )));
        }
    }
    for (code, ids) in &by_code {
        if ids.len() > 1 {
            return Err(AppError::business_displayable(format!(
                "模板 {} 的条款码 {code} 在当前合同条件下命中 {} 个适用变体（条款项 {ids:?}），\
                 同一条款码只允许出一条，模板配置歧义，拒绝签约",
                template.template_code,
                ids.len()
            )));
        }
    }

    let mut rows: Vec<PlannedClause> = Vec::with_capacity(selected.len());
    let mut used_keys: BTreeSet<String> = BTreeSet::new();
    for c in selected {
        let source_body = match (c.body.as_deref(), c.library_clause_id) {
            (Some(b), None) => b.to_string(),
            (None, Some(lid)) => {
                let lib = clause_library::Entity::find_by_id(lid)
                    .one(conn)
                    .await?
                    .ok_or_else(|| {
                        AppError::business_displayable(format!(
                            "条款项 {} 引用的条款库行 {lid} 不存在，引用悬空，拒绝签约",
                            c.id
                        ))
                    })?;
                if lib.status != contract_template_status::ACTIVE {
                    return Err(AppError::business_displayable(format!(
                        "条款项 {} 引用的条款库行 {lid} 状态为 {}，未启用的库文不再对新签生效，拒绝签约",
                        c.id, lib.status
                    )));
                }
                lib.body
            }
            _ => {
                return Err(AppError::business_displayable(format!(
                    "条款项 {} 的正文来源形态非法（自带正文与库引用必须恰好一个非空），模板行被旁路写坏，拒绝签约",
                    c.id
                )));
            }
        };
        let keys = extract_placeholder_keys(&source_body).map_err(|why| {
            AppError::business_displayable(format!(
                "条款项 {}（{}）正文占位符形态不合法：{why}",
                c.id, c.clause_code
            ))
        })?;
        used_keys.extend(keys.iter().cloned());
        let body_rendered = fill_placeholders(&source_body, values).map_err(|missing| {
            AppError::business_displayable(format!(
                "条款 {}（条款项 {}）的占位符未填满，缺少取值：{}",
                c.clause_code,
                c.id,
                missing.join("、")
            ))
        })?;
        rows.push(PlannedClause {
            template_clause_id: c.id,
            clause_code: c.clause_code.clone(),
            title: c.title.clone(),
            body_rendered,
            sort: c.sort,
        });
    }
    let unused: Vec<&String> = values.keys().filter(|k| !used_keys.contains(*k)).collect();
    if !unused.is_empty() {
        return Err(AppError::business_displayable(format!(
            "填充值表里的键 {} 在该模板的条款正文里没有出现，值表与模板可能错位，拒绝签约（请核对模板编码与版本）",
            unused
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("、")
        )));
    }
    Ok(RenderPlan { template, rows })
}

/// 落库固化 逐行写快照，版本与来源指针随写。合同创建腿在**同一事务**内调用本函数，
/// 渲染拒绝即回滚，不留半成品合同。返回固化行数。
pub async fn pin_plan_txn(
    txn: &DatabaseTransaction,
    contract_kind: &str,
    contract_id: i32,
    plan: &RenderPlan,
) -> Result<u32, AppError> {
    if !contract_side_kind::is_valid(contract_kind) {
        return Err(AppError::business(format!(
            "合同侧别 {contract_kind} 不在词表内"
        )));
    }
    for row in &plan.rows {
        contract_clause::ActiveModel {
            contract_kind: Set(contract_kind.to_string()),
            contract_id: Set(contract_id),
            clause_code: Set(row.clause_code.clone()),
            title: Set(row.title.clone()),
            body_rendered: Set(row.body_rendered.clone()),
            template_id: Set(plan.template.id),
            template_version: Set(plan.template.version),
            sort: Set(row.sort),
            template_clause_id: Set(Some(row.template_clause_id)),
            ..Default::default()
        }
        .insert(txn)
        .await?;
    }
    Ok(plan.rows.len() as u32)
}

/// 合同创建腿的事务内单点入口 渲染加固化。template_id 为空即未选模板签约，
/// 合法路径（条款快照为空，读侧按“模板前签约”口径返回），调用方跳过本函数即可。
pub async fn render_and_pin_for_new_contract(
    txn: &DatabaseTransaction,
    contract_kind: &str,
    contract_id: i32,
    template_id: i32,
    contract_market: Option<&str>,
    tax_inclusive: Option<bool>,
    values: &HashMap<String, String>,
) -> Result<u32, AppError> {
    let plan =
        plan_clauses_for_signing(txn, template_id, contract_market, tax_inclusive, values).await?;
    pin_plan_txn(txn, contract_kind, contract_id, &plan).await
}

/// 合同详情腿的条款数组出参。键 顺序 条款码 标题 固化正文 必填标记 来源模板 模板版本
/// 来源指针。必填标记解析 优先按 `template_clause_id` 单义回查条款项；
/// 指针为空的历史快照行按 (模板, 条款码) 回查，恰好一行才给值，多义或查不到如实给空值，
/// 绝不猜一个填上。快照为空返回空数组，是否“模板前签约”由出参消费方按空数组判定，
/// 本层不掺文案。
pub async fn list_clauses_for_contract(
    db: &DatabaseConnection,
    contract_kind: &str,
    contract_id: i32,
) -> Result<Vec<serde_json::Value>, AppError> {
    let rows = contract_clause::Entity::find()
        .filter(contract_clause::Column::ContractKind.eq(contract_kind))
        .filter(contract_clause::Column::ContractId.eq(contract_id))
        .order_by(contract_clause::Column::Sort, Order::Asc)
        .all(db)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let required = resolve_required(db, &r).await?;
        out.push(serde_json::json!({
            "sort": r.sort,
            "clause_code": r.clause_code,
            "title": r.title,
            "body_rendered": r.body_rendered,
            "required": required,
            "template_id": r.template_id,
            "template_version": r.template_version,
            "template_clause_id": r.template_clause_id,
        }));
    }
    Ok(out)
}

/// 必填标记单义回查。指针行存在即采信其 required；查不到指针行（模板草稿期被删）给空值。
async fn resolve_required(
    db: &DatabaseConnection,
    snapshot: &contract_clause::Model,
) -> Result<Option<bool>, AppError> {
    if let Some(tc_id) = snapshot.template_clause_id {
        let tc = contract_template_clause::Entity::find_by_id(tc_id)
            .one(db)
            .await?;
        return Ok(tc.map(|row| row.required));
    }
    let candidates = contract_template_clause::Entity::find()
        .filter(contract_template_clause::Column::TemplateId.eq(snapshot.template_id))
        .filter(contract_template_clause::Column::ClauseCode.eq(snapshot.clause_code.as_str()))
        .all(db)
        .await?;
    if candidates.len() == 1 {
        Ok(Some(candidates[0].required))
    } else {
        Ok(None)
    }
}

/// 打印腿的条款摘要文本 按固化顺序把“标题 正文”逐条拼为一块文本。
/// 快照为空时返回“模板前签约，条款未固化”，与历史合同展示口径同源，不回填伪造内容。
pub async fn clauses_summary_for_print(
    db: &DatabaseConnection,
    contract_kind: &str,
    contract_id: i32,
) -> Result<String, AppError> {
    let clauses = list_clauses_for_contract(db, contract_kind, contract_id).await?;
    if clauses.is_empty() {
        return Ok("模板前签约，条款未固化".to_string());
    }
    let mut parts: Vec<String> = Vec::with_capacity(clauses.len());
    for c in clauses {
        parts.push(format!(
            "{} {}",
            c["title"].as_str().unwrap_or_default(),
            c["body_rendered"].as_str().unwrap_or_default()
        ));
    }
    Ok(parts.join("\n"))
}

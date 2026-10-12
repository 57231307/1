//! PII 按需揭示域服务：字段白名单、原文投影与成功留痕的唯一实现。
//!
//! 消费方：`handlers/crm_customer_handler.rs::reveal_customer_pii`
//!（POST /api/v1/erp/crm/customers/{id}/pii/reveal）。
//! 设计边界：
//! - 白名单是"可揭示列"的唯一定义处——新列要放行必须在此登记并同步契约锁
//!   `backend/tests/contract_wave11_pii_reveal_test.rs`，禁止站点内再写私有清单；
//! - 本模块只做"取原文 + 记痕"，不做授权判定：RBAC 键（`customers:reveal`）由
//!   权限中间件按 URL 推导把关，行级 `data_scope` 门由 handler 走
//!   `CustomerService::get_customer(id, Some(&ctx))` 既有唯一判定源完成；
//! - 留痕只记指向与字段 token 集合，**绝不写入任何 PII 原文**（表语义见建表
//!   迁移文件头与 `models/pii_reveal_audit.rs`）。

use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection};

use crate::models::{customer, pii_reveal_audit};
use crate::utils::error::AppError;

/// 本域留痕记录类型取值（`pii_reveal_audit.record_type` 列值域，回读筛选用）
pub const RECORD_TYPE_CUSTOMER: &str = "customer";

/// 解析后的可揭示字段：`(请求 token, 实体列名)` 成对流转，
/// 出参键与留痕集合都取 token（与"揭示字段集合 == 请求白名单"判据逐字对齐），
/// 列名只用于从实体取原文。
pub type RevealedField = (&'static str, &'static str);

/// 客户域可揭示字段白名单：`(请求 token, 实体列名)`。
///
/// 取值依据（models/customer.rs 真实列）：`phone` → `contact_phone`、
/// `email` → `contact_email`、`address` → `address`。客户域没有身份证类真实
/// 载体列，`id_card` 一类 token 在此**不登记**——登记无列的 token 等于对外
/// 承诺一个恒空的揭示面（幽灵键缺陷族），需要时先有列再登记。
pub const PII_REVEAL_WHITELIST: &[RevealedField] = &[
    ("phone", "contact_phone"),
    ("email", "contact_email"),
    ("address", "address"),
];

/// 揭示字段拒绝文案（唯一出处，避免两处措辞漂移；只描述用户自己提交的字段
/// 与公开规则，可外显，不含记录 ID）
fn allowed_fields_hint() -> &'static str {
    "请选择要查看的字段，支持：phone（电话）、email（邮箱）、address（地址）"
}

/// 把请求 token 解析为白名单成对项（保序去重；任一未登记 token 整笔拒绝）。
pub fn resolve_fields(tokens: &[String]) -> Result<Vec<RevealedField>, AppError> {
    if tokens.is_empty() {
        return Err(AppError::validation_displayable(allowed_fields_hint()));
    }
    let mut resolved: Vec<RevealedField> = Vec::with_capacity(tokens.len());
    for raw in tokens {
        let pair = PII_REVEAL_WHITELIST
            .iter()
            .find(|(token, _)| *token == raw.as_str())
            .ok_or_else(|| AppError::validation_displayable(allowed_fields_hint()))?;
        if !resolved.contains(pair) {
            resolved.push(*pair);
        }
    }
    Ok(resolved)
}

/// 按解析后的成对项从**已通过行级门**的客户行投影原文值（列缺值如实为 null，
/// 出参键集合恒等于请求白名单——多给键即判红，见契约锁⑤）。
pub fn project_raw_fields(
    customer_row: &customer::Model,
    fields: &[RevealedField],
) -> serde_json::Map<String, serde_json::Value> {
    let mut out = serde_json::Map::new();
    for (token, column) in fields {
        let value: Option<&str> = match *column {
            "contact_phone" => customer_row.contact_phone.as_deref(),
            "contact_email" => customer_row.contact_email.as_deref(),
            "address" => customer_row.address.as_deref(),
            // resolve_fields 只产出白名单内列名；走到这里属白名单与投影分支
            // 脱钩的编程缺陷，必须显式炸点而非静默吞（fail-fast，禁止兜底）
            other => unreachable!("PII 揭示白名单与投影分支脱钩: {other}"),
        };
        out.insert(
            token.to_string(),
            match value {
                Some(s) => serde_json::Value::String(s.to_string()),
                None => serde_json::Value::Null,
            },
        );
    }
    out
}

/// 写入一条成功揭示留痕（全仓唯一插入点）。
///
/// - `operator_user_id` 必须来自服务端会话（AuthContext.user_id），调用方不得
///   从请求体取身份；
/// - `revealed_fields` 记字段 token 集合（排序去重后落库，回读面稳定），**不记原文**；
/// - 插入失败必须向上传播（AppError::database）：留痕是揭示的强制前置代价，
///   禁止"写痕失败但原文照常返回"的静默放行。
pub async fn record_reveal(
    db: &DatabaseConnection,
    record_type: &str,
    record_id: i32,
    fields: &[RevealedField],
    operator_user_id: i32,
    reason: &str,
) -> Result<(), AppError> {
    let mut revealed_fields: Vec<String> =
        fields.iter().map(|(token, _)| token.to_string()).collect();
    revealed_fields.sort();
    revealed_fields.dedup();

    let row = pii_reveal_audit::ActiveModel {
        record_type: Set(record_type.to_string()),
        record_id: Set(record_id),
        revealed_fields: Set(revealed_fields),
        operator_user_id: Set(operator_user_id),
        reason: Set(reason.to_string()),
        ..Default::default()
    };
    // revealed_at 由 DDL DEFAULT CURRENT_TIMESTAMP 填充，ActiveModel 缺省即取库值
    row.insert(db)
        .await
        .map_err(|e| AppError::database(format!("PII 揭示留痕写入失败: {e}")))?;
    Ok(())
}

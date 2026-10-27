//! 内置打印模板种子落库
//!
//! 把 [`crate::handlers::print_handler::builtin_print_templates`] 描述的内置模板集合以
//! `INSERT ... ON CONFLICT (template_code) DO NOTHING` 幂等写入 `print_templates` 表：仅在对应
//! `template_code` 缺失时插入，**绝不覆盖**已有行（用户编辑/自建模板永不受影响）。由应用启动阶段
//! （迁移完成、开始对外服务之前）调用，确保渲染链路（按 `doc_type` 取「启用且默认」模板）在首个
//! 打印请求前即可命中模板。
//!
//! 字段口径（与既有 `create_print_template` 及渲染查找路径一致，不臆造内容）：
//! - `template_code` 与 `module` 均取 `doc_type`：`template_code` 有列级 UNIQUE，内置集合里
//!   每个 `doc_type` 唯一故编码不冲突；`module` 写自身 `doc_type` 保证种子行归属独立的
//!   内置模块（模板管理页可见），不与任何用户自建模块名相撞。
//! - 默认位不变量按 `doc_type` 互斥（与渲染链路查找键同口径，由 `print_handler` 的
//!   建/改/设默认写入侧维持）：种子行落库时是该 `doc_type` 初始的「启用且默认」行；
//!   用户为同一 `doc_type` 新建默认模板后，写入侧竞位把种子行的默认位降交用户行，
//!   打印随之以用户行为准（租户自定义默认取代内置默认，是预期语义）。
//! - 种子行被降位后若再被停用/启用：重新启用只恢复”可经显式 id 出件”的资格，
//!   不发起竞位、不夺回默认位（要换默认行须走显式设默认）。
//! - `status` 取状态词表 [`print_template_status::ACTIVE`]、`is_default` 置真两条保持不变。
//! - 纸张/方向/四边边距/份数/条码等布局列一律不写，回落 `print_templates` 建表列默认
//!   （A4 / portrait / 15mm / copies 1 / barcode true），布局单一来源仍归渲染链路。
//! - `content` / `description` 取内置集合既有正文描述，不新造模板内容。

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement};
use tracing::{info, warn};

use crate::handlers::print_handler::builtin_print_templates;
use crate::models::status::print::print_template_status;
use crate::utils::error::AppError;

/// 种子创建人快照（内置模板非某个真实用户所建，以系统账号身份落库，展示用）。
const BUILTIN_SEED_CREATED_BY: i32 = 0;
const BUILTIN_SEED_CREATED_BY_NAME: &str = "系统内置";

/// 幂等落库全部内置打印模板：逐条 `ON CONFLICT (template_code) DO NOTHING`，只补不覆盖。
///
/// 单条插入失败（数据库错误）立即上抛 [`AppError`]，由启动阶段判定为致命初始化失败（打印子系统
/// 无模板即不可渲染）；不吞异常、不静默跳过。返回成功写入（实际新插入）的条数供日志留痕。
pub async fn seed_builtin_print_templates(db: &DatabaseConnection) -> Result<(), AppError> {
    let templates = builtin_print_templates();
    let mut inserted = 0usize;

    // 列名与占位符顺序一一对应；status/is_default/created_by/created_by_name 均以绑定值传入，
    // 避免任何业务字面量拼入 SQL 文本。
    let sql = "INSERT INTO print_templates \
               (template_code, template_name, description, module, doc_type, content, status, is_default, created_by, created_by_name) \
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
               ON CONFLICT (template_code) DO NOTHING";

    for t in &templates {
        let values = vec![
            sea_orm::Value::from(t.doc_type.clone()),
            sea_orm::Value::from(t.name.clone()),
            sea_orm::Value::from(t.template_content.clone()),
            sea_orm::Value::from(t.doc_type.clone()),
            sea_orm::Value::from(t.doc_type.clone()),
            sea_orm::Value::from(t.template_content.clone()),
            sea_orm::Value::from(print_template_status::ACTIVE.to_string()),
            sea_orm::Value::from(true),
            sea_orm::Value::from(BUILTIN_SEED_CREATED_BY),
            sea_orm::Value::from(BUILTIN_SEED_CREATED_BY_NAME.to_string()),
        ];
        let stmt = Statement::from_sql_and_values(DbBackend::Postgres, sql, values);
        let result = db.execute_raw(stmt).await.map_err(|e| {
            AppError::database(format!("种子内置打印模板失败（{}）: {e}", t.doc_type))
        })?;
        // rows_affected == 0 表示 template_code 已存在（用户已建/已编辑），按“只补不覆盖”跳过。
        if result.rows_affected() > 0 {
            inserted += 1;
        }
    }

    info!(
        total = templates.len(),
        inserted,
        skipped = templates.len() - inserted,
        "内置打印模板种子完成（幂等：仅补 template_code 缺失者，已存在者不覆盖）"
    );
    if templates.len() != inserted {
        warn!(
            skipped = templates.len() - inserted,
            "部分内置打印模板因 template_code 已存在而跳过插入（保留用户既有行，未覆盖）"
        );
    }
    Ok(())
}

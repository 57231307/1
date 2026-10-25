//! 导出审批令牌生命周期改造（m0105）
//!
//! 功能：为 export_approval_request 表增加 BPM 关联、令牌哈希化与重试窗口所需列，
//! 并把导出审批的流程定义登记到 BPM 流程定义表。
//! 调用方：迁移链，启动时按台账顺序执行。
//! 存什么：
//!   新增列 token_hash/token_fingerprint/token_viewed/retry_count/last_attempt_at/
//!   max_retries/token_consumed/token_consumed_at/bpm_instance_id；
//!   扩展 status CHECK 约束加 consumed/exhausted；
//!   向 bpm_process_definition 插入 export_approval 流程定义行。
//! 存哪里：export_approval_request 表、bpm_process_definition 表。
//!
//! 历史明文令牌处置：不在本迁移中 UPDATE 洗数据。新记录只写哈希，老行保留原列，
//! 校验侧优先走哈希路径、哈希缺失则回退到原明文路径；老行在有效期过后自然失效，
//! 不留长期可利用的明文凭据面。上线前只读摸底见本文件末尾的只读 SQL 注释。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.connection();

        add_columns_for_token_lifecycle(db).await?;
        apply_status_check_constraint(db).await?;
        create_token_hash_index(db).await?;
        seed_export_approval_process_definition(db).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.connection();

        remove_export_approval_process_definition(db).await?;
        drop_token_hash_index(db).await?;
        revert_status_check_constraint(db).await?;
        drop_new_columns(db).await?;

        Ok(())
    }
}

// ===== up 分步实现 =====

/// 为 export_approval_request 新增令牌生命周期所需列（幂等，逐列判断是否存在）
async fn add_columns_for_token_lifecycle(db: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    let blocks: &[(&str, &str)] = &[
        (
            "token_hash",
            r#"ALTER TABLE export_approval_request ADD COLUMN IF NOT EXISTS token_hash VARCHAR(64)"#,
        ),
        (
            "token_fingerprint",
            r#"ALTER TABLE export_approval_request ADD COLUMN IF NOT EXISTS token_fingerprint VARCHAR(16)"#,
        ),
        (
            "token_viewed",
            r#"ALTER TABLE export_approval_request ADD COLUMN IF NOT EXISTS token_viewed BOOLEAN NOT NULL DEFAULT FALSE"#,
        ),
        (
            "retry_count",
            r#"ALTER TABLE export_approval_request ADD COLUMN IF NOT EXISTS retry_count INTEGER NOT NULL DEFAULT 0"#,
        ),
        (
            "last_attempt_at",
            r#"ALTER TABLE export_approval_request ADD COLUMN IF NOT EXISTS last_attempt_at TIMESTAMPTZ"#,
        ),
        (
            "max_retries",
            r#"ALTER TABLE export_approval_request ADD COLUMN IF NOT EXISTS max_retries INTEGER NOT NULL DEFAULT 3"#,
        ),
        (
            "token_consumed",
            r#"ALTER TABLE export_approval_request ADD COLUMN IF NOT EXISTS token_consumed BOOLEAN NOT NULL DEFAULT FALSE"#,
        ),
        (
            "token_consumed_at",
            r#"ALTER TABLE export_approval_request ADD COLUMN IF NOT EXISTS token_consumed_at TIMESTAMPTZ"#,
        ),
        (
            "bpm_instance_id",
            r#"ALTER TABLE export_approval_request ADD COLUMN IF NOT EXISTS bpm_instance_id INTEGER"#,
        ),
    ];

    for (col_name, sql) in blocks {
        let already = column_exists(db, "export_approval_request", col_name).await?;
        if already {
            tracing::info!(column = col_name, "列已存在，跳过加列（幂等路径）");
        } else {
            db.execute(Statement::from_string(
                sea_orm::DbBackend::Postgres,
                sql.to_string(),
            ))
            .await?;
            tracing::info!(column = col_name, "列新增完成");
        }
    }

    // 外键：bpm_instance_id 引用 bpm_process_instance(id)，可空，不级联删除
    let fk_exists = raw_sql_scalar::<i64>(
        db,
        r#"SELECT COUNT(*) FROM information_schema.table_constraints
          WHERE table_name = 'export_approval_request'
            AND constraint_name = 'fk_ear_bpm_instance'"#,
    )
    .await?;
    if fk_exists == 0 {
        db.execute(Statement::from_string(
            sea_orm::DbBackend::Postgres,
            r#"ALTER TABLE export_approval_request
              ADD CONSTRAINT fk_ear_bpm_instance
              FOREIGN KEY (bpm_instance_id) REFERENCES bpm_process_instance(id)
              ON DELETE SET NULL"#
                .to_string(),
        ))
        .await?;
        tracing::info!("外键 fk_ear_bpm_instance 添加完成");
    }

    Ok(())
}

/// 扩展 status CHECK 约束，加入 consumed 与 exhausted 两个新态（幂等）
async fn apply_status_check_constraint(db: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    // 先 drop 旧约束（若存在），再加新约束，同约束名重建是仓库既有范式
    db.execute(Statement::from_string(
        sea_orm::DbBackend::Postgres,
        r#"ALTER TABLE export_approval_request
          DROP CONSTRAINT IF EXISTS chk_ear_status"#
            .to_string(),
    ))
    .await?;
    db.execute(Statement::from_string(
        sea_orm::DbBackend::Postgres,
        r#"ALTER TABLE export_approval_request
          ADD CONSTRAINT chk_ear_status
          CHECK (status IN ('pending', 'pending_l2', 'approved', 'rejected',
                            'expired', 'cancelled', 'consumed', 'exhausted'))"#
            .to_string(),
    ))
    .await?;

    Ok(())
}

/// 按 token_hash 查询的索引（令牌验证高频路径）
async fn create_token_hash_index(db: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute(Statement::from_string(
        sea_orm::DbBackend::Postgres,
        r#"CREATE UNIQUE INDEX IF NOT EXISTS idx_ear_token_hash
          ON export_approval_request (token_hash) WHERE token_hash IS NOT NULL"#
            .to_string(),
    ))
    .await?;

    Ok(())
}

/// 向 bpm_process_definition 插入 export_approval 流程定义（幂等，ON CONFLICT 跳过）
///
/// 节点链形态：start_event → user_task(一级审批) → [条件 risk_level] →
/// 低风险直走 end_event，中/高/严重风险走 user_task(二级审批) → end_event。
/// 审批人锚点取 admin 角色下 ID 最小的启用用户（与 bpm_definition_seed 同源），
/// 查不到时记 warn 并跳过（系统未完成初始化属正常部署时序）。
async fn seed_export_approval_process_definition(
    db: &sea_orm::DatabaseConnection,
) -> Result<(), DbErr> {
    let existing = raw_sql_scalar::<i64>(
        db,
        "SELECT COUNT(*) FROM bpm_process_definition WHERE code = 'export_approval'",
    )
    .await?;
    if existing > 0 {
        tracing::info!("export_approval 流程定义已存在，跳过登记（幂等路径）");
        return Ok(());
    }

    // 解析审批人锚点：admin 角色下 ID 最小的启用用户
    let approver_id = resolve_admin_anchor_user(db).await?;
    let Some(uid) = approver_id else {
        tracing::warn!(
            "export_approval 流程定义种子跳过：admin 角色或其启用用户尚不存在（系统未完成初始化）"
        );
        return Ok(());
    };

    let config = build_export_approval_flow_config(uid);

    let sql = r#"INSERT INTO bpm_process_definition
      (name, code, description, category, version, config, status, created_at, updated_at)
      VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW())
      ON CONFLICT (code) DO NOTHING"#;

    db.execute(Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        sql,
        [
            "导出审批流程".into(),
            "export_approval".into(),
            "敏感数据导出审批的 BPM 流程链路，按风险等级走一级或二级审批".into(),
            "system".into(),
            "v1".into(),
            sea_orm::Value::Json(Some(Box::new(config))),
            "ACTIVE".into(),
        ],
    ))
    .await?;

    tracing::info!(assignee_user_id = uid, "export_approval 流程定义登记完成");
    Ok(())
}

// ===== down 分步实现 =====

async fn remove_export_approval_process_definition(
    db: &sea_orm::DatabaseConnection,
) -> Result<(), DbErr> {
    db.execute(Statement::from_string(
        sea_orm::DbBackend::Postgres,
        "DELETE FROM bpm_process_definition WHERE code = 'export_approval'".to_string(),
    ))
    .await?;
    Ok(())
}

async fn drop_token_hash_index(db: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute(Statement::from_string(
        sea_orm::DbBackend::Postgres,
        "DROP INDEX IF EXISTS idx_ear_token_hash".to_string(),
    ))
    .await?;
    Ok(())
}

async fn revert_status_check_constraint(db: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    db.execute(Statement::from_string(
        sea_orm::DbBackend::Postgres,
        r#"ALTER TABLE export_approval_request
          DROP CONSTRAINT IF EXISTS chk_ear_status"#
            .to_string(),
    ))
    .await?;
    db.execute(Statement::from_string(
        sea_orm::DbBackend::Postgres,
        r#"ALTER TABLE export_approval_request
          ADD CONSTRAINT chk_ear_status
          CHECK (status IN ('pending', 'pending_l2', 'approved', 'rejected',
                            'expired', 'cancelled'))"#
            .to_string(),
    ))
    .await?;
    Ok(())
}

async fn drop_new_columns(db: &sea_orm::DatabaseConnection) -> Result<(), DbErr> {
    // 外键先撤再撤列
    db.execute(Statement::from_string(
        sea_orm::DbBackend::Postgres,
        r#"ALTER TABLE export_approval_request
          DROP CONSTRAINT IF EXISTS fk_ear_bpm_instance"#
            .to_string(),
    ))
    .await?;

    let cols = [
        "bpm_instance_id",
        "token_consumed_at",
        "token_consumed",
        "max_retries",
        "last_attempt_at",
        "retry_count",
        "token_viewed",
        "token_fingerprint",
        "token_hash",
    ];
    for col in &cols {
        db.execute(Statement::from_string(
            sea_orm::DbBackend::Postgres,
            format!("ALTER TABLE export_approval_request DROP COLUMN IF EXISTS {col}"),
        ))
        .await?;
    }
    Ok(())
}

// ===== 辅助函数 =====

/// 查 information_schema 判断列是否已存在（幂等加列前置门）
async fn column_exists(
    db: &sea_orm::DatabaseConnection,
    table: &str,
    column: &str,
) -> Result<bool, DbErr> {
    let sql = r#"SELECT COUNT(*) FROM information_schema.columns
                 WHERE table_name = $1 AND column_name = $2"#;
    let row = db
        .query_one(Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            sql,
            [table.into(), column.into()],
        ))
        .await?;
    Ok(match row {
        Some(r) => {
            let count: i64 = r.try_get_by::<i64, _>(0)?;
            count > 0
        }
        None => false,
    })
}

/// 取单列单值（用于 COUNT(*) 类标量查询）
async fn raw_sql_scalar<T>(db: &sea_orm::DatabaseConnection, sql: &str) -> Result<T, DbErr>
where
    T: sea_orm::TryGetResult<sea_orm::SqlxResult> + sea_orm::sea_query::ValueType,
{
    let row = db
        .query_one(Statement::from_string(
            sea_orm::DbBackend::Postgres,
            sql.to_string(),
        ))
        .await?;
    match row {
        Some(r) => Ok(r.try_get_by::<T, _>(0)?),
        None => Err(DbErr::Query(
            "标量查询无结果行（COUNT(*) 不应返回空集）".to_string(),
        )),
    }
}

/// 解析 admin 角色下 ID 最小的启用用户（与 bpm_definition_seed::resolve_default_approver 同源）
async fn resolve_admin_anchor_user(db: &sea_orm::DatabaseConnection) -> Result<Option<i32>, DbErr> {
    let sql = r#"SELECT u.id FROM users u
                 JOIN roles r ON u.role_id = r.id
                 WHERE r.code = 'admin' AND u.is_active = TRUE
                 ORDER BY u.id ASC LIMIT 1"#;
    let row = db
        .query_one(Statement::from_string(
            sea_orm::DbBackend::Postgres,
            sql.to_string(),
        ))
        .await?;
    Ok(row.and_then(|r| r.try_get_by::<i32, _>(0).ok()))
}

/// 构建 export_approval 流程配置：条件分支按 risk_level 走一级或二级
fn build_export_approval_flow_config(assignee_user_id: i32) -> serde_json::Value {
    serde_json::json!({
        "nodes": [
            { "id": "start", "name": "提交审批", "type": "start_event" },
            {
                "id": "l1_approve",
                "name": "一级审批",
                "type": "user_task",
                "assignee_type": "user",
                "assignee_value": assignee_user_id
            },
            {
                "id": "l2_approve",
                "name": "二级审批",
                "type": "user_task",
                "assignee_type": "user",
                "assignee_value": assignee_user_id
            },
            { "id": "end", "name": "完成", "type": "end_event" }
        ],
        "edges": [
            { "source": "start", "target": "l1_approve" },
            {
                "source": "l1_approve",
                "target": "end",
                "condition": "${risk_level} == 'low'"
            },
            {
                "source": "l1_approve",
                "target": "l2_approve",
                "condition": "${risk_level} != 'low'"
            },
            { "source": "l2_approve", "target": "end" }
        ]
    })
}

// ===== 上线前只读摸底 SQL（不在迁移中执行，供 DBA 手动运行） =====
//
// 当前未消费的明文令牌有多少条、最早过期时间：
//   SELECT COUNT(*), MIN(token_expires_at)
//   FROM export_approval_request
//   WHERE status = 'approved'
//     AND download_token IS NOT NULL
//     AND token_hash IS NULL
//     AND download_count < max_downloads
//     AND token_expires_at > NOW();
//
// 这些老行在 token_expires_at 过后自动失效（verify 侧按有效期拒绝），
// 不需要 UPDATE 洗数据；校验侧对 token_hash IS NULL 的行回退到原明文比对。

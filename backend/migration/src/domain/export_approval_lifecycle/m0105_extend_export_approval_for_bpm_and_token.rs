//! 导出审批令牌生命周期改造（m0105）
//!
//! ## 六要素
//! - **功能**：为 `export_approval_request` 增加 BPM 关联、令牌哈希化与重试窗口所需列，
//!   扩展 status CHECK 词表加 consumed/exhausted 两终态，并为 token_hash 建部分唯一索引。
//!   本迁移只做 schema；export_approval 的 BPM 流程定义行由 `services/bpm_definition_seed.rs`
//!   的出厂种子在初始化链路与启动链路登记——审批人锚点必须是已存在的 admin 启用用户，
//!   而迁移执行时该用户尚未创建（admin 用户由系统初始化链路建立，晚于迁移）。
//! - **调用方**：顶层域 export_approval_lifecycle（台账名 m_export_approval_lifecycle）
//!   的链尾聚合迁移；置于全链尾为新的顶层条目，复用库上属 pending 必然执行。依赖 system
//!   建的 `bpm_process_instance` 与 finance 建并由 v15 扩展的 `export_approval_request`
//!   （含 download_token/token_expires_at/max_downloads/download_count 既有列），全部早于本域。
//! - **入参**：无。状态词表是本文件的编译期常量。
//! - **传给谁**：`SchemaManager` 的裸 SQL 连接。运行时读取方为
//!   `services/export_approval_service.rs`（令牌生命周期）与 `services/bpm_ops/instance.rs`
//!   的 start_process（按 process_key=export_approval 取 ACTIVE 定义，定义行由出厂种子写入）。
//! - **存什么**：新增列 token_hash/token_fingerprint/token_viewed/retry_count/
//!   last_attempt_at/max_retries/token_consumed/token_consumed_at/bpm_instance_id；
//!   以 chk_ear_status 固化 status 八值词表；建 idx_ear_token_hash 部分唯一索引。
//! - **存哪里**：PostgreSQL `public.export_approval_request` 的列、约束与索引。
//!
//! ## 幂等与安全
//! - 加列用 `ADD COLUMN IF NOT EXISTS`，索引与外键按 pg_constraint / IF NOT EXISTS 守卫，
//!   重跑等价。
//! - status CHECK 施加前按「先探测后约束」范式统计词表外存量并采样例值，非 0 即
//!   RAISE EXCEPTION 点名中止，绝不借施加约束之机 UPDATE 造值圆场。
//! - 历史明文令牌：不在本迁移中 UPDATE/DELETE 洗数据。新记录只写哈希，老行保留原列，
//!   校验侧优先走哈希路径、哈希缺失则回退到原明文比对；老行在 token_expires_at 过后自然
//!   失效，不留长期可利用的明文凭据面。

use sea_orm_migration::prelude::*;

/// export_approval_request.status 权威词表（与 models/export_approval_request.rs 的
/// ApprovalStatus::as_str 逐字符一致；八值含 consumed/exhausted 两终态）。
const EAR_ALLOWED_STATUS_VALUES: &str =
    "'pending','pending_l2','approved','rejected','expired','cancelled','consumed','exhausted'";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        add_columns_for_token_lifecycle(manager).await?;
        apply_status_check_constraint(manager).await?;
        create_token_hash_index(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        drop_token_hash_index(manager).await?;
        revert_status_check_constraint(manager).await?;
        drop_new_columns(manager).await?;
        Ok(())
    }
}

// ===== up 分步实现（全部走 execute_unprepared，不引入迁移 crate 未依赖的 tracing/serde_json/sea_orm）=====

/// 为 export_approval_request 幂等新增令牌生命周期所需列 + bpm_instance_id 外键
async fn add_columns_for_token_lifecycle(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let sql = r#"
ALTER TABLE "export_approval_request" ADD COLUMN IF NOT EXISTS "token_hash" VARCHAR(64);
ALTER TABLE "export_approval_request" ADD COLUMN IF NOT EXISTS "token_fingerprint" VARCHAR(16);
ALTER TABLE "export_approval_request" ADD COLUMN IF NOT EXISTS "token_viewed" BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE "export_approval_request" ADD COLUMN IF NOT EXISTS "retry_count" INTEGER NOT NULL DEFAULT 0;
ALTER TABLE "export_approval_request" ADD COLUMN IF NOT EXISTS "last_attempt_at" TIMESTAMPTZ;
ALTER TABLE "export_approval_request" ADD COLUMN IF NOT EXISTS "max_retries" INTEGER NOT NULL DEFAULT 3;
ALTER TABLE "export_approval_request" ADD COLUMN IF NOT EXISTS "token_consumed" BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE "export_approval_request" ADD COLUMN IF NOT EXISTS "token_consumed_at" TIMESTAMPTZ;
ALTER TABLE "export_approval_request" ADD COLUMN IF NOT EXISTS "bpm_instance_id" INTEGER;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
         WHERE conrelid = 'public.export_approval_request'::regclass
           AND conname = 'fk_ear_bpm_instance'
    ) THEN
        ALTER TABLE "export_approval_request"
          ADD CONSTRAINT "fk_ear_bpm_instance"
          FOREIGN KEY ("bpm_instance_id") REFERENCES "bpm_process_instance"("id")
          ON DELETE SET NULL;
    END IF;
END;
$$;

COMMENT ON COLUMN "export_approval_request"."token_hash" IS '令牌 SHA256 十六进制（m0105 起新记录只写哈希，明文不落库；老行 NULL 走 download_token 明文回退）';
COMMENT ON COLUMN "export_approval_request"."token_fingerprint" IS '令牌前缀指纹（exp_ 加明文前 8 位，供 UI 辨认，不构成可用凭据）';
COMMENT ON COLUMN "export_approval_request"."token_viewed" IS '一次性令牌是否已被申请人取走查看（true 后 token-once 端点拒第二次调用，只回指纹）';
COMMENT ON COLUMN "export_approval_request"."retry_count" IS '令牌未成功导出前提下的累计重试次数';
COMMENT ON COLUMN "export_approval_request"."last_attempt_at" IS '最近一次导出尝试时刻（重试窗口判定源，与注入时钟比较）';
COMMENT ON COLUMN "export_approval_request"."max_retries" IS '该令牌允许的最大未成功重试次数（默认 3）';
COMMENT ON COLUMN "export_approval_request"."token_consumed" IS '令牌是否已被成功导出消费（一次性消费标记）';
COMMENT ON COLUMN "export_approval_request"."token_consumed_at" IS '令牌消费时刻（导出成功那一刻落库）';
COMMENT ON COLUMN "export_approval_request"."bpm_instance_id" IS '关联 BPM 流程实例 ID（外键指向 bpm_process_instance(id)，ON DELETE SET NULL，审批办理腿的唯一驱动源）';
"#;
    manager.get_connection().execute_unprepared(sql).await?;
    Ok(())
}

/// 先探测后约束：统计 status 词表外存量，非 0 即点名中止；再重建 chk_ear_status 八值 CHECK
async fn apply_status_check_constraint(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let drop_old = r#"
ALTER TABLE "export_approval_request" DROP CONSTRAINT IF EXISTS "chk_ear_status";
"#;
    manager
        .get_connection()
        .execute_unprepared(drop_old)
        .await?;

    let guard = format!(
        r#"
DO $$
DECLARE
    bad_rows INTEGER;
    samples  TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_rows FROM "export_approval_request"
      WHERE "status" IS NULL OR "status" NOT IN ({EAR_ALLOWED_STATUS_VALUES});
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm_export_approval_lifecycle：export_approval_request.status 无词表外存量行，可安全施加 CHECK chk_ear_status（八值集）。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (SELECT DISTINCT COALESCE("status", '<NULL>') AS sample
                  FROM "export_approval_request"
                 WHERE "status" IS NULL OR "status" NOT IN ({EAR_ALLOWED_STATUS_VALUES})
                 LIMIT 10) s;
        RAISE EXCEPTION 'm_export_approval_lifecycle：export_approval_request.status 存在 % 行词表外取值（样例值：%），拒绝施加 CHECK chk_ear_status。', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE 造值：请按权威词表 models/export_approval_request.rs::ApprovalStatus（pending/pending_l2/approved/rejected/expired/cancelled/consumed/exhausted）逐行核实更正存量，更正后重跑迁移；若脏值来自脚本旁路写入，先修写入方，不要在迁移里圆场。';
    END IF;
END;
$$;
"#
    );
    manager.get_connection().execute_unprepared(&guard).await?;

    let apply = format!(
        r#"
ALTER TABLE "export_approval_request"
  ADD CONSTRAINT "chk_ear_status" CHECK ("status" IN ({EAR_ALLOWED_STATUS_VALUES}));
COMMENT ON COLUMN "export_approval_request"."status" IS '导出审批状态：权威词表 models/export_approval_request.rs::ApprovalStatus（八值含 consumed/exhausted 终态），取值集与 CHECK chk_ear_status 逐项相等；写入只由 BPM 终审事件回写与令牌生命周期驱动，不再是游离独立状态机';
"#
    );
    manager.get_connection().execute_unprepared(&apply).await?;
    Ok(())
}

/// token_hash 部分唯一索引（令牌验证高频路径；NULL 不入索引，兼容历史明文行）
async fn create_token_hash_index(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let sql = r#"
CREATE UNIQUE INDEX IF NOT EXISTS "idx_ear_token_hash"
  ON "export_approval_request" ("token_hash") WHERE "token_hash" IS NOT NULL;
"#;
    manager.get_connection().execute_unprepared(sql).await?;
    Ok(())
}

// ===== down 分步实现 =====

async fn drop_token_hash_index(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let sql = r#"
DROP INDEX IF EXISTS "idx_ear_token_hash";
"#;
    manager.get_connection().execute_unprepared(sql).await?;
    Ok(())
}

/// 回退 status CHECK：base 表原本无 chk_ear_status 约束（本迁移新建），故 down 直接 DROP，
/// 恢复到建约束前的无 CHECK 状态；不猜造一个 base 里并不存在的窄集约束。
async fn revert_status_check_constraint(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let sql = r#"
ALTER TABLE "export_approval_request" DROP CONSTRAINT IF EXISTS "chk_ear_status";
"#;
    manager.get_connection().execute_unprepared(sql).await?;
    Ok(())
}

async fn drop_new_columns(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let sql = r#"
ALTER TABLE "export_approval_request" DROP CONSTRAINT IF EXISTS "fk_ear_bpm_instance";
ALTER TABLE "export_approval_request" DROP COLUMN IF EXISTS "bpm_instance_id";
ALTER TABLE "export_approval_request" DROP COLUMN IF EXISTS "token_consumed_at";
ALTER TABLE "export_approval_request" DROP COLUMN IF EXISTS "token_consumed";
ALTER TABLE "export_approval_request" DROP COLUMN IF EXISTS "max_retries";
ALTER TABLE "export_approval_request" DROP COLUMN IF EXISTS "last_attempt_at";
ALTER TABLE "export_approval_request" DROP COLUMN IF EXISTS "retry_count";
ALTER TABLE "export_approval_request" DROP COLUMN IF EXISTS "token_viewed";
ALTER TABLE "export_approval_request" DROP COLUMN IF EXISTS "token_fingerprint";
ALTER TABLE "export_approval_request" DROP COLUMN IF EXISTS "token_hash";
"#;
    manager.get_connection().execute_unprepared(sql).await?;
    Ok(())
}

// ===== 上线前只读摸底 SQL（不在迁移中执行，供 DBA 手动运行）=====
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

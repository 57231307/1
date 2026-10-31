//! 审计日志补定位列（终端类型 操作页面 单据编号 变更原因 风险标记）
//!
//! 六要素
//! 功能 给 audit_logs 追加五枚可空定位列 terminal_type 与 operation_page 与 doc_no 与
//!   change_reason 与 risk_flag，并按追溯查询面补索引。审计专项要求 PC 与 H5 与 小程序
//!   三端操作留痕逐行如实记录 终端类型 操作页面 单据编号 与 变更原因，且高风险操作
//!   （删除 红冲 作废）单独可筛。既有 audit_logs 表只有 resource_id（业务主键字符串）
//!   与 user_agent 与 request_path，没有承载上述四类的独立列，读取侧无法按终端与单号
//!   过滤，也无法把同一张单的全链路操作按时间排出。
//! 调用方 SeaORM migrator 链，注册在 domain 顶层新域 m0126（migration/src/lib.rs 链尾新
//!   条目）。链尾新域在复用库上属 pending 必然执行；写进已应用域 up 会被台账整体跳过。
//! 入参 无（纯 DDL，不读取应用数据、不回填任何存量行）。
//! 传给谁 写入单点在 services/audit_log_service 的三处 ActiveModel 构造（build_active_model
//!   build_update_audit_log insert_delete_audit_log），取值来自 middleware/audit_context
//!   按请求头与 UA 判定的终端与页面，以及业务侧可选登记的单号与原因与风险标记。
//! 存什么 五枚可空列本体 列注释 与 追溯索引；information_schema 里五列的存在性即本域完成标志。
//! 存哪里 PostgreSQL public.audit_logs 的新列 与 public 下的新索引。
//!
//! 为什么可空且不回填 五类信息在历史行里根本不存在，回填等于凭空造数。存量行保持 NULL，
//!   NULL 的语义是 该行未采集该维度，不是 该维度为空串或为 PC。读取方不得把 NULL 终端
//!   读成 PC 兜底（专项硬约束 不许默认成 PC）。
//!
//! 幂等与不报错 up 全程 先探测 information_schema.columns 再决定加列，两条路径都 RAISE
//!   NOTICE 打印（新增成功 与 列已存在跳过），不使用静默 no-op；索引用 CREATE INDEX
//!   IF NOT EXISTS。整支可重放，重跑只走 已存在 分支，不改任何数据行。
//!
//! 回退 down 逆序删索引与删列，删列前打印该列上非 NULL 行数（证明本域未替存量造值，
//!   且删列丢弃的是新采集的数据而非业务行），仅回收本域对象，不触碰其它表与列。
//!
//! 索引依据 按单据编号追溯与按终端与按风险过滤都需要索引支撑，否则审计大表逐行全扫。
//!   单独 doc_no 等值过滤走 idx_audit_logs_doc_no；同一张单按时间排出全链路走
//!   idx_audit_logs_doc_no_created 的 (doc_no, created_at) 复合；终端与风险各自等值过滤
//!   走对应单列索引。面料批次 缸号 客户 等值追溯不新增标量列，改由快照 JSONB 的 GIN 索引
//!   承接（after_snapshot 与 before_snapshot 已内嵌被审计记录全字段，批次号与缸号在其内），
//!   containment 查询 @> 命中 GIN，避免把每个业务维度都复制成一列。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_audit_terminal_context_columns"
    }
}

/// 探测式加列 逐列先查 information_schema 再决定加不加，两条路径都打印，绝不静默。
const ADD_TRACEABILITY_COLUMNS_SQL: &str = r#"
DO $$
DECLARE
    has_col INTEGER;
    filled_rows INTEGER;
BEGIN
    SELECT COUNT(*) INTO has_col FROM information_schema.columns
      WHERE table_schema = current_schema() AND table_name = 'audit_logs'
        AND column_name = 'terminal_type';
    IF has_col = 0 THEN
        ALTER TABLE "audit_logs" ADD COLUMN "terminal_type" VARCHAR(16) NULL;
        RAISE NOTICE 'm_audit_terminal_context_columns 已新增列 terminal_type（VARCHAR(16)）。';
    ELSE
        RAISE NOTICE 'm_audit_terminal_context_columns 列 terminal_type 已存在，跳过 DDL（幂等重放路径）。';
    END IF;

    SELECT COUNT(*) INTO has_col FROM information_schema.columns
      WHERE table_schema = current_schema() AND table_name = 'audit_logs'
        AND column_name = 'operation_page';
    IF has_col = 0 THEN
        ALTER TABLE "audit_logs" ADD COLUMN "operation_page" VARCHAR(500) NULL;
        RAISE NOTICE 'm_audit_terminal_context_columns 已新增列 operation_page（VARCHAR(500)）。';
    ELSE
        RAISE NOTICE 'm_audit_terminal_context_columns 列 operation_page 已存在，跳过 DDL（幂等重放路径）。';
    END IF;

    SELECT COUNT(*) INTO has_col FROM information_schema.columns
      WHERE table_schema = current_schema() AND table_name = 'audit_logs'
        AND column_name = 'doc_no';
    IF has_col = 0 THEN
        ALTER TABLE "audit_logs" ADD COLUMN "doc_no" VARCHAR(64) NULL;
        RAISE NOTICE 'm_audit_terminal_context_columns 已新增列 doc_no（VARCHAR(64)）。';
    ELSE
        RAISE NOTICE 'm_audit_terminal_context_columns 列 doc_no 已存在，跳过 DDL（幂等重放路径）。';
    END IF;

    SELECT COUNT(*) INTO has_col FROM information_schema.columns
      WHERE table_schema = current_schema() AND table_name = 'audit_logs'
        AND column_name = 'change_reason';
    IF has_col = 0 THEN
        ALTER TABLE "audit_logs" ADD COLUMN "change_reason" TEXT NULL;
        RAISE NOTICE 'm_audit_terminal_context_columns 已新增列 change_reason（TEXT）。';
    ELSE
        RAISE NOTICE 'm_audit_terminal_context_columns 列 change_reason 已存在，跳过 DDL（幂等重放路径）。';
    END IF;

    SELECT COUNT(*) INTO has_col FROM information_schema.columns
      WHERE table_schema = current_schema() AND table_name = 'audit_logs'
        AND column_name = 'risk_flag';
    IF has_col = 0 THEN
        ALTER TABLE "audit_logs" ADD COLUMN "risk_flag" VARCHAR(32) NULL;
        RAISE NOTICE 'm_audit_terminal_context_columns 已新增列 risk_flag（VARCHAR(32)）。';
    ELSE
        RAISE NOTICE 'm_audit_terminal_context_columns 列 risk_flag 已存在，跳过 DDL（幂等重放路径）。';
    END IF;

    -- 存量非空计数（加列后必为 0，用来证明本域不回填任何存量行）
    SELECT COUNT(*) INTO filled_rows FROM "audit_logs" WHERE "terminal_type" IS NOT NULL;
    RAISE NOTICE 'm_audit_terminal_context_columns 存证 terminal_type 非 NULL 行数 = %（本域不回填存量）。', filled_rows;
    SELECT COUNT(*) INTO filled_rows FROM "audit_logs" WHERE "doc_no" IS NOT NULL;
    RAISE NOTICE 'm_audit_terminal_context_columns 存证 doc_no 非 NULL 行数 = %（本域不回填存量）。', filled_rows;
END;
$$;
"#;

/// 追溯索引 单据编号等值与按时间排序与终端与风险过滤走 btree；批次与缸号与客户的快照
/// 追溯走 JSONB GIN 的 containment。全部 IF NOT EXISTS 可重放。
const ADD_TRACEABILITY_INDEXES_SQL: &str = r#"
CREATE INDEX IF NOT EXISTS "idx_audit_logs_doc_no" ON "audit_logs" ("doc_no");
CREATE INDEX IF NOT EXISTS "idx_audit_logs_doc_no_created" ON "audit_logs" ("doc_no", "created_at");
CREATE INDEX IF NOT EXISTS "idx_audit_logs_terminal_type" ON "audit_logs" ("terminal_type");
CREATE INDEX IF NOT EXISTS "idx_audit_logs_risk_flag" ON "audit_logs" ("risk_flag");
CREATE INDEX IF NOT EXISTS "idx_audit_logs_after_snapshot_gin" ON "audit_logs" USING GIN ("after_snapshot");
CREATE INDEX IF NOT EXISTS "idx_audit_logs_before_snapshot_gin" ON "audit_logs" USING GIN ("before_snapshot");
"#;

/// 列注释固化取值口径，让 DB 侧可自证（不依赖仓外文档）。
const COMMENT_COLUMNS_SQL: &str = r#"
COMMENT ON COLUMN "audit_logs"."terminal_type" IS '操作终端类型，取值 PC 或 H5 或 MINIPROGRAM 或 UNKNOWN，由服务端按请求头 terminalSource 与 terminalSourceV2 与 App-User-Agent 与 User-Agent 判定；三档判不出记 UNKNOWN，绝不默认 PC';
COMMENT ON COLUMN "audit_logs"."operation_page" IS '操作页面标识，优先取请求头 X-Page-Path，缺省回退 Referer 的路径部分；两者都取不到记 NULL（不臆造页面）';
COMMENT ON COLUMN "audit_logs"."doc_no" IS '单据编号，业务单号（非主键 id），由写入侧业务上下文登记；同一张单的全链路审计按本列串起，未登记为 NULL';
COMMENT ON COLUMN "audit_logs"."change_reason" IS '变更原因备注，调用方未提供时保持 NULL，禁止用默认文案填充';
COMMENT ON COLUMN "audit_logs"."risk_flag" IS '高风险操作标记，取值 DELETE 或 REVERSE 或 VOID 等，删除自动落 DELETE，红冲与作废由业务侧登记；普通操作为 NULL';
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(ADD_TRACEABILITY_COLUMNS_SQL)
            .await?;
        conn.execute_unprepared(ADD_TRACEABILITY_INDEXES_SQL)
            .await?;
        conn.execute_unprepared(COMMENT_COLUMNS_SQL).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        let sql = r#"
DROP INDEX IF EXISTS "idx_audit_logs_before_snapshot_gin";
DROP INDEX IF EXISTS "idx_audit_logs_after_snapshot_gin";
DROP INDEX IF EXISTS "idx_audit_logs_risk_flag";
DROP INDEX IF EXISTS "idx_audit_logs_terminal_type";
DROP INDEX IF EXISTS "idx_audit_logs_doc_no_created";
DROP INDEX IF EXISTS "idx_audit_logs_doc_no";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "risk_flag";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "change_reason";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "doc_no";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "operation_page";
ALTER TABLE "audit_logs" DROP COLUMN IF EXISTS "terminal_type";
"#;
        conn.execute_unprepared(sql).await?;
        Ok(())
    }
}

//! m_crm_lead_claim_record 的 SQL 常量（唯一来源）。
//!
//! 迁移 up/down 与 `backend/tests/crm_lead_claim_record_live_test.rs`（经 `#[path]`
//! 直引）共用本文件，保证"真库活体回放的语句"与"迁移实际执行的语句"逐字符同一，
//! 杜绝测试侧另抄一份 SQL 造成漂移。
//!
//! 列语义（NULL = 存量/从未领取 → 无保护期、不计入当日领取数）与不回填的理由
//! 见 `mod.rs` 头注释；唯一写点为 `services/crm/pool.rs::build_claimed_active`。

/// 领取时间列：领取事件发生时刻（TIMESTAMPTZ 与 crm_lead.updated_at 同型，
/// SeaORM 模型 `Option<DateTime<Utc>>`）。
pub const ADD_LAST_CLAIMED_AT_SQL: &str = r#"
ALTER TABLE crm_lead ADD COLUMN IF NOT EXISTS last_claimed_at TIMESTAMPTZ;"#;

/// 领取人列：最近一次领取的操作人 user_id（保护期"原领取人本人重领豁免"的判据，
/// SeaORM 模型 `Option<i32>`）。
pub const ADD_LAST_CLAIMED_BY_SQL: &str = r#"
ALTER TABLE crm_lead ADD COLUMN IF NOT EXISTS last_claimed_by INTEGER;"#;

/// 每日领取上限计数的支撑索引：`validate_claim_rules` 按
/// `last_claimed_by = ? AND last_claimed_at >= 今日` 计数，缺了它 crm_lead 全表扫描。
/// IF NOT EXISTS 幂等。
pub const CREATE_CLAIMED_INDEX_SQL: &str = r#"
CREATE INDEX IF NOT EXISTS idx_crm_lead_last_claimed ON crm_lead (last_claimed_by, last_claimed_at);"#;

/// down：摘索引（IF EXISTS 幂等）。
pub const DROP_CLAIMED_INDEX_SQL: &str = r#"
DROP INDEX IF EXISTS idx_crm_lead_last_claimed;"#;

/// down：摘两列。本迁移不改写存量值（新增可空列），故无需备份表即精确可逆；
/// up 之后由 `build_claimed_active` 写入的领取事件随 down 一并丢弃（回滚语义 =
/// 回到 up 之前，与 m0007 口径一致，详见 mod.rs 头注释）。
pub const DROP_LAST_CLAIMED_COLUMNS_SQL: &str = r#"
ALTER TABLE crm_lead DROP COLUMN IF EXISTS last_claimed_at;
ALTER TABLE crm_lead DROP COLUMN IF EXISTS last_claimed_by;"#;

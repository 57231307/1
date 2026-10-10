//! crm_lead 领取事件列：`last_claimed_at` / `last_claimed_by`（+ 计数支撑索引）。
//!
//! ## 要修的缺陷（实证，非推测）
//! - 公海保护期与每日领取上限此前用 `updated_at` 作判据
//!   （`services/crm/pool.rs` 旧实现）。而**回收只改 `lead_status` 也会刷新
//!   `updated_at`**（`handlers/crm_pool_handler.rs::recycle_to_pool` →
//!   `update_lead`），于是"回收 → 立即领取"这条合法业务链被默认 7 天保护期
//!   直接判负（既有 e2e `frontend/e2e/fullflow/27-crm-chain.spec.ts` 27-06
//!   依赖该顺序）；每日计数同理把"今天被动过的存量线索"误计为领取量。
//! - 单条领取路径 `/crm/pool/claim` 与批量路径校验口径统一后（用户 2026-10-02
//!   拍板 ①），两条路径都需要一个**只在领取时写入**的事件列作判据。
//!
//! ## 设计
//! - 唯一写点：`services/crm/pool.rs::build_claimed_active`（两条领取路径共用的
//!   唯一归属实现）在领取时写 `last_claimed_at=now`、`last_claimed_by=user_id`。
//!   本迁移只补列与索引，不回填——领取事件无法从历史数据可靠还原
//!   （`updated_at`/`owner_id` 正是被证明不可靠的判据，据其回填等于把缺陷固化）。
//! - **列可空；存量行 NULL 的语义 = "无保护期、不计入当日领取数"**。理由：
//!   保护期是防他人抢单的业务约束，其成立必须以"确实发生过一次领取"为前提；
//!   无法确证的存量行若按保守猜测注入保护期，会凭空拒绝合法的公海领取
//!   （把不确定转嫁成业务阻断）。该选择只影响**保护期/计数判定**，不影响
//!   归属/越权 403——403 门（`check_resource_owner`）与 `owner_id`/`department_id`
//!   有关，与本列无关，写侧收紧程度零变化（只严不宽的红线不被触碰）。
//! - 幂等：`ADD COLUMN IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS`，可重放。
//! - 精确可回退（对照 `domain/system/m0007_...rs` 的备份表思路）：本迁移不改写
//!   任何存量值（新增可空列，存量行一律 NULL），因此**无需备份表**即可精确回退——
//!   down 直接 DROP 两列与索引即回到 up 前状态，且 `IF EXISTS` 保证重放安全。
//!   如实声明边界：up 之后应用写入的领取事件数据在 down 后随之丢弃（回滚语义
//!   即"回到 up 之前"，与 m0007 的口径一致）。
//!
//! SQL 常量在 `sql.rs`（唯一来源，活体测试经 `#[path]` 直引，防两处漂移）。
//!
//! 部署顺序：依赖 business 域所建 `crm_lead` 表（含 rls_dept 的 department_id
//! 补列，索引语句引用该列所在表即可，不依赖其值）；置于 crm_vocab_check 之前，
//! 遵循"建表/补列类迁移先于 CHECK 词表迁移收尾"的既有约定（lib.rs 头注释）。

use sea_orm_migration::prelude::*;

pub mod sql;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_crm_lead_claim_record"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        conn.execute_unprepared(sql::ADD_LAST_CLAIMED_AT_SQL)
            .await?;
        conn.execute_unprepared(sql::ADD_LAST_CLAIMED_BY_SQL)
            .await?;
        conn.execute_unprepared(sql::CREATE_CLAIMED_INDEX_SQL)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let conn = manager.get_connection();
        // 先摘索引再摘列（PG 下 DROP COLUMN 会连带删除其单列索引，显式 DROP 使
        // 回退语句自描述、且在 sqlite/PG 重放时不因索引残留而报差异）
        conn.execute_unprepared(sql::DROP_CLAIMED_INDEX_SQL).await?;
        conn.execute_unprepared(sql::DROP_LAST_CLAIMED_COLUMNS_SQL)
            .await?;
        Ok(())
    }
}

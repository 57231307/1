//! 收款单备注列迁移
//!
//! 向 ar_collections 补 remark 备注列（TEXT，可空）。收款表单采集的备注此前被写入
//! check_no 支票号列承载，备注与支票号共用一列导致互相覆盖；补列后备注落 remark、
//! 支票号只承载真实支票号，两列各归其位。
//!
//! 不做任何数据回填：历史行 check_no 中混入的备注文本与真实支票号在库内无法区分，
//! 猜测式回填等于制造假数据。新列对全部历史行取值 NULL，语义即"未填写备注"；
//! 存量 check_no 的甄别与更正属业务侧人工核对范围，up 仅以 NOTICE 点名非空行数
//! 供核查，不改写任何数据行。
//!
//! 注册位置：finance 域 up 链尾/down 链首。目标表 ar_collections 由 business 域
//! m0012_add_ap_ar_finance_analysis.rs 建表（check_no VARCHAR(50) 为支票号列，
//! 建表时无备注列），business 域先于 finance 域执行，补列晚于建表，顺序成立。
//!
//! 幂等与回退：up 为 ADD COLUMN IF NOT EXISTS，重跑等价；down 对称 DROP COLUMN
//! IF EXISTS，仅移除本迁移补的列，不触碰其他列与任何数据行。
//!
//! 上线前只读摸底（需活库执行，供业务侧核查存量污染面）：
//! ```text
//! SELECT id, collection_no, check_no
//!   FROM ar_collections
//!  WHERE check_no IS NOT NULL;
//! ```

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"ALTER TABLE "ar_collections" ADD COLUMN IF NOT EXISTS "remark" TEXT;
COMMENT ON COLUMN "ar_collections"."remark" IS '收款备注（可空，与 check_no 支票号分列承载；历史混入 check_no 的备注不回填，甄别归业务侧）';
DO $$
DECLARE
    dirty_rows INTEGER;
BEGIN
    SELECT COUNT(*) INTO dirty_rows FROM "ar_collections" WHERE "check_no" IS NOT NULL;
    IF dirty_rows > 0 THEN
        RAISE NOTICE 'ar_collections 存在 % 行 check_no 非空：其中可能混有历史备注文本，本迁移不回填 remark、不做猜测式甄别，存量更正请业务侧核对后进行。', dirty_rows;
    END IF;
END;
$$;"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"ALTER TABLE "ar_collections" DROP COLUMN IF EXISTS "remark";"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}

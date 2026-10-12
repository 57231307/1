//! 价格状态词表 DB CHECK 约束迁移
//!
//! 将价格域权威词表 `models/status/sales.rs::price_approval` 钉入数据库，
//! 阻止任何绕过服务层入参校验的写入产生词表外脏值（CHECK 与入参校验同一字典，
//! 避免两处维护漂移）：
//! - `sales_prices.status`    VARCHAR(20) NOT NULL：('pending','approved')
//!   销售价目写入方全集只有建单 pending 与审批 approved（services/sales_price_service.rs
//!   建单/审批路径，quality_inspection_service.rs 标准价落库亦只写 approved）；
//!   无任何写入点向本表写 inactive，故不纳入——不把从未被写入的值钉进约束。
//! - `purchase_prices.status` VARCHAR(20) NOT NULL：('pending','approved','inactive')
//!   除建单/审批外，inactive 有真实生产者：采购价目页 PUT 透传 status
//!   （frontend/src/views/purchase-price/composables/usePpProc.ts →
//!   services/purchase_price_service.rs 更新路径直接落库），必须纳入，
//!   否则停用写入会撞约束。
//!
//! 取值集按"各表写入方全集"分钉而非共用一个集合：值域约束的边界必须恰等于
//! 该表合法写入者的能力集合——取并集会让销售侧旁路写出 inactive 合法落库，
//! 取交集会把采购侧真实停用判为非法。两 CHECK 取值集与词表常量逐项相等，
//! 由契约测试 `backend/tests/contract_wave8_price_status_parity_test.rs` 锁定。
//!
//! 两列均 NOT NULL（建表 business/m0009_add_purchase_extensions.rs /
//! m0011_add_sales_and_logistics_extensions.rs 即 NOT NULL DEFAULT 'ACTIVE'），
//! CHECK 直接写 "status" IN (...) 不留 NULL 放行。建表默认值 'ACTIVE' 属词表外
//! 历史值，加约束前按 v15 域同口径幂等重申 SET DEFAULT 'pending' 并回填
//! 'ACTIVE'→'pending'（向权威值修正，非猜测归一）；其余词表外存量不洗数据：
//! DO 守卫按表统计残留行数，非 0 即 RAISE EXCEPTION 点名表名/行数/样例值并中止，
//! 由业务侧核实更正后重跑（口径同 production/m0076_add_inventory_piece_measured_checks.rs
//! 的只读点名守卫）。
//!
//! 幂等：先 DROP CONSTRAINT IF EXISTS 再 ADD，可安全重跑。注册在迁移链尾、
//! 晚于 v15 域（两表建表在 business，默认值与 'ACTIVE' 回填在 v15），
//! 全部建表/回填类迁移先行完成后才施加 CHECK，全新迁移库上两表届时无词表外写入点。
//!
//! 存量库上线前先做只读摸底（守卫按设计中止，摸出脏值交业务定性，不在迁移里猜洗）：
//!
//! ```text
//! SELECT 'sales_prices' AS tbl, COALESCE(status, '<NULL>') AS val, COUNT(*) AS n
//!   FROM sales_prices
//!  WHERE status IS NULL OR status NOT IN ('pending', 'approved')
//!  GROUP BY 2
//! UNION ALL
//! SELECT 'purchase_prices', COALESCE(status, '<NULL>'), COUNT(*)
//!   FROM purchase_prices
//!  WHERE status IS NULL OR status NOT IN ('pending', 'approved', 'inactive')
//!  GROUP BY 2;
//! ```
//!
//! 上式有命中即需人工逐行定性（参照 approved_by / approved_at / effective_date /
//! expiry_date 判断该行应归 pending 还是 approved），更正数据后重跑本迁移；
//! 建议先在结构等价的存量快照库预跑一次，把中止暴露在预发而非生产。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_price_vocab_check"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1) 幂等前置：删旧约束 + 默认值收敛 pending + 回填 'ACTIVE'（词表外历史值
        //    的唯一已知形态）。全部裸 SQL 走 DO 块之外，DO 块只承担存量守卫。
        let setup = r#"
ALTER TABLE "sales_prices"    DROP CONSTRAINT IF EXISTS "chk_sales_price_status";
ALTER TABLE "purchase_prices" DROP CONSTRAINT IF EXISTS "chk_purchase_price_status";
ALTER TABLE "sales_prices"    ALTER COLUMN "status" SET DEFAULT 'pending';
ALTER TABLE "purchase_prices" ALTER COLUMN "status" SET DEFAULT 'pending';
UPDATE "sales_prices"    SET "status" = 'pending' WHERE "status" = 'ACTIVE';
UPDATE "purchase_prices" SET "status" = 'pending' WHERE "status" = 'ACTIVE';
"#;
        manager.get_connection().execute_unprepared(setup).await?;

        // 2) 存量守卫（fail-visible，严禁 UPDATE 造值洗数据）：逐表统计词表外残留
        //    行数并采样例值；非 0 即点名拒绝，本迁移不加约束直接中止。
        let guard = r#"
DO $$
DECLARE
    bad_rows INTEGER;
    samples  TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_rows FROM "sales_prices"
      WHERE "status" IS NULL OR "status" NOT IN ('pending','approved');
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm_price_vocab_check：sales_prices.status 无词表外存量行，可安全施加 CHECK chk_sales_price_status。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (SELECT DISTINCT COALESCE("status", '<NULL>') AS sample
                  FROM "sales_prices"
                 WHERE "status" IS NULL OR "status" NOT IN ('pending','approved')
                 LIMIT 10) s;
        RAISE EXCEPTION 'm_price_vocab_check：sales_prices.status 存在 % 行词表外取值（样例值：%），拒绝施加 CHECK chk_sales_price_status。', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE 造值：请按权威词表 models/status/sales.rs::price_approval（pending/approved）逐行核实更正存量，更正后重跑迁移；若脏值来自脚本旁路写入，先修写入方，不要在迁移里圆场。';
    END IF;

    SELECT COUNT(*) INTO bad_rows FROM "purchase_prices"
      WHERE "status" IS NULL OR "status" NOT IN ('pending','approved','inactive');
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm_price_vocab_check：purchase_prices.status 无词表外存量行，可安全施加 CHECK chk_purchase_price_status。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (SELECT DISTINCT COALESCE("status", '<NULL>') AS sample
                  FROM "purchase_prices"
                 WHERE "status" IS NULL OR "status" NOT IN ('pending','approved','inactive')
                 LIMIT 10) s;
        RAISE EXCEPTION 'm_price_vocab_check：purchase_prices.status 存在 % 行词表外取值（样例值：%），拒绝施加 CHECK chk_purchase_price_status。', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE 造值：请按权威词表 models/status/sales.rs::price_approval（pending/approved/inactive）逐行核实更正存量，更正后重跑迁移；若脏值来自脚本旁路写入，先修写入方，不要在迁移里圆场。';
    END IF;
END;
$$;
"#;
        manager.get_connection().execute_unprepared(guard).await?;

        // 3) 施加 CHECK 并以列注释固化权威词表指向（列 NOT NULL，不设 NULL 放行）。
        let apply = r#"
ALTER TABLE "sales_prices"    ADD CONSTRAINT "chk_sales_price_status"    CHECK ("status" IN ('pending','approved'));
ALTER TABLE "purchase_prices" ADD CONSTRAINT "chk_purchase_price_status" CHECK ("status" IN ('pending','approved','inactive'));
COMMENT ON COLUMN "sales_prices"."status"    IS '销售价目审批态：权威词表 models/status/sales.rs::price_approval，取值集与 CHECK chk_sales_price_status 逐项相等（契约锁 contract_wave8_price_status_parity_test）';
COMMENT ON COLUMN "purchase_prices"."status" IS '采购价目状态（含停用 inactive）：权威词表 models/status/sales.rs::price_approval，取值集与 CHECK chk_purchase_price_status 逐项相等（同一契约锁）';
"#;
        manager.get_connection().execute_unprepared(apply).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚仅移除两条约束，不回退数据：'ACTIVE'→'pending' 的回填与默认值收敛
        // 是向权威词表的规范修正，历史脏值属错误数据、无恢复价值（口径同
        // m_crm_vocab_check 的 down——删约束即回到可写状态，不复活旧越界值）。
        let sql = r#"
ALTER TABLE "sales_prices"    DROP CONSTRAINT IF EXISTS "chk_sales_price_status";
ALTER TABLE "purchase_prices" DROP CONSTRAINT IF EXISTS "chk_purchase_price_status";
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}

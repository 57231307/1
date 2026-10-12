//! 售后工单类型 CHECK 补齐 return_goods（退货）——三端词表同源收口
//!
//! 写入方唯一权威取值集合 `services/custom_order_aftersales_service.rs:145`
//! （create 的类型白名单，UpdateAfterSalesDto 不含 issue_type、无其他写点）为 5 值：
//! complaint / repair / exchange / return_goods / refund（return_goods=退货，
//! 与退款是不同业务：物流收货+库存回库 vs 财务出账）；
//! 前端候选三处同源同为 5 值（`AfterSalesPanel.vue` 创建表单 radio 候选与
//! getIssueTypeLabel known 数组、`api/custom-order.ts::AFTER_SALES_TYPE`、
//! locales zh-CN/en-US `common.afterSales.issueType.*` 键）。
//! 而建表迁移 m0044 的生效 CHECK `chk_aftersales_type` 只含原 4 值
//! （complaint/repair/exchange/refund），缺 `return_goods`。
//! ⇒ `POST /custom-orders/{orderId}/after-sales` 创建"退货"工单必违反该约束，
//! PG 回 SQLSTATE 23514（violates check constraint），经 aftersales_err 的
//! Database 通道冒 DATABASE_ERROR(500) 裸错——用户与操作侧都看不到原因；
//! 问题形态与 m0067 处理的 chk_aftersales_status 缺 accepted/evaluated 同型，
//! 处理范式同 m0067（扩值不更名、fail-visible、down 真回退）。
//!
//! 本迁移重建该 CHECK，取值集合与写入方白名单逐字符对齐（全小写、无派生别名），
//! 使 写入方白名单 = 本 CHECK 取值集 = 前端候选 三端同源。
//! 约束名保持 `chk_aftersales_type` 不变：既有文档引用与错误归因均以该名字判定，
//! 更名会造成跨端引用漂移（与 m0067 同口径）。
//!
//! 存量数据策略（照 m0063/m0064/m0067 先例，fail-visible 拒绝静默洗数据）：
//! 新 5 值集是旧 4 值集的超集，凡曾被建表 CHECK 有效约束的行必然落入新集，up 无损；
//! 仅当历史库约束缺失/漂移（ADD CONSTRAINT ... NOT VALID 之类）才可能存在白名单外
//! 取值，此类行属未知业务类型、不允许借扩集之机洗进约束——up 先只读检测，存在
//! 白名单外取值即 RAISE EXCEPTION 点名（样例最多 10 个），交人工核实真实业务类型
//! 处置后重跑；检测通过则以 RAISE NOTICE 逐值打印存量行数（逐值留痕，供回放核对）。
//!
//! 幂等性：up 为「只读检测 → NOTICE 计数 → DROP CONSTRAINT IF EXISTS → ADD
//! CONSTRAINT」，重跑等价；down 反向恢复 m0044 原 4 值集，并带 return_goods 在途
//! 行的 fail-visible 拒滚检查（本仓纪律：down 不留空实现）。
//!
//! 契约锁：`backend/tests/contract_wave7_aftersales_status_check_parity_test.rs`
//! 的类型族段将 写入方白名单 / 本文件 CHECK 取值集合 / 前端候选（radio + known +
//! AFTER_SALES_TYPE + zh/en i18n 键）三端逐 token 双向断言，防止一边改一边忘。

use sea_orm_migration::prelude::*;

/// 写入方权威白名单（custom_order_aftersales_service.rs create 校验）的 5 个合法取值
/// （= 前端候选集合，逐字符一致，全小写）。
const ALLOWED_TYPE_VALUES: &str = "'complaint', 'repair', 'exchange', 'return_goods', 'refund'";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = format!(
            r#"-- 1) fail-visible 存量检查：after_sales.issue_type 出现写入方白名单外的取值即中止。
--    旧 4 值 CHECK 有效时本检查恒空（5 值集为 4 值集超集）；非空即历史库约束漂移，
--    属未知业务类型，拒绝借扩集静默洗数据（样例最多 10 个，供人工定位处置）。
DO $$
DECLARE
    bad_row_count INTEGER;
    bad_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_row_count
      FROM "after_sales"
     WHERE "issue_type" NOT IN ({ALLOWED_TYPE_VALUES});
    IF bad_row_count > 0 THEN
        SELECT string_agg("issue_type", ', ' ORDER BY "issue_type")
          INTO bad_samples
          FROM (
            SELECT DISTINCT "issue_type"
              FROM "after_sales"
             WHERE "issue_type" NOT IN ({ALLOWED_TYPE_VALUES})
             LIMIT 10
          ) s;
        RAISE EXCEPTION 'after_sales.issue_type 存在 % 行取值位于写入方权威白名单（complaint/repair/exchange/return_goods/refund）之外（样例：%）。本迁移拒绝在带白名单外存量值的列上重建 CHECK，且不做静默归一（禁止 UPDATE 洗数据）：请人工核实上述行的真实业务类型并处置后重跑。',
            bad_row_count, bad_samples;
    END IF;
END
$$;

-- 2) 检测通过：逐值打印存量行数（成功也要有显式日志——判责与回放证据）。
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN
        SELECT v AS issue_type_value,
               (SELECT COUNT(*) FROM "after_sales" a WHERE a."issue_type" = v) AS row_count
          FROM unnest(ARRAY['complaint', 'repair', 'exchange', 'return_goods', 'refund']) v
    LOOP
        RAISE NOTICE 'm0074 up：after_sales.issue_type = % 存量行数 %', r.issue_type_value, r.row_count;
    END LOOP;
END
$$;

-- 3) 重建 CHECK：m0044 的 4 值集 + return_goods（与写入方白名单同源），
--    约束名保持不变（后端引用/错误归因按该名字）。
ALTER TABLE "after_sales" DROP CONSTRAINT IF EXISTS "chk_aftersales_type";
ALTER TABLE "after_sales" ADD CONSTRAINT "chk_aftersales_type" CHECK ("issue_type" IN ({ALLOWED_TYPE_VALUES}));
COMMENT ON COLUMN "after_sales"."issue_type" IS '售后类型：complaint(客诉) / repair(维修) / exchange(换货) / return_goods(退货) / refund(退款)';"#
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚恢复建表迁移 m0044 的原始 4 值约束（不含 return_goods）。
        // fail-visible：return_goods（退货）工单已进入物流收货/库存回库履行链，
        // 把这类行移出允许集会直接让 ADD CONSTRAINT 失败或迫使静默 UPDATE 洗数据，
        // 本迁移拒绝回滚吞掉真实退货单据。人工归一规则：退货业务若须保留，改走
        // exchange（换货）语义并补描述、若须终止则关闭工单后删除单据由人工执行；
        // 处置完 return_goods 行后重跑 down。
        let sql = r#"DO $$
DECLARE
    new_type_row_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO new_type_row_count
      FROM "after_sales"
     WHERE "issue_type" = 'return_goods';
    IF new_type_row_count > 0 THEN
        RAISE EXCEPTION 'after_sales.issue_type 存在 % 行 return_goods（退货类在途工单，物流收货/库存回库链已启动）。回滚将把该值移出 CHECK 允许集，本迁移拒绝回滚吞掉真实退货单据、也拒绝静默 UPDATE 归一：请先人工处置上述行后重跑 down。',
            new_type_row_count;
    END IF;
END
$$;

ALTER TABLE "after_sales" DROP CONSTRAINT IF EXISTS "chk_aftersales_type";
ALTER TABLE "after_sales" ADD CONSTRAINT "chk_aftersales_type" CHECK ("issue_type" IN (
    'complaint', 'repair', 'exchange', 'refund'
));
COMMENT ON COLUMN "after_sales"."issue_type" IS '售后类型：complaint(客诉) / repair(维修) / exchange(换货) / refund(退款)';"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}

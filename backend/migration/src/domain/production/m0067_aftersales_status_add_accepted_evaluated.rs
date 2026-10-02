//! 售后工单状态 CHECK 补齐受理/已评价两态（三端词表同源收口）
//!
//! 写入方权威词表 `models/status/sales.rs::custom_order_ext::AFTERSALES_ALL` 为 7 态
//! （opened/accepted/processing/resolved/evaluated/closed/rejected），服务层唯一写入方
//! `services/custom_order_aftersales_service.rs::AFTERSALES_TRANSITIONS` 状态机节点集与
//! 其逐 token 相等；而建表迁移 m0044 的生效 CHECK `chk_aftersales_status` 只含原 5 态
//! （opened/processing/resolved/closed/rejected），缺 `accepted`/`evaluated`。
//! `PUT /custom-orders/after-sales/{id}` 走 opened→accepted（V15 P1 batch-19 缺陷 23.3.2
//! 受理链）或 resolved→evaluated（评价链）时写 status 必违反该约束，PG 回 SQLSTATE 23514
//! （violates check constraint），对上游冒 DATABASE_ERROR(500)——CI #4669 用例 65-01 判责；
//! `tests/contract_wave3_after_sales_customer_name_test.rs` 首跑注释亦点名此为
//! 「迁移/表结构缺口」，禁止反向改断言或改服务迁就约束。
//!
//! 本迁移重建该 CHECK，取值集合与 `AFTERSALES_ALL` 逐字符对齐（全小写、无派生别名），
//! 使 写入方常量 = 本 CHECK 取值集 = 服务层状态机节点集 三端同源。
//! 约束名保持 `chk_aftersales_status` 不变：既有文档引用与错误归因均以该名字判定，
//! 更名会造成跨端引用漂移。
//!
//! 存量数据策略（照 m0063/m0064 先例，fail-visible 拒绝静默洗数据）：
//! 新 7 值集是旧 5 值集的超集，凡曾被建表 CHECK 有效约束的行必然落入新集，up 无损；
//! 仅当历史库约束缺失/漂移（ADD CONSTRAINT ... NOT VALID 之类）才可能存在词表外取值，
//! 此类行属未知业务态、不允许借扩集之机洗进约束——up 先只读检测，存在词表外取值即
//! RAISE EXCEPTION 中止，交人工核实真实业务状态处置后重跑。
//!
//! 幂等性：up 为「只读检测 → DROP CONSTRAINT IF EXISTS → ADD CONSTRAINT」，重跑等价；
//! down 反向恢复 m0044 原 5 值集，并带 accepted/evaluated 在途行的 fail-visible 拒滚
//! 检查（本仓纪律：down 不留空实现，教训见 rls_dept down 空实现登记）。
//!
//! 契约锁：`backend/tests/contract_wave7_aftersales_status_check_parity_test.rs`
//! 将 AFTERSALES_ALL / AFTERSALES_TRANSITIONS 节点集 / 本文件 CHECK 取值集合三端
//! 逐 token 双向断言，防止一边改一边忘。

use sea_orm_migration::prelude::*;

/// 写入方权威词表 `custom_order_ext::AFTERSALES_ALL` 的 7 个合法取值
/// （= 服务层状态机 `AFTERSALES_TRANSITIONS` 节点集，逐字符一致，全小写）。
const ALLOWED_STATUS_VALUES: &str = "'opened', 'accepted', 'processing', \
     'resolved', 'evaluated', 'closed', 'rejected'";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = format!(
            r#"-- 1) fail-visible 存量检查：after_sales.status 出现新词表外的取值即中止。
--    旧 5 值 CHECK 有效时本检查恒空（7 值集为 5 值集超集）；非空即历史库约束漂移，
--    属未知业务态，拒绝借扩集静默洗数据（样例最多 10 个，供人工定位处置）。
DO $$
DECLARE
    bad_row_count INTEGER;
    bad_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_row_count
      FROM "after_sales"
     WHERE "status" NOT IN ({ALLOWED_STATUS_VALUES});
    IF bad_row_count > 0 THEN
        SELECT string_agg("status", ', ' ORDER BY "status")
          INTO bad_samples
          FROM (
            SELECT DISTINCT "status"
              FROM "after_sales"
             WHERE "status" NOT IN ({ALLOWED_STATUS_VALUES})
             LIMIT 10
          ) s;
        RAISE EXCEPTION 'after_sales.status 存在 % 行取值位于权威词表 AFTERSALES_ALL 之外（样例：%）。本迁移拒绝在带词表外存量值的列上重建 CHECK，且不做静默归一（禁止 UPDATE 洗数据）：请人工核实上述行的真实业务状态并处置后重跑。',
            bad_row_count, bad_samples;
    END IF;
END
$$;

-- 2) 重建 CHECK：m0044 的 5 值集 + accepted/evaluated（与写入方 AFTERSALES_ALL 同源），
--    约束名保持不变（后端文档/错误归因按该名字引用）。
ALTER TABLE "after_sales" DROP CONSTRAINT IF EXISTS "chk_aftersales_status";
ALTER TABLE "after_sales" ADD CONSTRAINT "chk_aftersales_status" CHECK ("status" IN ({ALLOWED_STATUS_VALUES}));
COMMENT ON COLUMN "after_sales"."status" IS '状态：opened(已开启) / accepted(已受理) / processing(处理中) / resolved(已解决) / evaluated(已评价) / closed(已关闭) / rejected(已拒绝)';"#
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚恢复建表迁移 m0044 的原始 5 值约束（不含 accepted/evaluated）。
        // fail-visible：accepted 是 opened 已受理在途态（已写 accepted_at），
        // evaluated 是 resolved 已评价结论态（已写 evaluated_at/evaluation_score/
        // evaluation_comment）；把这类行移出允许集会吞掉真实在途工单，本迁移拒绝
        // 静默 UPDATE 归一。人工归一规则（依据状态机 AFTERSALES_TRANSITIONS）：
        // 前进——推进/关闭到 closed 或 rejected；回退——accepted 清 accepted_at 回
        // opened，evaluated 清评价三列+evaluated_at 回 resolved。处置后重跑 down。
        let sql = r#"DO $$
DECLARE
    new_state_row_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO new_state_row_count
      FROM "after_sales"
     WHERE "status" IN ('accepted', 'evaluated');
    IF new_state_row_count > 0 THEN
        RAISE EXCEPTION 'after_sales.status 存在 % 行 accepted/evaluated（受理/评价链在途工单）。回滚将把这两值移出 CHECK 允许集，本迁移拒绝回滚吞掉在途单据：请先按状态机人工处置（推进到 closed/rejected，或回退 accepted→opened 并清 accepted_at、evaluated→resolved 并清评价三列+evaluated_at）后重跑 down。',
            new_state_row_count;
    END IF;
END
$$;

ALTER TABLE "after_sales" DROP CONSTRAINT IF EXISTS "chk_aftersales_status";
ALTER TABLE "after_sales" ADD CONSTRAINT "chk_aftersales_status" CHECK ("status" IN (
    'opened', 'processing', 'resolved', 'closed', 'rejected'
));
COMMENT ON COLUMN "after_sales"."status" IS '状态：opened(已开) / processing(处理中) / resolved(已解决) / closed(已关闭) / rejected(已拒绝)';"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}

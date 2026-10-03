//! 定制订单状态 CHECK 补齐大额变更挂起态 change_pending
//!
//! `services/custom_order_crud_service.rs::submit_change_request` 在金额变化超阈值
//! 时把 `custom_orders.status` 写入 `change_pending`（`approve_change` 状态门亦按该值
//! 比较），但生效 CHECK（m0061 重建后 = 状态机 `as_str()` 的 10 值集）不含该 token，
//! 真实业务动作（大额变更挂起）落库必违反 `chk_custom_order_status`，对上游冒 500。
//!
//! 收口方向（按五方对齐证据裁定）：`change_pending` 与 `lab_dip`/`quotation` 同为
//! 真实可达业务态，扩词表 + 补 CHECK，不砍功能。本迁移把取值集合扩为
//! 权威模块 `models/status/sales.rs::custom_order::ALL` 的 11 值（逐字符一致），
//! 存量数据做 fail-visible 检查：存在词表外取值即 RAISE EXCEPTION 中止迁移，
//! 绝不 `UPDATE ... SET status='draft'` 静默洗数据（照 m0063 重复检测先例）。
//!
//! 幂等性：up 为「只读检测 → DROP CONSTRAINT IF EXISTS → ADD CONSTRAINT」，
//! 重跑等价（约束重建后取值集不变）；down 反向恢复 m0061 的 10 值集，且带
//! 存量 change_pending 行的 fail-visible 拒滚检查——有挂起在途单据时拒绝回滚，
//! 由人工先处置（审批通过/驳回使其离开 change_pending）再重跑 down。
//!
//! 契约锁：`backend/tests/contract_wave5_custom_order_status_unity_test.rs` 将本文件
//! CHECK 取值集合与 `custom_order::ALL` 逐项双向断言，防止一边改一边忘。

use sea_orm_migration::prelude::*;

/// 权威模块 `models/status/sales.rs::custom_order::ALL` 的 11 个合法取值
/// （状态机 10 态 + 变更挂起态，全小写下划线，逐字符一致；不含任何派生别名）。
const ALLOWED_STATUS_VALUES: &str = "'draft', 'lab_dip', 'quotation', 'yarn_purchasing', \
     'dyeing', 'finishing', 'delivery', 'after_sales', 'change_pending', 'completed', 'cancelled'";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = format!(
            r#"-- 1) fail-visible 存量检查：custom_orders.status 出现新词表外的取值即中止，
--    拒绝静默洗数据（样例最多 10 个，消息含违规行数与样例值，供人工定位处置）。
DO $$
DECLARE
    bad_row_count INTEGER;
    bad_samples TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_row_count
      FROM "custom_orders"
     WHERE "status" NOT IN ({ALLOWED_STATUS_VALUES});
    IF bad_row_count > 0 THEN
        SELECT string_agg("status", ', ' ORDER BY "status")
          INTO bad_samples
          FROM (
            SELECT DISTINCT "status"
              FROM "custom_orders"
             WHERE "status" NOT IN ({ALLOWED_STATUS_VALUES})
             LIMIT 10
          ) s;
        RAISE EXCEPTION 'custom_orders.status 存在 % 行取值位于新词表之外（样例：%）。本迁移拒绝在带词表外存量值的列上重建 CHECK，且不做静默归一（禁止 UPDATE 洗数据）：请人工核实上述行的真实业务状态并处置后重跑。',
            bad_row_count, bad_samples;
    END IF;
END
$$;

-- 2) 重建 CHECK：m0061 的 10 值集 + change_pending（写入方 = submit_change_request）
ALTER TABLE "custom_orders" DROP CONSTRAINT IF EXISTS "chk_custom_order_status";
ALTER TABLE "custom_orders" ADD CONSTRAINT "chk_custom_order_status" CHECK ("status" IN ({ALLOWED_STATUS_VALUES}));
COMMENT ON COLUMN "custom_orders"."status" IS '订单状态：draft(草稿) / lab_dip(打样中) / quotation(报价中) / yarn_purchasing(纱线采购) / dyeing(染整) / finishing(后整理) / delivery(交付) / after_sales(售后) / change_pending(变更待审批) / completed(已完成) / cancelled(已取消)';"#
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚恢复 m0061 的 10 值约束（不含 change_pending）。
        // fail-visible：存在挂起在途行时 ADD CONSTRAINT 会真库校验失败并冒 500 级
        // DbErr，先显式检测并给出可操作的报错文案，避免裸约束错误难以定位。
        let sql = r#"DO $$
DECLARE
    pending_row_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO pending_row_count
      FROM "custom_orders"
     WHERE "status" = 'change_pending';
    IF pending_row_count > 0 THEN
        RAISE EXCEPTION 'custom_orders.status 存在 % 行 change_pending（变更审批在途）。回滚将把该值移出 CHECK 允许集，本迁移拒绝回滚吞掉在途单据：请先经 approve_change 处置（通过/驳回）使其离开 change_pending 后重跑 down。',
            pending_row_count;
    END IF;
END
$$;

ALTER TABLE "custom_orders" DROP CONSTRAINT IF EXISTS "chk_custom_order_status";
ALTER TABLE "custom_orders" ADD CONSTRAINT "chk_custom_order_status" CHECK ("status" IN (
    'draft', 'lab_dip', 'quotation', 'yarn_purchasing', 'dyeing', 'finishing',
    'delivery', 'after_sales', 'completed', 'cancelled'
));
COMMENT ON COLUMN "custom_orders"."status" IS '订单状态：draft(草稿) / lab_dip(打样中) / quotation(报价中) / yarn_purchasing(纱线采购) / dyeing(染整) / finishing(后整理) / delivery(交付) / after_sales(售后) / completed(已完成) / cancelled(已取消)';"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}

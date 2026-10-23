//! customers 两列可空无默认 → NOT NULL + 成文缺省值归一
//! （"迁移加列可空、模型非 Option"族的存量库通道）
//!
//! ## 根因（全部 file:line 实测于本分支）
//! - `domain/system/mod.rs:243` = `ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS
//!   "payment_terms" INTEGER;`（可空、无默认）；
//! - `domain/system/mod.rs:250` = `ALTER TABLE "customers" ADD COLUMN IF NOT EXISTS
//!   "status" VARCHAR(255);`（可空、无默认）；
//!   （`domain/finance/mod.rs:547/554` 的同名 `ADD COLUMN IF NOT EXISTS` 因列已存在
//!   被 `IF NOT EXISTS` 吃成恒 no-op，域执行序 system→…→finance，见 `lib.rs` 顺序说明。）
//! - 模型 `backend/src/models/customer.rs` 中 `payment_terms: i32`、`status: String`
//!   均为非 Option ⇒ 任一行为 NULL 时整行解码报
//!   `Missing value for column 'payment_terms' / 'status'`，该行在列表/详情/导出全不可见。
//! - 同型收敛惯例已在册：`domain/system/mod.rs:256-261` 对 inventory_count_items.batch_no、
//!   `domain/system/m0070_normalize_customers_owner_id.rs` 对 customers.owner_id，
//!   均按「UPDATE 清 NULL → SET DEFAULT → SET NOT NULL」归一，绝不修改已应用迁移文本。
//!
//! ## 缺省值词表证据（不是发明值）
//! status 合法值域（写入方全集）与创建缺省：
//! - `services/customer_ops/update.rs:104-108`：创建通道 `args.status` 为空时
//!   `unwrap_or_else(|| master_data::ACTIVE.to_string())` ⇒ 缺省语义 = 'active'；
//! - `services/crm/lead.rs:718`：线索转客户 `status: Set(master_data::ACTIVE…)`；
//! - `services/import_export_ops/import.rs:202`：导入建客户 `status: Set(master_data::ACTIVE…)`；
//! - `services/customer_ops/crud.rs:179-187`：软删除置 `master_data::INACTIVE`（'inactive'）；
//! - `handlers/customer_merge_handler.rs:224`：合并源客户置 'merged'（用户 2026-10-02
//!   裁定方案 A，见该文件 :50 注释）；
//! - `services/data_permission_service.rs:278`：读侧按 `status != 'blacklist'` 排除黑名单行，
//!   'blacklist' 为模型注释（`models/customer.rs:61` "active-活跃，inactive-停用，
//!   blacklist-黑名单"）在册的第四合法值；
//! - 前端同源：`views/crm/tabs/CustomerListTab.vue:94-97,569-585`（注释明示"active/inactive/
//!   blacklist 三态取值与后端写入方状态词表逐字符同源"）、`views/customer/index.vue:98-99`
//!   （筛选 active/inactive）、`views/customer/tabs/CustomerFormTab.vue:178,326,405`
//!   （表单选项与重置缺省均 'active'）。
//! ⇒ 存量为 NULL 的行都是"建列之前就已存在、从未被写入过状态"的行，其成文缺省
//!   即创建通道一致的 'active'；回填 'active' 是落实既有词表缺省，不是词表外造值。
//!
//! payment_terms 缺省：
//! - `backend/src/constants.rs:28` `DEFAULT_PAYMENT_TERMS_DAYS: i32 = 30`（成文常量）；
//! - `handlers/customer_handler.rs:306-309`：创建入参为空时 `unwrap_or(DEFAULT_PAYMENT_TERMS_DAYS)`；
//! - `services/crm/lead.rs:716`、`services/import_export_ops/import.rs:198`：两条建客户
//!   通道直接 `Set(DEFAULT_PAYMENT_TERMS_DAYS)`；
//! - 前端 `views/customer/tabs/CustomerFormTab.vue:323,402`：表单初值与重置值均 30。
//! ⇒ 缺省语义 = 30 天；回填 30 有业务默认证据（任务书中"无业务默认证据才用 0"的前提
//!   不成立，本列有明确成文默认）。
//!
//! ## 纪律
//! - 只新建链尾顶层域，绝不修改已应用迁移：`system/mod.rs:243/250` 的裸可空定义保持
//!   原样（历史迁移被 `_seaorm_` 台账记账，改写文本对已部署库永不重放）；并入已应用
//!   的 system 域同样会在复用库上被整体跳过，故按 lib.rs 惯例置于全链尾。
//! - 本迁移只归一 NULL（唯一异常形态，语义由上述写口缺省闭合）；非 NULL 的存量值
//!   一律不触碰。'merged'/'blacklist' 等词表内既有值原样保留。
//! - 不加 CHECK：本任务口径只要求 DEFAULT + NOT NULL；status 词表含四值
//!   （active/inactive/blacklist/merged）且 merged 由合并流程写入，CHECK 若收紧
//!   取值需另行按词表扩值惯例（同 price_vocab_extend 的"拒滚探测再重建"）单独立迁移，
//!   不在此顺带发明。
//!
//! ## 幂等
//! UPDATE 在 NOT NULL 生效后恒无匹配行；SET DEFAULT / SET NOT NULL / COMMENT 重放等价 ⇒ up 可重跑。
//!
//! ## down 真实可逆（本仓纪律：down 不留空实现）
//! 恢复"可空、无默认"的未归一形态。数据无损性论证：up 只把 NULL 归为成文缺省
//! （'active' / 30），这两个取值在旧形态同样可表达，down 后不存在被吞掉的取值形态，
//! 无需拒滚探测；但回滚即重新打开"NULL 行整行解码失败"缺陷（模型仍是非 Option），
//! 故 down 仅用于整链回退演练。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- 1) 存量回填：NULL = 建列前已存在、从未写入过的行，按各列成文缺省归一
--    （词表与写口证据见文件头；非 NULL 行一律不触碰）。
UPDATE "customers" SET "status" = 'active' WHERE "status" IS NULL;
UPDATE "customers" SET "payment_terms" = 30 WHERE "payment_terms" IS NULL;

-- 2) 列约束归一：与模型 customer.rs 的非 Option 声明对齐（先 DEFAULT 再 NOT NULL，
--    与 batch_no（system/mod.rs:256-261）、owner_id（m0070）同族收敛顺序一致）。
ALTER TABLE "customers" ALTER COLUMN "status" SET DEFAULT 'active';
ALTER TABLE "customers" ALTER COLUMN "status" SET NOT NULL;
ALTER TABLE "customers" ALTER COLUMN "payment_terms" SET DEFAULT 30;
ALTER TABLE "customers" ALTER COLUMN "payment_terms" SET NOT NULL;

-- 3) 列注释固化词表与缺省，防止后续把词表外取值当缺省写入。
COMMENT ON COLUMN "customers"."status" IS '状态：active-活跃（创建缺省），inactive-停用，blacklist-黑名单，merged-已合并（源客户）；比较点与写入值逐字符同源';
COMMENT ON COLUMN "customers"."payment_terms" IS '账期天数（创建缺省 30，权威常量 backend/src/constants.rs DEFAULT_PAYMENT_TERMS_DAYS）';
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 恢复为可空、无默认的未归一形态。up 只把 NULL 归为旧形态同样可表达的取值，
        // 回滚不吞数据、无需拒滚探测（论证见文件头）；回滚会重新打开"NULL 行整行
        // 解码失败"缺陷，仅用于整链回退演练。
        let sql = r#"
ALTER TABLE "customers" ALTER COLUMN "status" DROP NOT NULL;
ALTER TABLE "customers" ALTER COLUMN "status" DROP DEFAULT;
ALTER TABLE "customers" ALTER COLUMN "payment_terms" DROP NOT NULL;
ALTER TABLE "customers" ALTER COLUMN "payment_terms" DROP DEFAULT;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}

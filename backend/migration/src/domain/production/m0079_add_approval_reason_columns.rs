//! 交易域审批「通过 / 拒绝」双理由专列迁移（非 BPM 域）
//!
//! ## 六要素
//! - **功能**：为交易域 8 张业务审批表补齐可空理由专列（通过理由 `approval_reason`、
//!   拒绝理由 `rejected_reason`、取消理由 `cancel_reason`），使审批双动作各有独立落库列，
//!   不再挪用通用列（`notes` / `reason_detail`）或借用他动作列。
//! - **调用方**：SeaORM migrator 链，经 `domain/production/mod.rs` 聚合在本域 up 链尾调用
//!   （注册位置依据见 production/mod.rs 注释：目标表 sales_quotations 由 sales_crm 域建表，
//!   production 域晚于其执行；其余 7 表由 system/business 域更早建表）。
//! - **入参**：无（纯 DDL 迁移，不读取应用数据）。
//! - **传给谁**：列由后续后端写入方落值——`services/sales_price_service.rs`、
//!   `purchase_price_service.rs`、`quotation_approval_service.rs`、
//!   `purchase_contract_service.rs`、`sales_contract_service.rs`、`po/contract.rs`、
//!   `so/contract.rs`、`purchase_return_service.rs` 的 approve/reject/cancel 路径；
//!   列语义与归属逐条见下方列注释。应用层"必填/选填"分档在 handler 强制
//!   （必填=价目两表/报价单/两合同；选填=采购订单/销售订单/采购退货），
//!   DB 层一律可空：存量行无从回填理由，NOT NULL 会在加列瞬间逼出造值。
//! - **存什么**：审批裁量理由原文（TEXT，不做截断；应用层校验上限与列型对齐）。
//! - **存哪里**：PostgreSQL `public` schema 下各业务表新增列，无数据行搬移。
//!
//! ## 存储范式分叉边界（本迁移的否决面，防误用）
//! BPM 接入裁决（生产工单审批、中大额报价走 BPM 的裁决意见）只写
//! `bpm_task.approval_opinion`，本迁移**不为 BPM 域业务表新建理由列**；
//! `sales_quotations.approval_reason` 仅承载非 BPM 直批路径的通过理由，
//! BPM 裁决不回写该列，避免同一裁量双源。
//!
//! ## 存量安全性论证
//! 全部 14 列均为「可空、无默认值」的 ADD COLUMN：PostgreSQL 11+ 加 NULL 默认列
//! 不重写表，存量行该列一律 NULL，语义=「历史上未采集/未落该列」，零数据改写；
//! 不做任何 UPDATE 回填（拒绝在迁移里造值）。既有列一律不动：
//! - `sales_quotations.rejection_reason`（sales_crm/mod.rs:60，TEXT）已存在——不重命名、
//!   不新建同义列；
//! - `purchase_orders.rejected_reason`（system/mod.rs:344，VARCHAR(255)）已存在且真实
//!   承载 reject 理由——保持原名原型，仅由新列 `cancel_reason` 拆出被挪用的 cancel 语义；
//! - `purchase_return.rejected_reason`（business/m0009:199，TEXT）已存在——不重建。
//!
//! ## 幂等与回退
//! up 用 `ADD COLUMN IF NOT EXISTS` + `COMMENT ON COLUMN`，重跑等价；
//! down 对称 `DROP COLUMN IF EXISTS` 全部 14 列。回滚语义=回到 up 之前：up 之后
//! 应用写入这些列的值随列丢弃（口径同 crm_lead_claim_record——新增可空列无需备份表，
//! 存量行本就是 NULL）。列上不得建任何依赖对象（本迁移不建索引/约束），DROP 无级联意外。
//!
//! ## 上线前只读摸底（存量库，确认列不存在/形态预期）
//! ```text
//! SELECT table_name, column_name, data_type, is_nullable
//!   FROM information_schema.columns
//!  WHERE table_schema = 'public'
//!    AND table_name IN ('sales_prices','purchase_prices','sales_quotations',
//!                       'purchase_contracts','sales_contracts','purchase_orders',
//!                       'sales_orders','purchase_return')
//!    AND column_name IN ('approval_reason','rejected_reason','rejection_reason','cancel_reason')
//!  ORDER BY table_name, column_name;
//! -- 预期仅见：purchase_orders.rejected_reason varchar(255)、
//! --           purchase_return.rejected_reason text、
//! --           sales_quotations.rejection_reason text；
//! -- 出现其它组合说明链上有漂移，先人工定性再跑本迁移。
//! ```

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- ========== 1) 价目两表：通过理由（应用层必填档，DB 可空）+ 拒绝理由 ==========
ALTER TABLE "sales_prices"    ADD COLUMN IF NOT EXISTS "approval_reason" TEXT;
ALTER TABLE "sales_prices"    ADD COLUMN IF NOT EXISTS "rejected_reason"  TEXT;
ALTER TABLE "purchase_prices" ADD COLUMN IF NOT EXISTS "approval_reason" TEXT;
ALTER TABLE "purchase_prices" ADD COLUMN IF NOT EXISTS "rejected_reason"  TEXT;
COMMENT ON COLUMN "sales_prices"."approval_reason"    IS '审批通过理由（生效承诺类，应用层必填；NULL=历史行未采集或选填留空）。写入方：services/sales_price_service.rs approve 路径';
COMMENT ON COLUMN "sales_prices"."rejected_reason"    IS '审批拒绝理由（必填，服务端 trim 非空强制）。写入方：sales_price_service reject 路径；与 approval_reason 两动作两列，不共用';
COMMENT ON COLUMN "purchase_prices"."approval_reason" IS '审批通过理由（生效承诺类，应用层必填；NULL=历史行未采集或选填留空）。写入方：services/purchase_price_service.rs approve 路径';
COMMENT ON COLUMN "purchase_prices"."rejected_reason" IS '审批拒绝理由（必填，服务端 trim 非空强制）。写入方：purchase_price_service reject 路径';

-- ========== 2) 销售报价单：通过理由专列 ==========
-- 拒绝理由已有专列 rejection_reason（sales_crm 建表，TEXT），保持不动。
ALTER TABLE "sales_quotations" ADD COLUMN IF NOT EXISTS "approval_reason" TEXT;
COMMENT ON COLUMN "sales_quotations"."approval_reason" IS '审批通过理由（生效承诺类，应用层必填；NULL=历史行未采集）。仅承载非 BPM 直批路径；中大额报价的 BPM 裁决意见只存 bpm_task.approval_opinion，禁止双写本列';

-- ========== 3) 两合同表：通过理由 + 拒绝理由 ==========
ALTER TABLE "purchase_contracts" ADD COLUMN IF NOT EXISTS "approval_reason" TEXT;
ALTER TABLE "purchase_contracts" ADD COLUMN IF NOT EXISTS "rejected_reason"  TEXT;
ALTER TABLE "sales_contracts"    ADD COLUMN IF NOT EXISTS "approval_reason" TEXT;
ALTER TABLE "sales_contracts"    ADD COLUMN IF NOT EXISTS "rejected_reason"  TEXT;
COMMENT ON COLUMN "purchase_contracts"."approval_reason" IS '合同审批通过理由（权利义务生效承诺类，应用层必填；NULL=历史行未采集）。写入方：services/purchase_contract_service.rs approve 路径';
COMMENT ON COLUMN "purchase_contracts"."rejected_reason" IS '合同审批拒绝理由（必填，服务端强制）。写入方：purchase_contract_service reject 路径；cancel（作废）动作不使用本列';
COMMENT ON COLUMN "sales_contracts"."approval_reason"    IS '合同审批通过理由（生效承诺类，应用层必填；NULL=历史行未采集）。写入方：services/sales_contract_service.rs approve 路径';
COMMENT ON COLUMN "sales_contracts"."rejected_reason"    IS '合同审批拒绝理由（必填，服务端强制）。写入方：sales_contract_service reject 路径；cancel 动作不使用本列';

-- ========== 4) 采购订单：通过理由 + 拆出取消理由专列 ==========
-- rejected_reason（VARCHAR(255)，system/mod.rs 加列）继续只承载 reject 拒绝理由；
-- 此前 po/contract.rs cancel 路径挪用该列，本列建好后 cancel 改写 cancel_reason。
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "approval_reason" TEXT;
ALTER TABLE "purchase_orders" ADD COLUMN IF NOT EXISTS "cancel_reason"    TEXT;
COMMENT ON COLUMN "purchase_orders"."approval_reason" IS '审批通过理由（流程放行类，选填：可留空=NULL）。写入方：services/po/contract.rs approve 路径';
COMMENT ON COLUMN "purchase_orders"."cancel_reason"   IS '取消（cancel）动作理由专列。应使用本列的写入方：services/po/contract.rs cancel 路径；不得再挪用 rejected_reason（该列回归只承载 reject 拒绝理由，两动作两列）';

-- ========== 5) 销售订单：通过理由 + 拒绝理由专列（停写 notes） ==========
-- 现 reject 理由覆盖写 sales_orders.notes（通用备注列，毁数挪用）；本列建好后
-- so/contract.rs reject 改写 rejected_reason，notes 回归订单备注语义。
-- 存量 notes 中混杂的历史拒绝理由不回填：无类型标记，猜语义会造成二次污染。
ALTER TABLE "sales_orders" ADD COLUMN IF NOT EXISTS "approval_reason" TEXT;
ALTER TABLE "sales_orders" ADD COLUMN IF NOT EXISTS "rejected_reason"  TEXT;
COMMENT ON COLUMN "sales_orders"."approval_reason" IS '审批通过理由（流程放行类，选填：可留空=NULL）。写入方：services/so/contract.rs approve 路径';
COMMENT ON COLUMN "sales_orders"."rejected_reason" IS '审批拒绝理由（必填，服务端强制）。写入方：services/so/contract.rs reject 路径；历史拒绝理由曾挤在 notes 列，本列生效起不再挪用 notes';

-- ========== 6) 采购退货：通过理由（rejected_reason 已由 m0009 建列，不重建） ==========
ALTER TABLE "purchase_return" ADD COLUMN IF NOT EXISTS "approval_reason" TEXT;
COMMENT ON COLUMN "purchase_return"."approval_reason" IS '审批通过理由（流程放行类，选填：可留空=NULL）。写入方：services/purchase_return_service.rs approve 路径；拒绝理由既有专列 rejected_reason（m0009 建列，TEXT），列注释更正与错列写入收口属写入方批次，本迁移不动该列';
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 对称回退：逐一 DROP 本迁移新增的 14 列（IF EXISTS 保证重放安全）。
        // 只 DROP 列，不触碰任何既有列（rejected_reason of purchase_orders、
        // rejection_reason of sales_quotations、rejected_reason of purchase_return
        // 均先于本迁移存在，不属本迁移回退面）。up 之后应用已写入的列值随回滚丢弃，
        // 回滚语义=回到 up 之前（新增可空列无存量改写，无需备份表）。
        let sql = r#"
ALTER TABLE "sales_prices"       DROP COLUMN IF EXISTS "approval_reason";
ALTER TABLE "sales_prices"       DROP COLUMN IF EXISTS "rejected_reason";
ALTER TABLE "purchase_prices"    DROP COLUMN IF EXISTS "approval_reason";
ALTER TABLE "purchase_prices"    DROP COLUMN IF EXISTS "rejected_reason";
ALTER TABLE "sales_quotations"   DROP COLUMN IF EXISTS "approval_reason";
ALTER TABLE "purchase_contracts" DROP COLUMN IF EXISTS "approval_reason";
ALTER TABLE "purchase_contracts" DROP COLUMN IF EXISTS "rejected_reason";
ALTER TABLE "sales_contracts"    DROP COLUMN IF EXISTS "approval_reason";
ALTER TABLE "sales_contracts"    DROP COLUMN IF EXISTS "rejected_reason";
ALTER TABLE "purchase_orders"    DROP COLUMN IF EXISTS "approval_reason";
ALTER TABLE "purchase_orders"    DROP COLUMN IF EXISTS "cancel_reason";
ALTER TABLE "sales_orders"       DROP COLUMN IF EXISTS "approval_reason";
ALTER TABLE "sales_orders"       DROP COLUMN IF EXISTS "rejected_reason";
ALTER TABLE "purchase_return"    DROP COLUMN IF EXISTS "approval_reason";
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }
}

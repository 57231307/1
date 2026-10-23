//! 进项税额二级科目建账：把"进项"从销项科目 222101 下拆出来
//!
//! ## 六要素
//! - **功能**：在 `account_subjects` 增建 `222102 应交税费-应交增值税-进项税额`
//!   （level 2，parent `2221`，余额方向 debit），使采购侧进项税额有独立科目可落。
//! - **调用方**：SeaORM 迁移链，注册在 `domain/mod.rs` 的域列表末尾与 `lib.rs`
//!   的迁移列表末尾（晚于建 `account_subjects` 并种入 2221/222101 的 finance 域）。
//! - **入参**：无。科目码、名称、方向都是本文件的编译期常量。
//! - **传给谁**：`SchemaManager` 的裸 SQL 连接；运行时由
//!   `services/ap_invoice_ops/receipt.rs` 的进项分录按 `222102` 提交，
//!   凭证服务再用 `subject_code` 反查 `account_subjects` 补 id/name 落 `voucher_items`。
//! - **存什么**：只写 `account_subjects` 的一行科目档案。
//! - **存哪里**：PostgreSQL `public.account_subjects`。
//!
//! ## 为什么要单拆一枚
//! `222101` 的种子名称是"应交税费-应交增值税-销项税额"，但采购入库/退货的进项税额
//! 分录一直写这个码，且行上 `subject_name` 记的是"进项税额"——销售销项与采购进项同挂
//! 一个科目码，月末同码互抵，进税额被销项吃掉，应交增值税明细账两条业务线的发生额
//! 再也分不开。本迁移只建科目，历史凭证行不回写（见下）。
//!
//! ## 存量口径
//! 历史 `voucher_items` 里 `subject_code = '222101'` 且 `subject_name` 含"进项税额"的行
//! 属既成事实，按会计留痕原则不 UPDATE 改码；迁移只把这类行的数量以 NOTICE 报出，
//! 由财务按《凭证调整》流程自行更正。新凭证自本迁移生效起直接落 `222102`。
//!
//! ## 幂等与安全
//! `ON CONFLICT (code) DO NOTHING` 使重跑等价；父科目 `2221` 不存在时 RAISE EXCEPTION
//! 中止（缺父科目即建不出可用的二级科目，静默跳过会让进项分录继续落错码）。
//! down 只删"没有任何凭证行引用"的 `222102`；被引用时保留并 RAISE NOTICE，
//! 避免删科目把已入账凭证变成悬空引用。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 进项税额科目码：销项 222101 之后新开一枚，二者同为 2221 的二级科目
const INPUT_TAX_CODE: &str = "222102";
const INPUT_TAX_NAME: &str = "应交税费-应交增值税-进项税额";
const TAX_PARENT_CODE: &str = "2221";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = format!(
            r#"
-- 1) 父科目必须已在：2221 由 finance 域种子建出，缺它即本迁移前提不成立
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM "account_subjects" WHERE "code" = '{parent}') THEN
        RAISE EXCEPTION 'm_ap_input_tax_subject：父科目 % 不存在，无法建进项税额二级科目；请核对 finance 域科目种子是否已应用，禁止在缺父科目时静默跳过。', '{parent}';
    END IF;
END
$$;

-- 2) 建科目（余额方向 debit：进项税额是应交增值税的借方明细）
INSERT INTO "account_subjects"
    ("code", "name", "level", "parent_id", "full_code", "balance_direction",
     "assist_customer", "assist_supplier", "is_cash_account", "is_bank_account", "status")
SELECT '{code}', '{name}', 2,
       (SELECT p."id" FROM "account_subjects" p WHERE p."code" = '{parent}'),
       '{code}', 'debit', false, false, false, false, 'active'
 WHERE NOT EXISTS (SELECT 1 FROM "account_subjects" x WHERE x."code" = '{code}')
 ON CONFLICT ("code") DO NOTHING;

-- 3) 存量非可见化：历史错码进项行只报数不改写（会计留痕，改由财务走调整凭证）
DO $$
DECLARE
    hit INTEGER;
BEGIN
    SELECT COUNT(*) INTO hit FROM "voucher_items"
     WHERE "subject_code" = '{parent_code_01}' AND "subject_name" LIKE '%进项税额%';
    IF hit > 0 THEN
        RAISE NOTICE 'm_ap_input_tax_subject：历史凭证行中有 % 条进项税额记在销项科目 % 下；本迁移不改写已入账凭证，请按凭证调整流程更正，新建凭证已落进项科目 %。', hit, '{parent_code_01}', '{code}';
    END IF;
END
$$;
"#,
            code = INPUT_TAX_CODE,
            name = INPUT_TAX_NAME,
            parent = TAX_PARENT_CODE,
            parent_code_01 = "222101",
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = format!(
            r#"
-- 只回收无人引用的进项科目；已被凭证行引用时保留，删了会把已入账凭证留成悬空引用
DO $$
DECLARE
    used INTEGER;
BEGIN
    SELECT COUNT(*) INTO used FROM "voucher_items" vi
      JOIN "account_subjects" s ON s."id" = vi."subject_id"
     WHERE s."code" = '{code}';
    IF used > 0 THEN
        RAISE NOTICE 'm_ap_input_tax_subject down：科目 % 已被 % 条凭证行引用，保留不删。', '{code}', used;
    ELSE
        DELETE FROM "account_subjects" WHERE "code" = '{code}';
        RAISE NOTICE 'm_ap_input_tax_subject down：科目 % 无凭证引用，已删除。', '{code}';
    END IF;
END
$$;
"#,
            code = INPUT_TAX_CODE,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}

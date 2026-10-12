//! 不合格品台账两条被吞掉约束的列收紧为 NOT NULL + DEFAULT
//!
//! ## 六要素
//! - **功能**：把 `unqualified_products.stock_grade_synced`（降级联动库存同步标志）与
//!   `unqualified_products.scrap_approval_status`（报废二级审批状态）由「可空且无默认」
//!   收紧为 `NOT NULL` 并补默认 `false` / `'not_required'`。
//! - **调用方**：SeaORM migrator 链，注册在 `domain/ddl_nullability_tighten/` 域，排全链尾
//!   （建表与两次加列都早于本域）。
//! - **入参**：无（纯 DDL 迁移）。
//! - **传给谁**：无应用层调用方；形态由 `backend/tests/contract_wave7_column_shape_lock_test.rs`
//!   的 `EXPECTED` 钉住。写入侧 `models/unqualified_product.rs` 两字段声明为非 Option
//!   的 `bool` / `String`，读取侧在 `services/quality_inspection_service.rs` 直接取值。
//! - **存什么**：只改列的 nullability 与 default，不改任何数据行。
//! - **存哪里**：PostgreSQL `public.unqualified_products` 上述两列。
//!
//! ## 缺陷成因（同域前几条都源于"更早的裸加列吞掉后一条整句"）
//! 更早的加列语句把这两列写成裸 `BOOLEAN` / `VARCHAR(255)`（无 NOT NULL、无 DEFAULT），
//! 更晚的域再用 `ADD COLUMN IF NOT EXISTS ... NOT NULL DEFAULT …` 表达归一意图时，
//! 整句因列已存在而成为 no-op —— 于是新库上这两列的可空性与"模型非 Option"直接冲突：
//! 任何未显式给值的写入（裸 SQL 插入、历史行）落 NULL，随后一次普通读取即报列空值错误。
//! 这里同时补回 DEFAULT，让省略列的写入取合法初值而不是重新制造 NULL。
//!
//! ## 默认值取值依据
//! - `stock_grade_synced` 取 `false`：未同步是唯一诚实初值，同步动作由降级联动显式置真。
//! - `scrap_approval_status` 取 `'not_required'`：与写入方权威词表
//!   `SCRAP_NOT_REQUIRED` 逐字一致，也是不需报废审批的行应有的态。
//!
//! ## 幂等与安全
//! 先 fail-visible 点名存量 NULL 并中止（拒绝在带 NULL 存量的列上静默收紧，也不在迁移里
//! UPDATE 造值），再 `SET DEFAULT` + `SET NOT NULL`，最后读 `information_schema` 自证
//! 默认与 NOT NULL 双双生效。down 撤销 NOT NULL 与本次补上的默认，回到收紧前形态。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const COLS: [(&str, &str, &str); 2] = [
    ("unqualified_products", "stock_grade_synced", "false"),
    (
        "unqualified_products",
        "scrap_approval_status",
        "'not_required'::character varying",
    ),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut guard = String::from("DO $$\nDECLARE\n    bad_count INTEGER;\nBEGIN\n");
        for (table, column, _) in COLS {
            guard.push_str(&format!(
                "    SELECT COUNT(*) INTO bad_count FROM \"{table}\" WHERE \"{column}\" IS NULL;\n\
                 IF bad_count > 0 THEN\n\
                 \x20   RAISE EXCEPTION '{table}.{column} 存在 % 行 NULL。本迁移拒绝在带 NULL 存量的列上静默收紧 NOT NULL，且不做 UPDATE 造值归一：请人工核实上述行的真实取值后重跑。', bad_count;\n\
                 END IF;\n",
            ));
        }
        guard.push_str("END\n$$;\n");
        manager
            .get_connection()
            .execute_unprepared(&guard)
            .await
            .map_err(|e| DbErr::Custom(format!("不合格品台账列收紧存量点名守卫执行失败: {e}")))?;

        for (table, column, default) in COLS {
            let sql = format!(
                "ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" SET DEFAULT {default};\n\
                 ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" SET NOT NULL;",
            );
            manager
                .get_connection()
                .execute_unprepared(&sql)
                .await
                .map_err(|e| {
                    DbErr::Custom(format!(
                        "{table}.{column} 收紧失败: {e}（该列此前长期处于可空且无默认形态）"
                    ))
                })?;
        }

        let mut verify =
            String::from("DO $$\nDECLARE\n    v_nullable TEXT;\n    v_default TEXT;\nBEGIN\n");
        for (table, column, _) in COLS {
            verify.push_str(&format!(
                "    SELECT is_nullable, column_default INTO v_nullable, v_default\n\
                 \x20     FROM information_schema.columns\n\
                 \x20    WHERE table_schema = 'public' AND table_name = '{table}' AND column_name = '{column}';\n\
                 \x20   IF v_nullable IS NULL THEN\n\
                 \x20     RAISE EXCEPTION 'information_schema 中找不到 {table}.{column}，与建表迁移漂移，中止';\n\
                 \x20   END IF;\n\
                 \x20   IF v_nullable <> 'NO' THEN\n\
                 \x20     RAISE EXCEPTION '收尾自证失败：{table}.{column} is_nullable = %（期望 NO），NOT NULL 未真实生效', v_nullable;\n\
                 \x20   END IF;\n\
                 \x20   IF v_default IS NULL THEN\n\
                 \x20     RAISE EXCEPTION '收尾自证失败：{table}.{column} column_default 为空，DEFAULT 未真实生效';\n\
                 \x20   END IF;\n",
            ));
        }
        verify.push_str("END\n$$;\n");
        manager.get_connection().execute_unprepared(&verify).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for (table, column, _) in COLS {
            let sql = format!(
                "ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" DROP NOT NULL;\n\
                 ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" DROP DEFAULT;",
            );
            manager.get_connection().execute_unprepared(&sql).await?;
        }
        Ok(())
    }
}

//! 排序权重与出站开关列收紧为 NOT NULL（补回被吞掉的 DEFAULT）
//!
//! ## 六要素
//! - **功能**：把 `product_color_prices.priority`（整型排序权重）与
//!   `notification_settings.enable_webhook`（出站开关）由「可空且**无默认**」收紧为
//!   `NOT NULL` 并补上默认值。这两列是实测出来的写法缺陷受害者：建表时写的是
//!   `INT NOT NULL DEFAULT 0` / `BOOLEAN NOT NULL DEFAULT false` 整句，但更早的裸列定义
//!   使整句成为 no-op，结果连 DEFAULT 一起丢失，NOT NULL 也没落地。
//! - **调用方**：SeaORM migrator 链，注册在 `domain/ddl_nullability_tighten/` 域，排全链尾
//!   （两表分别由 price / system 域建表，均早于本域）。
//! - **入参**：无（纯 DDL 迁移）。
//! - **传给谁**：无应用层调用方；形态由 `backend/tests/contract_wave7_column_shape_lock_test.rs`
//!   的 `EXPECTED` 钉住。
//! - **存什么**：只改列的 nullability 与 default，不改任何数据行。
//! - **存哪里**：PostgreSQL `public.product_color_prices.priority`、
//!   `public.notification_settings.enable_webhook`。
//!
//! ## 默认值取值依据
//! - `priority` 取 `0`：写入方在请求未给权重时按 0 落库（语义中立，不新增口径）。
//! - `enable_webhook` 取 `false`：fail-closed 侧，且是本列唯一写入方的建单值；
//!   同表其余通道列本就是 `NOT NULL DEFAULT`，此列只是漏收紧。
//!
//! ## 幂等与安全
//! 先 fail-visible 点名存量 NULL 并中止，再 `SET DEFAULT` + `SET NOT NULL`，
//! 最后读 `information_schema` 自证默认与 NOT NULL 双双生效（本列的历史教训正是
//! "默认写了但没生效"，故自证不可省）。down 撤销 NOT NULL 与本次补上的默认，
//! 回到收紧前的裸可空形态。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

const COLS: [(&str, &str, &str); 2] = [
    ("product_color_prices", "priority", "0"),
    ("notification_settings", "enable_webhook", "false"),
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
            .map_err(|e| DbErr::Custom(format!("权重/开关列收紧存量点名守卫执行失败: {e}")))?;

        for (table, column, default) in COLS {
            let sql = format!(
                "ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" SET DEFAULT {default};\n\
                 ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" SET NOT NULL;",
            );
            manager.get_connection().execute_unprepared(&sql).await?;
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
                 \x20     RAISE EXCEPTION '收尾自证失败：{table}.{column} column_default 为空，DEFAULT 未真实生效（本列历史上正是默认被吞的受害者）';\n\
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

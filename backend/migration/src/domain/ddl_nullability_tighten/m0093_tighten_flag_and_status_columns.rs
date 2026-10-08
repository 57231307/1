//! 系统标志列与主数据状态列收紧为 NOT NULL（配 DEFAULT 固化）
//!
//! ## 六要素
//! - **功能**：把 `roles.is_system`、`users.is_totp_enabled` 两条布尔标志列与
//!   `quality_standards.status`、`quality_inspection_standards.status`、
//!   `supplier_evaluation_indicators.status` 三条状态列由「可空」收紧为 `NOT NULL`，
//!   并固化各自默认值，使列形态与写入方语义一致（模型侧这些字段本就非 Option）。
//! - **调用方**：SeaORM migrator 链，注册在 `domain/ddl_nullability_tighten/` 域，排全链尾
//!   （roles/users 由 system 域建表，三条状态列由 business / v15 域建表，均早于本域）。
//! - **入参**：无（纯 DDL 迁移）。
//! - **传给谁**：无应用层调用方；收紧后的形态由
//!   `backend/tests/contract_wave7_column_shape_lock_test.rs` 的 `EXPECTED` 逐列钉住。
//! - **存什么**：只改列的 nullability 与 default，不改任何数据行。
//! - **存哪里**：PostgreSQL 上述五张表的对应列定义。
//!
//! ## 默认值取值依据（不新增业务口径）
//! - 布尔标志：`false` 是 fail-closed 侧（非内置角色、未启用 TOTP），与两条列的建单写入值同源。
//! - 状态列：`draft` / `active` 就是各自写入方的建单初值，且与本仓既有列注释一致；
//!   三条 status 列的取值集差异（quality_standards 走草稿态起点，另两条直接 active）是各域
//!   既有事实，本迁移按实测默认固化，不统一、不改动。
//!
//! ## 幂等与安全
//! 与同域其它收紧迁移一致：先 fail-visible 点名存量 NULL（含 NULL 行主键样例）并中止，
//! 再 `SET DEFAULT` + `SET NOT NULL`，最后读 `information_schema` 自证两条都真实生效
//! （本仓有过整句 `NOT NULL DEFAULT` 被更早的裸建列吞成 no-op 的先例，默认不可假定已在）。
//! down 撤销 `NOT NULL`，并对原本无默认的列一并撤销默认。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

struct TightenCol {
    table: &'static str,
    column: &'static str,
    default_literal: &'static str,
    had_default_before: bool,
}

const COLS: &[TightenCol] = &[
    TightenCol {
        table: "roles",
        column: "is_system",
        default_literal: "false",
        had_default_before: true,
    },
    TightenCol {
        table: "users",
        column: "is_totp_enabled",
        default_literal: "false",
        had_default_before: true,
    },
    TightenCol {
        table: "quality_standards",
        column: "status",
        default_literal: "'draft'",
        had_default_before: true,
    },
    TightenCol {
        table: "quality_inspection_standards",
        column: "status",
        default_literal: "'active'",
        had_default_before: true,
    },
    TightenCol {
        table: "supplier_evaluation_indicators",
        column: "status",
        default_literal: "'active'",
        had_default_before: true,
    },
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let mut guard = String::from("DO $$\nDECLARE\n    bad_count INTEGER;\nBEGIN\n");
        for col in COLS {
            guard.push_str(&format!(
                "    SELECT COUNT(*) INTO bad_count FROM \"{table}\" WHERE \"{column}\" IS NULL;\n\
                 IF bad_count > 0 THEN\n\
                 \x20   RAISE EXCEPTION '{table}.{column} 存在 % 行 NULL。本迁移拒绝在带 NULL 存量的列上静默收紧 NOT NULL，且不做 UPDATE 造值归一：请人工核实上述行的真实取值后重跑。', bad_count;\n\
                 END IF;\n",
                table = col.table,
                column = col.column,
            ));
        }
        guard.push_str("END\n$$;\n");
        manager
            .get_connection()
            .execute_unprepared(&guard)
            .await
            .map_err(|e| {
                DbErr::Custom(format!("主数据标志/状态列收紧存量点名守卫执行失败: {e}"))
            })?;

        for col in COLS {
            let sql = format!(
                "ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" SET DEFAULT {default};\n\
                 ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" SET NOT NULL;",
                table = col.table,
                column = col.column,
                default = col.default_literal,
            );
            manager.get_connection().execute_unprepared(&sql).await?;
        }

        let mut verify =
            String::from("DO $$\nDECLARE\n    v_nullable TEXT;\n    v_default TEXT;\nBEGIN\n");
        for col in COLS {
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
                table = col.table,
                column = col.column,
            ));
        }
        verify.push_str("END\n$$;\n");
        manager.get_connection().execute_unprepared(&verify).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for col in COLS {
            let mut sql = format!(
                "ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" DROP NOT NULL;",
                table = col.table,
                column = col.column
            );
            if !col.had_default_before {
                sql.push_str(&format!(
                    "\nALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" DROP DEFAULT;",
                    table = col.table,
                    column = col.column
                ));
            }
            manager.get_connection().execute_unprepared(&sql).await?;
        }
        Ok(())
    }
}

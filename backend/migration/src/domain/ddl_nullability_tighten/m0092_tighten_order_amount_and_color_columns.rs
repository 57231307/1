//! 订单金额列与色号列收紧为 NOT NULL（配 DEFAULT 固化）
//!
//! ## 六要素
//! - **功能**：把 `sales_orders` 的七条金额列、`purchase_orders.total_amount` 与 `sales_order_items.color_no`、
//!   `sales_delivery_item.color_no` 由「可空、部分无默认」收紧为 `NOT NULL` 并固化默认，
//!   使列形态与应用层写入语义一致（模型侧这些字段本就非 Option，可空 DDL 只可能来自
//!   旁路写入，不是领域允许为空）。
//! - **调用方**：SeaORM migrator 链，注册在 `domain/ddl_nullability_tighten/` 域，排全链尾
//!   （被收紧的列由 sales_crm / v15 等更早的域建表，必须晚于建表）。
//! - **入参**：无（纯 DDL 迁移）。
//! - **传给谁**：无应用层调用方；收紧后的形态由
//!   `backend/tests/contract_wave7_column_shape_lock_test.rs` 的 `EXPECTED` 逐列钉住。
//! - **存什么**：只改列的 nullability 与 default，不改任何数据行、不动约束名。
//! - **存哪里**：PostgreSQL `public.sales_orders` / `public.sales_order_items` /
//!   `public.sales_delivery_item` 的列定义。
//!
//! ## 为什么默认值取 0 / 空串
//! 金额列的写入方在计算订单总额时恒给出数值（缺项按 0 参与求和），色号列的白坯语义在本仓
//! 统一以空串表示（与同族已收紧列一致）；两者都不新增业务口径。
//!
//! ## 幂等与安全
//! up 顺序为「fail-visible 存量点名 → SET DEFAULT → SET NOT NULL → 收尾自证」：
//! 存量 NULL 行非 0 即中止并列出行数与样例主键，绝不用 UPDATE 造值洗数据；
//! `SET DEFAULT`/`SET NOT NULL` 本身可重复执行，重跑等价。收尾自证读
//! `information_schema.columns`，确认 `is_nullable='NO'` 且默认值真实生效——
//! 本仓有过 `ADD COLUMN IF NOT EXISTS … NOT NULL DEFAULT` 被更早的裸建列整句吞成 no-op
//! 的先例，故默认值必须逐列复核而非假定已在。
//! down 只撤销本迁移引入的两项：`DROP NOT NULL` + 对**原本无默认**的列 `DROP DEFAULT`；
//! 原本已带默认的列保留其默认，避免回滚把改动前就存在的定义一并抹掉。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 列收紧规格：`had_default_before` 决定 down 是否撤销默认。
struct TightenCol {
    table: &'static str,
    column: &'static str,
    /// `SET DEFAULT` 的字面值（数字不带引号、字符串带单引号）
    default_literal: &'static str,
    /// 收紧前该列是否已带默认（实测于当前迁移链产物）
    had_default_before: bool,
}

const COLS: &[TightenCol] = &[
    TightenCol {
        table: "sales_orders",
        column: "total_amount",
        default_literal: "0",
        had_default_before: true,
    },
    TightenCol {
        table: "sales_orders",
        column: "discount_amount",
        default_literal: "0",
        had_default_before: true,
    },
    TightenCol {
        table: "sales_orders",
        column: "paid_amount",
        default_literal: "0",
        had_default_before: true,
    },
    TightenCol {
        table: "sales_orders",
        column: "subtotal",
        default_literal: "0",
        had_default_before: false,
    },
    TightenCol {
        table: "sales_orders",
        column: "tax_amount",
        default_literal: "0",
        had_default_before: false,
    },
    TightenCol {
        table: "sales_orders",
        column: "shipping_cost",
        default_literal: "0",
        had_default_before: false,
    },
    TightenCol {
        table: "sales_orders",
        column: "balance_amount",
        default_literal: "0",
        had_default_before: false,
    },
    TightenCol {
        table: "purchase_orders",
        column: "total_amount",
        default_literal: "0",
        had_default_before: true,
    },
    TightenCol {
        table: "sales_order_items",
        column: "color_no",
        default_literal: "''",
        had_default_before: false,
    },
    TightenCol {
        table: "sales_delivery_item",
        column: "color_no",
        default_literal: "''",
        had_default_before: false,
    },
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1) fail-visible 存量点名：任一列存在 NULL 行即中止（整批单事务，中止即无半态）。
        let mut guard =
            String::from("DO $$\nDECLARE\n    bad_count INTEGER;\n    bad_samples TEXT;\nBEGIN\n");
        for col in COLS {
            let stmt = format!(
                "    SELECT COUNT(*) INTO bad_count FROM \"{table}\" WHERE \"{column}\" IS NULL;\n\
                 IF bad_count > 0 THEN\n\
                 \x20   SELECT string_agg(\"{column}\"::text, ', ') INTO bad_samples\n\
                 \x20     FROM (SELECT \"{column}\" FROM \"{table}\" WHERE \"{column}\" IS NULL LIMIT 10) s;\n\
                 \x20   RAISE EXCEPTION '{table}.{column} 存在 % 行 NULL（样例值：%）。本迁移拒绝在带 NULL 存量的列上静默收紧 NOT NULL，且不做 UPDATE 造值归一：请人工核实上述行的真实业务取值后重跑。', bad_count, bad_samples;\n\
                 END IF;\n",
                table = col.table,
                column = col.column,
            );
            guard.push_str(&stmt);
        }
        guard.push_str("END\n$$;\n");
        manager
            .get_connection()
            .execute_unprepared(&guard)
            .await
            .map_err(|e| DbErr::Custom(format!("订单金额/色号列收紧存量点名守卫执行失败: {e}")))?;

        // 2) 逐列固化默认并收紧。
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

        // 3) 收尾自证：读 information_schema 确认逐列真的既 NO 可空、又有默认。
        let mut verify =
            String::from("DO $$\nDECLARE\n    v_nullable TEXT;\n    v_default TEXT;\nBEGIN\n");
        for col in COLS {
            let stmt = format!(
                "    SELECT is_nullable, column_default INTO v_nullable, v_default\n\
                 \x20     FROM information_schema.columns\n\
                 \x20    WHERE table_schema = 'public' AND table_name = '{table}' AND column_name = '{column}';\n\
                 \x20   IF v_nullable IS NULL THEN\n\
                 \x20     RAISE EXCEPTION 'information_schema 中找不到 {table}.{column}，与本域建表迁移漂移，中止';\n\
                 \x20   END IF;\n\
                 \x20   IF v_nullable <> 'NO' THEN\n\
                 \x20     RAISE EXCEPTION '收尾自证失败：{table}.{column} is_nullable = %（期望 NO），NOT NULL 未真实生效', v_nullable;\n\
                 \x20   END IF;\n\
                 \x20   IF v_default IS NULL THEN\n\
                 \x20     RAISE EXCEPTION '收尾自证失败：{table}.{column} column_default 为空，DEFAULT 未真实生效';\n\
                 \x20   END IF;\n",
                table = col.table,
                column = col.column,
            );
            verify.push_str(&stmt);
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

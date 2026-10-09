//! 时间戳列形态归位：DDL 裸建 TIMESTAMP 但模型声明 DateTime<Utc> 的列改为 TIMESTAMPTZ
//!
//! ## 六要素
//! - **功能**：把「迁移/DDL 以不带时区的裸 `TIMESTAMP` 建列、而 SeaORM 模型字段声明为
//!   `DateTime<Utc>`（对应 SQL `TIMESTAMPTZ`）」的时间戳列，统一改为 `TIMESTAMPTZ`，
//!   使库列形态与模型语义对齐。本域专司列形态归位，本条负责时间戳这一列族。
//! - **调用方**：SeaORM migrator 链，注册在 `domain/ddl_nullability_tighten/` 域 up 链尾、
//!   down 链首。该域排在全部建表域（system / business / sales_crm / production / finance /
//!   v15 等）之后，被触及的列此时必已存在，改型不会撞到尚未建的表。
//! - **入参**：无（纯 DDL 迁移，不读取应用之外的数据）。
//! - **传给谁**：写入方为各业务 service（按 `DateTime<Utc>` 取值/写值），读取方为对应实体
//!   的列解码；列形态由 `backend/tests/contract_wave7_column_shape_lock_test.rs` 的
//!   `EXPECTED` 在活库 `information_schema` 逐列钉死。
//! - **存什么**：只改列类型，不改任何数据行。naive 值经 `AT TIME ZONE 'UTC'` 解释为 UTC
//!   瞬时再落 `TIMESTAMPTZ`，与 v15 全表归位所用完全同一转换式，值语义无损。
//! - **存哪里**：PostgreSQL `public` schema 下失配列的类型定义（不新建表、不删列）。
//!
//! ## 缺陷成因（列类型与模型解耦产生的解码面）
//! v15 域曾按一份硬编码清单把历史裸建时间戳列批量改为 `TIMESTAMPTZ`（清单成文即固化了
//! 当时存在的表列）。此后新增、且模型声明 `DateTime<Utc>` 的时间戳列若仍以裸 `TIMESTAMP`
//! 建出，就会落在该清单覆盖之外：库列停留在不带时区的 `TIMESTAMP`，而应用侧期望带时区的
//! `TIMESTAMPTZ`，读取即触发列解码类型失配、所在事务整体回滚。本条把这类"清单之外新裸建"
//! 的列改型归位，方向与全表 `created_at/updated_at` 的既有 `TIMESTAMPTZ` 口径一致。
//!
//! ## 幂等与安全
//! 先读 `information_schema.columns.data_type` 判定列当前形态：已是 `timestamp with time
//! zone` 则跳过（安全重跑），仍为不带时区的 `timestamp without time zone` 才执行改型，
//! 列在库中缺失则点名并中止（与建表漂移，不静默放过）。改型后另起一段收尾自证，强制每一列
//! 的 `data_type` 必为 `timestamp with time zone`，否则 RAISE EXCEPTION 中止、不留半态。
//! down 对称回退：把 `TIMESTAMPTZ` 以 `AT TIME ZONE 'UTC'` 转回不带时区的 `TIMESTAMP`，
//! 已是 naive 则跳过，与 up 互逆。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 普查坐实的失配列（DDL 裸建 TIMESTAMP、模型声明 DateTime<Utc>、且表在链尾存活）。
const COLS: [(&str, &str); 2] = [
    ("purchase_receipt", "concession_at"),
    ("purchase_receipt", "rejudge_at"),
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 1) 条件改型：仅对仍为不带时区的列做 TIMESTAMP → TIMESTAMPTZ，已归位则跳过（幂等）。
        let mut norm = String::from("DO $$\nDECLARE\n    v_dt TEXT;\nBEGIN\n");
        for (table, column) in COLS {
            norm.push_str(&format!(
                "    SELECT data_type INTO v_dt FROM information_schema.columns\n\
                 \x20     WHERE table_schema = 'public' AND table_name = '{table}' AND column_name = '{column}';\n\
                 \x20   IF v_dt IS NULL THEN\n\
                 \x20     RAISE EXCEPTION 'information_schema 中找不到 {table}.{column}，与建表迁移漂移，中止';\n\
                 \x20   END IF;\n\
                 \x20   IF v_dt = 'timestamp with time zone' THEN\n\
                 \x20     RAISE NOTICE '{table}.{column} 已是 timestamp with time zone，跳过改型（幂等重跑安全）';\n\
                 \x20   ELSE\n\
                 \x20     ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" TYPE TIMESTAMPTZ USING \"{column}\" AT TIME ZONE 'UTC';\n\
                 \x20   END IF;\n",
            ));
        }
        norm.push_str("END\n$$;\n");
        manager
            .get_connection()
            .execute_unprepared(&norm)
            .await
            .map_err(|e| DbErr::Custom(format!("时间戳列改型执行失败: {e}")))?;

        // 2) 收尾自证：改型后每列 data_type 必为 timestamp with time zone，否则中止。
        let mut verify = String::from("DO $$\nDECLARE\n    v_dt TEXT;\nBEGIN\n");
        for (table, column) in COLS {
            verify.push_str(&format!(
                "    SELECT data_type INTO v_dt FROM information_schema.columns\n\
                 \x20     WHERE table_schema = 'public' AND table_name = '{table}' AND column_name = '{column}';\n\
                 \x20   IF v_dt IS NULL OR v_dt <> 'timestamp with time zone' THEN\n\
                 \x20     RAISE EXCEPTION '收尾自证失败：{table}.{column} data_type = %（期望 timestamp with time zone），TIMESTAMPTZ 未真实生效，中止', v_dt;\n\
                 \x20   END IF;\n",
            ));
        }
        verify.push_str("END\n$$;\n");
        manager.get_connection().execute_unprepared(&verify).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 对称回退：TIMESTAMPTZ → 不带时区 TIMESTAMP（AT TIME ZONE 'UTC' 取 UTC 墙上时间），
        // 已是 naive 则跳过，保证 down 可安全重跑。
        let mut revert = String::from("DO $$\nDECLARE\n    v_dt TEXT;\nBEGIN\n");
        for (table, column) in COLS {
            revert.push_str(&format!(
                "    SELECT data_type INTO v_dt FROM information_schema.columns\n\
                 \x20     WHERE table_schema = 'public' AND table_name = '{table}' AND column_name = '{column}';\n\
                 \x20   IF v_dt IS NULL THEN\n\
                 \x20     RAISE EXCEPTION 'information_schema 中找不到 {table}.{column}，与建表迁移漂移，中止';\n\
                 \x20   END IF;\n\
                 \x20   IF v_dt = 'timestamp without time zone' THEN\n\
                 \x20     RAISE NOTICE '{table}.{column} 已是 timestamp without time zone，跳过回退（幂等重跑安全）';\n\
                 \x20   ELSE\n\
                 \x20     ALTER TABLE \"{table}\" ALTER COLUMN \"{column}\" TYPE TIMESTAMP USING \"{column}\" AT TIME ZONE 'UTC';\n\
                 \x20   END IF;\n",
            ));
        }
        revert.push_str("END\n$$;\n");
        manager.get_connection().execute_unprepared(&revert).await?;

        Ok(())
    }
}

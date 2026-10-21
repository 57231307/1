//! 状态/取值域词表的 DB CHECK 约束补齐迁移（九面状态列 + 质检类型/结论 + 流程定义状态）
//!
//! 将应用侧权威词表（models/status 各域 ALL）钉入数据库，阻止任何绕过服务层入参校验
//! 的写入通道产生词表外脏值；每列的 CHECK 取值集与对应词表逐元素相等，等值关系由
//! 契约测试 contract_state_vocab_check_db_channel_test 以「同读两侧原文」的方式锁定，
//! 两侧任一侧单独增删值都判红（口径与 export_inspection_vocab_check 先例一致）。
//!
//! 覆盖列（表.列 ← 权威词表）：
//! - sales_orders.status ← sales_order::ALL（小写八值）
//! - purchase_orders.order_status ← purchase_order 常量组（大写九值，列可空）
//! - purchase_receipt.receipt_status ← purchase_receipt 常量组（大写三值）
//! - inventory_transfers.status ← inventory_transfer::ALL（小写五值，列可空）
//! - inventory_counts.status ← inventory_count 常量组（小写三值，列可空）
//! - inventory_stocks.stock_status ← inventory_stock_status::ALL（中文三值，列可空）
//! - inventory_stocks.quality_status ← inventory_stock_quality_status::ALL（中文三值，列可空）
//! - greige_fabric.status ← greige_fabric_status::ALL（中文两值，列可空）
//! - logistics_waybills.status ← logistics_waybill::ALL（大写三值，列可空无默认）
//! - quality_inspection_standards.inspection_type ← quality_inspection_type::ALL
//! - quality_inspection_records.inspection_type ← quality_inspection_type::ALL
//! - quality_inspection_records.inspection_result ← quality_inspection_result::ALL
//!   （中文三值，列可空：权威词表 models/status/quality_dyeing.rs 声明的定义域就是本列；
//!   m0005 遗留的 result 列实体不映射、无任何写入方，不钉）
//! - bpm_process_definition.status ← bpm_process_definition_status::ALL（大写三值）
//!
//! 存量守卫（fail-visible，严禁 UPDATE 造值洗数据）：加约束前逐列统计词表外残留行数，
//! 非 0 即点名样例值并中止迁移；可空列的 NULL 属既有合法形态（收窄可空性另有专域
//! ddl_nullability_tighten，不在本迁移职责内），守卫与 CHECK 都对 NULL 放行、
//! 只拦词表外非空值。NOT NULL 列的 CHECK 不留 NULL 放行。
//!
//! 列默认值与词表同源：加 CHECK 前必须把「建表遗留的越界 DEFAULT」收敛到权威词表值，
//! 否则任何省略该列的写入（含 SeaORM ActiveModel 未显式赋值的默认建单路径）都会落
//! DEFAULT 越界值撞 CHECK 冒裸 23514——这是 CHECK 自身的写口责任，不属存量洗数据。
//! 本迁移唯一命中此形的是 `greige_fabric.status`：建表 system/m0002:26 遗留
//! `DEFAULT 'available'`，而权威词表 `greige_fabric_status::ALL` 是中文 `在库/已出库`，
//! 应用建单默认写 `在库`（见 handlers/greige_fabric_handler.rs create 的
//! `None => greige_fabric_status::IN_STOCK`）。故在施加 CHECK 前幂等重申
//! `SET DEFAULT '在库'`，使 DB 默认值与权威词表、应用建单默认三者同源（口径同
//! price_vocab_check 的 `SET DEFAULT 'pending'`；这是 DEFAULT 子句收敛，非 UPDATE 造值，
//! 不触碰本文件契约测试锁定的「禁止 UPDATE 清洗」接线形态）。存量库遗留的
//! `available` 行不在此洗，交由存量守卫 fail-visible 点名，业务核实后更正再重跑。
//!
//! 幂等：每列先 DROP CONSTRAINT IF EXISTS 再施加，可安全重跑。注册在全链尾：
//! 所有建表/加列/默认值收敛域（system→business→sales_crm→production→finance→v15→…）
//! 先行完成后才施加 CHECK，全新库与存量库均不违反 CHECK。

use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &'static str {
        "m_status_vocab_check"
    }
}

/// 词表 SQL 字面量清单（与各自权威词表 ALL/常量组逐字符一致；守卫、CHECK、列注释三处
/// 共用同一份，禁止另写第二套）。

const SALES_ORDER_STATUS_ALLOWED: &str =
    "'draft','pending','approved','partial_shipped','shipped','completed','cancelled','rejected'";
const PURCHASE_ORDER_STATUS_ALLOWED: &str = "'DRAFT','PENDING_APPROVAL','SUBMITTED','APPROVED','REJECTED','CLOSED','CANCELLED','COMPLETED','PARTIAL_RECEIVED'";
const PURCHASE_RECEIPT_STATUS_ALLOWED: &str = "'DRAFT','CONFIRMED','COMPLETED'";
const INVENTORY_TRANSFER_STATUS_ALLOWED: &str =
    "'pending','approved','rejected','shipped','completed'";
const INVENTORY_COUNT_STATUS_ALLOWED: &str = "'pending','in_review','completed'";
const INVENTORY_STOCK_STATUS_ALLOWED: &str = "'正常','报废','已删除'";
const INVENTORY_STOCK_QUALITY_ALLOWED: &str = "'合格','待检','不合格'";
const GREIGE_FABRIC_STATUS_ALLOWED: &str = "'在库','已出库'";
const LOGISTICS_WAYBILL_STATUS_ALLOWED: &str = "'IN_TRANSIT','DELIVERED','SIGNED'";
const QI_TYPE_ALLOWED: &str = "'incoming','process','finished','outgoing','outsourcing_receipt'";
const QI_RECORD_RESULT_ALLOWED: &str = "'待检','合格','不合格'";
const BPM_DEFINITION_STATUS_ALLOWED: &str = "'DRAFT','ACTIVE','INACTIVE'";

/// 一列的约束配方：表、列、约束名、词表 SQL、列是否 NOT NULL（NOT NULL 列的 CHECK
/// 不留 NULL 放行）。
struct ColumnCheck<'a> {
    table: &'a str,
    column: &'a str,
    constraint: &'a str,
    allowed: &'a str,
    not_null: bool,
}

const COLUMN_CHECKS: &[ColumnCheck] = &[
    ColumnCheck {
        table: "sales_orders",
        column: "status",
        constraint: "chk_sales_orders_status",
        allowed: SALES_ORDER_STATUS_ALLOWED,
        not_null: true,
    },
    ColumnCheck {
        table: "purchase_orders",
        column: "order_status",
        constraint: "chk_purchase_orders_order_status",
        allowed: PURCHASE_ORDER_STATUS_ALLOWED,
        not_null: false,
    },
    ColumnCheck {
        table: "purchase_receipt",
        column: "receipt_status",
        constraint: "chk_purchase_receipt_receipt_status",
        allowed: PURCHASE_RECEIPT_STATUS_ALLOWED,
        not_null: true,
    },
    ColumnCheck {
        table: "inventory_transfers",
        column: "status",
        constraint: "chk_inventory_transfers_status",
        allowed: INVENTORY_TRANSFER_STATUS_ALLOWED,
        not_null: false,
    },
    ColumnCheck {
        table: "inventory_counts",
        column: "status",
        constraint: "chk_inventory_counts_status",
        allowed: INVENTORY_COUNT_STATUS_ALLOWED,
        not_null: false,
    },
    ColumnCheck {
        table: "inventory_stocks",
        column: "stock_status",
        constraint: "chk_inventory_stocks_stock_status",
        allowed: INVENTORY_STOCK_STATUS_ALLOWED,
        not_null: false,
    },
    ColumnCheck {
        table: "inventory_stocks",
        column: "quality_status",
        constraint: "chk_inventory_stocks_quality_status",
        allowed: INVENTORY_STOCK_QUALITY_ALLOWED,
        not_null: false,
    },
    ColumnCheck {
        table: "greige_fabric",
        column: "status",
        constraint: "chk_greige_fabric_status",
        allowed: GREIGE_FABRIC_STATUS_ALLOWED,
        not_null: false,
    },
    ColumnCheck {
        table: "logistics_waybills",
        column: "status",
        constraint: "chk_logistics_waybills_status",
        allowed: LOGISTICS_WAYBILL_STATUS_ALLOWED,
        not_null: false,
    },
    ColumnCheck {
        table: "quality_inspection_standards",
        column: "inspection_type",
        constraint: "chk_quality_inspection_standards_inspection_type",
        allowed: QI_TYPE_ALLOWED,
        not_null: true,
    },
    ColumnCheck {
        table: "quality_inspection_records",
        column: "inspection_type",
        constraint: "chk_quality_inspection_records_inspection_type",
        allowed: QI_TYPE_ALLOWED,
        not_null: true,
    },
    ColumnCheck {
        table: "quality_inspection_records",
        column: "inspection_result",
        constraint: "chk_quality_inspection_records_inspection_result",
        allowed: QI_RECORD_RESULT_ALLOWED,
        // 结论列真实形态：v15 追加时即可空且无默认（v15/mod.rs:3844），历史行存在
        // NULL；权威词表 models/status/quality_dyeing.rs::quality_inspection_result
        // 声明的定义域就是本列（不是 m0005 遗留、实体不映射的 result 列）。守卫与
        // CHECK 放行 NULL、只拦词表外非空值——写入口径以本列为准，钉错列等于真实
        // 旁路门未关、正常写入反被死列的 NOT NULL 语义判死。
        not_null: false,
    },
    ColumnCheck {
        table: "bpm_process_definition",
        column: "status",
        constraint: "chk_bpm_process_definition_status",
        allowed: BPM_DEFINITION_STATUS_ALLOWED,
        not_null: true,
    },
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 0) 列默认值收敛：加 CHECK 前先把建表遗留的越界 DEFAULT 改为权威词表值，
        //    令「省略该列的写入」不再落 DEFAULT 撞新 CHECK。唯一命中的是
        //    greige_fabric.status（m0002 遗留 DEFAULT 'available'，权威词表为中文，
        //    应用建单默认写「在库」）。此句只改 DEFAULT 子句、不动任何数据行，
        //    与「禁止 UPDATE 造值」接线锁不冲突；幂等可重跑。
        let normalize_defaults = r#"
ALTER TABLE "greige_fabric" ALTER COLUMN "status" SET DEFAULT '在库';
"#;
        manager
            .get_connection()
            .execute_unprepared(normalize_defaults)
            .await?;

        for spec in COLUMN_CHECKS {
            // 1) 幂等前置：删旧约束（重跑/半态库不报错）。
            let drop_old = format!(
                "ALTER TABLE \"{}\" DROP CONSTRAINT IF EXISTS \"{}\";",
                spec.table, spec.constraint
            );
            manager
                .get_connection()
                .execute_unprepared(&drop_old)
                .await?;

            // 2) 存量守卫（fail-visible）：统计词表外残留行数并采样例值；非 0 即点名中止。
            //    可空列的 NULL 为既有合法形态，不计入脏值；NOT NULL 列 NULL 即脏。
            let dirty_pred = if spec.not_null {
                format!(
                    "\"{col}\" IS NULL OR \"{col}\" NOT IN ({allowed})",
                    col = spec.column,
                    allowed = spec.allowed
                )
            } else {
                format!(
                    "\"{col}\" IS NOT NULL AND \"{col}\" NOT IN ({allowed})",
                    col = spec.column,
                    allowed = spec.allowed
                )
            };
            let sample_expr = if spec.not_null {
                format!("COALESCE(\"{col}\", '<NULL>')", col = spec.column)
            } else {
                format!("\"{col}\"", col = spec.column)
            };
            let guard = format!(
                r#"
DO $$
DECLARE
    bad_rows INTEGER;
    samples  TEXT;
BEGIN
    SELECT COUNT(*) INTO bad_rows FROM "{table}"
      WHERE {dirty};
    IF bad_rows = 0 THEN
        RAISE NOTICE 'm_status_vocab_check：{table}.{col} 无词表外存量行，可安全施加 CHECK {constraint}。';
    ELSE
        SELECT string_agg(sample, ', ') INTO samples
          FROM (SELECT DISTINCT {sample} AS sample
                  FROM "{table}"
                 WHERE {dirty}
                 LIMIT 10) s;
        RAISE EXCEPTION 'm_status_vocab_check：{table}.{col} 存在 % 行词表外取值（样例值：%），拒绝施加 CHECK {constraint}。', bad_rows, samples
            USING HINT = '本迁移拒绝 UPDATE 造值：请按 models/status 对应域权威词表逐行核实更正存量，更正后重跑迁移；若脏值来自旁路写入，先修写入方，不要在迁移里圆场。';
    END IF;
END;
$$;
"#,
                table = spec.table,
                col = spec.column,
                constraint = spec.constraint,
                dirty = dirty_pred,
                sample = sample_expr,
            );
            manager.get_connection().execute_unprepared(&guard).await?;

            // 3) 施加 CHECK 并以列注释固化权威词表指向；可空列 CHECK 放行 NULL。
            let check_expr = if spec.not_null {
                format!(
                    "\"{col}\" IN ({allowed})",
                    col = spec.column,
                    allowed = spec.allowed
                )
            } else {
                format!(
                    "\"{col}\" IS NULL OR \"{col}\" IN ({allowed})",
                    col = spec.column,
                    allowed = spec.allowed
                )
            };
            let apply = format!(
                "ALTER TABLE \"{table}\" ADD CONSTRAINT \"{constraint}\" CHECK ({check_expr});\nCOMMENT ON COLUMN \"{table}\".\"{col}\" IS '权威词表见 backend/src/models/status 对应域，取值集与 CHECK {constraint} 逐项相等（契约锁 contract_state_vocab_check_db_channel_test）';",
                table = spec.table,
                col = spec.column,
                constraint = spec.constraint,
                check_expr = check_expr,
            );
            manager.get_connection().execute_unprepared(&apply).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚仅移除约束，不回退数据（口径同 export_inspection_vocab_check 的 down）。
        for spec in COLUMN_CHECKS {
            let sql = format!(
                "ALTER TABLE \"{}\" DROP CONSTRAINT IF EXISTS \"{}\";",
                spec.table, spec.constraint
            );
            manager.get_connection().execute_unprepared(&sql).await?;
        }
        Ok(())
    }
}

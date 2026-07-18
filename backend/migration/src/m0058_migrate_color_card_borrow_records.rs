use sea_orm_migration::prelude::*;

// Batch 477 P0-F13 修复：迁移旧 color_card_borrow_records 数据到 color_card_issues
//
// 旧表 color_card_borrow_records 在 migration 20260617000008_create_color_card_borrow_records
// 创建，状态枚举为 borrowed/returned/lost/damaged（缺 cancelled）。
// 新表 color_card_issues 在本批次 m0057_create_color_card_issues 创建。
//
// 字段映射规则：
//   color_card_id         -> color_card_id（直接映射）
//   customer_id           -> customer_id（直接映射）
//   borrowed_by           -> issued_by
//   borrowed_at           -> issued_at
//   expected_return_at    -> expected_return_date（TIMESTAMPTZ → DATE，强转）
//   actual_return_at      -> actual_return_date（TIMESTAMPTZ → DATE，强转）
//   notes                 -> remark
//   purpose               -> purpose（直接映射）
//   compensation_amount  -> compensation_amount（直接映射）
//   status='borrowed'     -> status='issued'（状态映射）
//   status='returned'     -> status='returned'（直接映射）
//   status='lost'         -> status='lost'（直接映射）
//   status='damaged'      -> status='damaged'（直接映射）
//   缺 returned_by        -> 由 borrowed_by 推断（归还操作员=借出人）
//   缺 issue_qty          -> 默认 1（旧表无数量概念，色卡单张发放）
//   缺 dye_lot_no         -> NULL（旧表无染色批号字段）
//   created_at/updated_at -> 直接映射
//   is_deleted            -> FALSE（旧表无软删除）
//
// 旧表处理策略：保留不重命名，应用层不再读写（避免破坏 Rust migration m0029 链路）
//
// 设计依据：V15 审计报告 类九 P0-F13
// 关联文件：models/color_card_issue.rs / migrations/20260617000008_create_color_card_borrow_records

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 仅当旧表存在时执行迁移（幂等保护，避免重复执行报错）
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DO $$
                BEGIN
                    IF EXISTS (
                        SELECT 1 FROM information_schema.tables
                        WHERE table_name = 'color_card_borrow_records'
                    ) THEN
                        -- 迁移旧表数据到 color_card_issues
                        -- ON CONFLICT DO NOTHING 保证可重复执行（按 id 幂等）
                        INSERT INTO color_card_issues (
                            id, color_card_id, customer_id, issue_qty, issued_by, issued_at,
                            expected_return_date, actual_return_date, status, purpose, remark,
                            compensation_amount, returned_by, dye_lot_no, created_at, updated_at, is_deleted
                        )
                        SELECT
                            b.id,
                            b.color_card_id,
                            b.customer_id,
                            1 AS issue_qty,
                            b.borrowed_by AS issued_by,
                            b.borrowed_at AS issued_at,
                            b.expected_return_at::date AS expected_return_date,
                            b.actual_return_at::date AS actual_return_date,
                            CASE b.status
                                WHEN 'borrowed' THEN 'issued'
                                ELSE b.status  -- returned/lost/damaged 直接映射
                            END AS status,
                            b.purpose,
                            b.notes AS remark,
                            b.compensation_amount,
                            b.borrowed_by AS returned_by,  -- 旧表无归还人字段，由借出人推断
                            NULL::varchar AS dye_lot_no,
                            b.created_at,
                            b.updated_at,
                            FALSE AS is_deleted
                        FROM color_card_borrow_records b
                        WHERE NOT EXISTS (
                            SELECT 1 FROM color_card_issues i WHERE i.id = b.id
                        );

                        -- 同步 color_card_issues_id_seq 到迁移后最大 id，避免后续 INSERT 主键冲突
                        SELECT setval(
                            pg_get_serial_sequence('color_card_issues', 'id'),
                            COALESCE((SELECT MAX(id) FROM color_card_issues), 1),
                            true
                        );
                    END IF;
                END $$;
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 回滚仅删除迁移自旧表的记录（保留新建记录，避免数据丢失）
        // 仅删除 id 在旧表中存在的记录
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DO $$
                BEGIN
                    IF EXISTS (
                        SELECT 1 FROM information_schema.tables
                        WHERE table_name = 'color_card_borrow_records'
                    ) THEN
                        DELETE FROM color_card_issues i
                        WHERE i.id IN (SELECT id FROM color_card_borrow_records);
                    END IF;
                END $$;
                "#,
            )
            .await?;
        Ok(())
    }
}

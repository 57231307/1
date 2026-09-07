//! inventory_piece 匹号领域扩展（匹号领域模型一期，设计见 docs/piece-number-domain-design.md）
//!
//! 领域规则（用户确认）：
//! - 生产报工逐匹登记生产匹号 + 机台号（胚布无缸号，机台号仅存在于生产环节）
//! - 染色完成后生成染色匹号 + 缸号，染色匹号贯穿入库/外发/销售/出库/对账
//! - 生产匹号与染色匹号不通用：生产匹号仅限生产环节
//! - 净布工艺外发免缸号

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- 匹类型：greige=生产匹（胚布，生产环节专用）；dyed=染色匹（入库/外发/销售/出库/对账）
-- 存量数据全部为染色匹语义
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS piece_type VARCHAR(20) NOT NULL DEFAULT 'dyed';
-- 机台号：仅生产匹（胚布织造机台）
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS machine_no VARCHAR(100);

-- 生产匹无缸号：dye_lot_id 放宽为可空
ALTER TABLE inventory_piece ALTER COLUMN dye_lot_id DROP NOT NULL;

-- 机台号记录开机人（什么人开的机器）
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS machine_operator VARCHAR(100);
-- 入库时间（何时入的胚布仓库/成品仓库）
ALTER TABLE inventory_piece ADD COLUMN IF NOT EXISTS warehouse_in_at TIMESTAMPTZ;

-- 仓库类型约束：胚布仓(greige)只能存放未染色/未做工艺的胚布；
-- 成品仓(finished)只能存放染色/工艺后的成品；NULL/other 不校验（兼容存量仓库）
ALTER TABLE warehouses ADD COLUMN IF NOT EXISTS warehouse_type VARCHAR(20);
CREATE INDEX IF NOT EXISTS idx_warehouses_type ON warehouses (warehouse_type);

-- 唯一约束调整：原 (dye_lot_id, piece_no) 组合唯一按匹类型拆分为部分唯一索引
ALTER TABLE inventory_piece DROP CONSTRAINT IF EXISTS uniq_inventory_piece_dye_lot_piece;
CREATE UNIQUE INDEX IF NOT EXISTS uniq_greige_piece_no
    ON inventory_piece (piece_no) WHERE piece_type = 'greige';
CREATE UNIQUE INDEX IF NOT EXISTS uniq_dyed_piece_no
    ON inventory_piece (dye_lot_id, piece_no) WHERE piece_type = 'dyed' AND dye_lot_id IS NOT NULL;
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
DROP INDEX IF EXISTS uniq_dyed_piece_no;
DROP INDEX IF EXISTS uniq_greige_piece_no;
ALTER TABLE inventory_piece DROP CONSTRAINT IF EXISTS uniq_inventory_piece_dye_lot_piece;
-- 回滚前置：生产匹（无缸号）数据无法满足 dye_lot_id 非空，仅清理后才能恢复
DELETE FROM inventory_piece WHERE piece_type = 'greige';
ALTER TABLE inventory_piece ALTER COLUMN dye_lot_id SET NOT NULL;
ALTER TABLE inventory_piece DROP COLUMN IF EXISTS machine_operator;
ALTER TABLE inventory_piece DROP COLUMN IF EXISTS warehouse_in_at;
ALTER TABLE inventory_piece DROP COLUMN IF EXISTS machine_no;
ALTER TABLE inventory_piece DROP COLUMN IF EXISTS piece_type;
ALTER TABLE warehouses DROP COLUMN IF EXISTS warehouse_type;
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}

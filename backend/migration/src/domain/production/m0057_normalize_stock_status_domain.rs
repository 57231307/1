//! 库存台账状态、质量状态与入库单检验状态的跨域字面量归一
//!
//! `inventory_stocks.stock_status` 的取值域是中文主数据（正常/报废/已删除），
//! `quality_status` 是（合格/待检/不合格）。历史上 `POST /inventory/stock` 这一条
//! 建单路径把两列分别写成 `active` 与 `qualified`（批次 213 只是把字面量换成了
//! `master_data::ACTIVE` 常量——常量属于另一张表的状态域，写入值依旧越界），
//! 于是这批库存：
//! - 对可用量、可出库、缺料预警等一律按「正常 + 合格」过滤的查询永久不可见（账上有货、界面/出库无货）；
//! - 反而只有仪表盘那几处同样按错误词表过滤的统计能看见它们（错误相互抵消成"看起来正常"）。
//! 写入侧与读取侧已统一到本列常量，本迁移把存量行一并归一，避免同一缺陷在历史数据上继续生效。
//!
//! 同一批还归一 `purchase_receipt.inspection_status`：该列取值域是大写码（PENDING/PASSED/REJECTED），
//! 而质检结论回写路径把中文结论（待检/合格/不合格）原样复制进来，使这些入库单在按大写码判断的
//! 读取方眼里等于"从未检验"。`purchase_receipt` 表在 business 域建立，本迁移所在 production 域
//! 晚于它，执行顺序安全。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 只改写本列取值域之外、且能确定来源拼写的值；大小写变体一并覆盖。
        // 其余值（含报废/待检等合法值）保持原样，不做批量猜测式重写。
        let sql = r#"
UPDATE "inventory_stocks"
   SET "stock_status" = '正常'
 WHERE lower("stock_status") IN ('active', 'normal');

UPDATE "inventory_stocks"
   SET "quality_status" = '合格'
 WHERE lower("quality_status") IN ('qualified', 'pass');

UPDATE "purchase_receipt"
   SET "inspection_status" = CASE "inspection_status"
       WHEN '待检' THEN 'PENDING'
       WHEN '合格' THEN 'PASSED'
       WHEN '不合格' THEN 'REJECTED'
       ELSE "inspection_status"
   END
 WHERE "inspection_status" IN ('待检', '合格', '不合格');
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // 归一后无法区分哪些行原本写的是 active/qualified，回滚不可逆；
        // 反向迁移显式不做任何写操作，避免把正确值改回错误值（与 m0056 同策略）。
        Ok(())
    }
}

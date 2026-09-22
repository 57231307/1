//! 委外收回单质检结论取值规范化（`outsourcing_receipt.quality_status`）
//!
//! 该列历史上同时存在四套写法：收回单界面提交 `qualified/concession/unqualified`，
//! E2E 用例提交 `passed`（染化料来料检验域的字面量），模型注释又写 `passed/failed`，
//! 库存质量状态域的中文「合格/不合格」也曾被当成本列取值写进来。
//! 而确认回仓时只有字面量 `qualified` 会被判为接收，其余一律落到「不合格」分支，
//! 于是 `passed` 的收回单在确认时生成一条「不合格」质检记录且合格数量记 0，
//! 与实际收回等级（如 A 级）自相矛盾，且全程只有一条 warn。
//!
//! 写入侧已统一为 `outsourcing_receipt_quality_status` 常量并在入口校验取值域，
//! 本迁移把存量行归一，避免同一缺陷在历史数据上继续生效；
//! 结论为空（NULL）的行不代表任何判定，统一改为 `pending`（待检），
//! 由确认侧要求补录结论，而不是被默认成合格。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
UPDATE "outsourcing_receipt"
   SET "quality_status" = CASE "quality_status"
        WHEN 'passed'  THEN 'qualified'
        WHEN 'failed'  THEN 'unqualified'
        WHEN '合格'    THEN 'qualified'
        WHEN '不合格'  THEN 'unqualified'
        WHEN '待检'    THEN 'pending'
        ELSE "quality_status"
   END
 WHERE "quality_status" IN ('passed', 'failed', '合格', '不合格', '待检');

UPDATE "outsourcing_receipt"
   SET "quality_status" = 'pending'
 WHERE "quality_status" IS NULL;
"#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // 归一后无法还原某行原本是 passed 还是 qualified，也无法还原哪些行的结论本为 NULL；
        // 反向迁移显式不做任何写操作，避免把正确值改回错误值。
        Ok(())
    }
}

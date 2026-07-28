//! V15 P1 迁移整合：执行所有 V15 P1 批次的 SQL 迁移文件
//!
//! 整合方案（参考 m0044_integrate_unreferenced_migrations）：
//! 用 include_str! 编译期嵌入 V15 P1 SQL 迁移文件，按顺序执行。
//! 所有 SQL 均使用 IF NOT EXISTS / IF EXISTS，保证幂等可重入。
//!
//! 包含的 V15 P1 SQL 迁移：
//! - 051_v15_p1_batch_trace_log_extend：batch_trace_log 字段扩展（dye_lot_no/color_no/product_id/from_status/to_status）
//! - 052_v15_p1_fabric_physical_test_record：面料检验物理指标建模
//! - 053_v15_p1_dye_batch_state_machine_on_hold_failed：缸号状态机 OnHold+Failed
//! - 054_v15_p1_product_compliance_fields：产品模型合规字段（执行标准/厂名/厂址/产品等级）
//! - 055_v15_p1_wage_record_detail_overtime：工资明细加班工时字段（《劳动法》第 44 条）

use sea_orm_migration::prelude::*;

/// V15 P1 SQL 迁移条目：(名称, SQL 内容)
const V15_P1_MIGRATIONS: &[(&str, &str)] = &[
    (
        "051_v15_p1_batch_trace_log_extend",
        include_str!("../../../database/migration/051_v15_p1_batch_trace_log_extend.sql"),
    ),
    (
        "052_v15_p1_fabric_physical_test_record",
        include_str!("../../../database/migration/052_v15_p1_fabric_physical_test_record.sql"),
    ),
    (
        "053_v15_p1_dye_batch_state_machine_on_hold_failed",
        include_str!(
            "../../../database/migration/053_v15_p1_dye_batch_state_machine_on_hold_failed.sql"
        ),
    ),
    (
        "054_v15_p1_product_compliance_fields",
        include_str!("../../../database/migration/054_v15_p1_product_compliance_fields.sql"),
    ),
    (
        "055_v15_p1_wage_record_detail_overtime",
        include_str!("../../../database/migration/055_v15_p1_wage_record_detail_overtime.sql"),
    ),
];

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for (name, sql) in V15_P1_MIGRATIONS {
            if !sql.trim().is_empty() {
                db.execute_unprepared(sql)
                    .await
                    .map_err(|e| DbErr::Custom(format!("执行 V15 P1 迁移 {} 失败: {}", name, e)))?;
            }
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        // V15 P1 迁移为增量字段/表，down 操作按业务需要不实现（保留数据）
        Ok(())
    }
}

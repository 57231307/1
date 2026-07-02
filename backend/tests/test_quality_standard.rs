//! 质量标准审批流程测试（P1 批 65 测试资产清理）
//!
//! 原文件含 9 个伪测试（测数组断言 / 测 String::is_empty / 测 NaiveDate 比较 /
//! 测本地 can_approve/can_publish 函数），均不调用 QualityStandardService 任何方法，
//! 已于批次 65 删除。其中本地 can_approve 的逻辑（仅允许 draft）与真实 Service
//! 的 approve_standard（允许 draft 或 rejected）不一致，属于错误的伪测试。
//!
//! QualityStandardService 的所有业务方法（get_standards_list / create_standard /
//! approve_standard / publish_standard 等）均需要数据库连接，
//! 无法在无 DB 的集成测试环境中调用，完整业务流程由 CI 集成环境执行。
//!
//! 保留下来的真实测试：QualityStandardService 构造函数签名编译期断言。

use bingxi_backend::services::quality_standard_service::QualityStandardService;
use std::sync::Arc;

/// 验证 QualityStandardService 构造函数签名：fn(Arc<DatabaseConnection>) -> QualityStandardService
#[test]
fn test_quality_standard_service_constructor_signature() {
    let _: fn(Arc<sea_orm::DatabaseConnection>) -> QualityStandardService =
        QualityStandardService::new;
}

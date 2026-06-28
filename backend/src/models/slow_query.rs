#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
// 当前抑制理由：模型字段由 SeaORM DeriveEntityModel 派生宏使用，不能手工逐字段标注。

//! 慢查询日志模型（P13 批 1 B-慢查询审计）
//!
//! 数据来源：pg_stat_statements 视图 + 后台定时采集任务（slow_query_collector）
//! 用途：前端慢查询审计页面（/system/slow-query）+ 运维 SQL 性能优化
//!
//! 表名：`slow_query_log`（迁移 m0025）
//! 关键索引：idx_slow_query_captured / idx_slow_query_exec_time / idx_slow_query_tenant

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 慢查询日志 Entity
///
/// 字段命名规范：均用 snake_case，符合 SeaORM 默认列名
/// 字段类型选择：
/// - query_text: TEXT（SQL 文本可能很长）
/// - execution_time_ms: DOUBLE PRECISION（pg_stat_statements.mean_exec_time 是 float8）
/// - calls / rows_examined: BIGINT（pg_stat_statements.calls / rows 是 int8）
/// - database_name: VARCHAR(128)（DB 名最长 ~64 字符，128 留余量）
/// - captured_at: TIMESTAMPTZ（带时区，便于跨时区分析）
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize, Default)]
#[sea_orm(table_name = "slow_query_log")]
pub struct Model {
    /// 日志 ID（主键自增）
    #[sea_orm(primary_key)]
    pub id: i64,

    /// SQL 文本（来自 pg_stat_statements.query）
    pub query_text: String,

    /// 平均执行时间（毫秒，pg_stat_statements.mean_exec_time）
    pub execution_time_ms: f64,

    /// 调用次数（pg_stat_statements.calls）
    pub calls: i64,

    /// 平均扫描行数（pg_stat_statements.rows）
    pub rows_examined: i64,

    /// 数据库名（系统级元数据；多库部署时区分来源）
    pub database_name: Option<String>,

    /// 采集时间
    pub captured_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// 列表查询 DTO（用于 handler 入参 / 出参）
///
/// 注意：DTO 与 Model 解耦，避免数据库 schema 变更直接污染 API 契约
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowQueryDto {
    /// 日志 ID
    pub id: i64,
    /// SQL 文本
    pub query_text: String,
    /// 平均执行时间（毫秒）
    pub execution_time_ms: f64,
    /// 调用次数
    pub calls: i64,
    /// 平均扫描行数
    pub rows_examined: i64,
    /// 数据库名
    pub database_name: Option<String>,
    /// 采集时间（ISO8601 字符串）
    pub captured_at: String,
}

impl From<Model> for SlowQueryDto {
    fn from(m: Model) -> Self {
        Self {
            id: m.id,
            query_text: m.query_text,
            execution_time_ms: m.execution_time_ms,
            calls: m.calls,
            rows_examined: m.rows_examined,
            database_name: m.database_name,
            captured_at: m.captured_at.to_rfc3339(),
        }
    }
}

/// 慢查询聚合统计 DTO（按 query_text 分组）
///
/// 用于 `/api/v1/erp/slow-queries/stats` 接口的 TOP 10 列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlowQueryStatDto {
    /// SQL 文本（去重后）
    pub query_text: String,
    /// 该 SQL 的最大平均执行时间（毫秒）
    pub max_exec_time_ms: f64,
    /// 该 SQL 的累计调用次数
    pub total_calls: i64,
    /// 该 SQL 的累计平均扫描行数
    pub avg_rows: f64,
    /// 该 SQL 被采集到的次数
    pub sample_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 模型字段默认值验证
    #[test]
    fn test_model_default_values() {
        let m = Model::default();
        assert_eq!(m.id, 0);
        assert_eq!(m.query_text, String::new());
        assert_eq!(m.execution_time_ms, 0.0);
        assert_eq!(m.calls, 0);
        assert_eq!(m.rows_examined, 0);
        assert!(m.database_name.is_none());
    }

    /// Model → DTO 转换正确性
    #[test]
    fn test_model_to_dto_conversion() {
        let captured = chrono::Utc::now();
        let m = Model {
            id: 100,
            query_text: "SELECT * FROM users".to_string(),
            execution_time_ms: 250.5,
            calls: 42,
            rows_examined: 1024,
            database_name: Some("bingxi_erp".to_string()),
            captured_at: captured,
        };
        let dto: SlowQueryDto = m.into();
        assert_eq!(dto.id, 100);
        assert_eq!(dto.query_text, "SELECT * FROM users");
        assert_eq!(dto.execution_time_ms, 250.5);
        assert_eq!(dto.calls, 42);
        assert_eq!(dto.rows_examined, 1024);
        assert_eq!(dto.database_name, Some("bingxi_erp".to_string()));
        // captured_at 已转成 RFC3339 字符串（包含时区偏移）
        assert!(dto.captured_at.contains("T"));
        assert!(dto.captured_at.contains("+") || dto.captured_at.ends_with("Z"));
    }

    /// DTO 序列化/反序列化（验证 JSON 字段命名）
    #[test]
    fn test_dto_serialize() {
        let dto = SlowQueryDto {
            id: 1,
            query_text: "SELECT 1".to_string(),
            execution_time_ms: 100.0,
            calls: 1,
            rows_examined: 1,
            database_name: None,
            captured_at: "2026-06-18T10:00:00+00:00".to_string(),
        };
        let json = serde_json::to_string(&dto).expect("序列化应成功");
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"query_text\":\"SELECT 1\""));
        assert!(json.contains("\"execution_time_ms\":100.0"));
        assert!(json.contains("\"calls\":1"));
        // 验证可反序列化
        let round: SlowQueryDto = serde_json::from_str(&json).expect("反序列化应成功");
        assert_eq!(round.id, 1);
        assert_eq!(round.query_text, "SELECT 1");
    }
}

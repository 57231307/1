//! 集成测试公共夹具模块（P0-D11）
//!
//! 抽取自 3 个集成测试文件（tests/）中重复定义的 setup_test_db 函数。
//! 支持 TEST_DATABASE_URL 环境变量指定数据库，默认回退到 sqlite::memory:。
//! 集成测试使用方式：`mod common; use common::setup_test_db;`

use sea_orm::DatabaseConnection;

/// 创建测试用数据库连接
///
/// 优先使用 TEST_DATABASE_URL 环境变量（用于真实数据库测试），
/// 默认回退到 sqlite::memory:（快速单元测试）。
///
/// A.23 修复：sqlite 与生产 PostgreSQL 方言有保真度差距（JSONB/部分索引/DO 块/RLS）。
/// 回退到 sqlite 时输出警告，提示开发者设置 TEST_DATABASE_URL 指向本地 PG
/// （如 `postgres://user:pass@localhost:5432/bingxi_test`）以获得与 CI 一致的保真度。
/// CI 已通过 service container 用 PostgreSQL 16 运行测试，本地 sqlite 仅用于快速迭代。
pub async fn setup_test_db() -> DatabaseConnection {
    let db_url = std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        eprintln!(
            "⚠️  测试使用 sqlite::memory: 回退，与生产 PostgreSQL 方言有差距（JSONB/部分索引/DO 块/RLS）。\n\
             设置 TEST_DATABASE_URL 环境变量指向本地 PostgreSQL 以获得与 CI 一致的保真度：\n\
             export TEST_DATABASE_URL=postgres://user:pass@localhost:5432/bingxi_test"
        );
        "sqlite::memory:".to_string()
    });
    sea_orm::Database::connect(&db_url)
        .await
        .expect("测试夹具：数据库连接失败")
}

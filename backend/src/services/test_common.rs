//! 测试公共夹具模块（P0-D11）
//!
//! 抽取自 21 处重复定义的 setup_test_db 函数（tests/ 3 处 + src/services/ 18 处）。
//! 统一支持 TEST_DATABASE_URL 环境变量回退到 sqlite::memory:。

use sea_orm::DatabaseConnection;

/// 创建测试用数据库连接
///
/// 优先使用 TEST_DATABASE_URL 环境变量（用于真实数据库测试），
/// 默认回退到 sqlite::memory:（快速单元测试）。
pub async fn setup_test_db() -> DatabaseConnection {
    let db_url =
        std::env::var("TEST_DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
    sea_orm::Database::connect(&db_url)
        .await
        .expect("测试夹具：数据库连接失败")
}

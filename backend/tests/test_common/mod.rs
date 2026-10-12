//! 集成测试公共夹具（tests/ 侧入口）
//!
//! 实现只有一份：`bingxi_backend::services::test_common`。本模块只做再导出，
//! 让 `mod test_common; use test_common::setup_test_db;` 的既有写法继续成立，
//! 避免"tests/ 与 src/ 两套夹具、只有一套被改"的漂移。再导出清单按真实
//! 消费者收窄（`grep -rn 'test_common::' tests/` 可复核），src 内当前无夹具调用点。
//!
//! 语义要点（详见实现文件头注释）：
//! - `setup_test_db()` 必须连已迁移的 PostgreSQL，缺 `TEST_DATABASE_URL` 或指向
//! sqlite 直接 panic —— 静默回退 sqlite::memory: 是 CI 约 130 例方言解码
//!   红的根因，已彻底禁止。
//! - 每次调用先 `TRUNCATE` 业务表（保留迁移种子参照表），用例之间互不串库。
//! - 需要"空 schema"负前提交集用 `connect_empty_schema_db()`（`TEST_EMPTY_DATABASE_URL`）。

// 每个测试二进制各自 `mod test_common;` 只用到其中一部分入口，未用项会报 unused_imports；
// 在 shim 处统一放行（不是把实现变成死代码——实现只有 lib 那一份）。
#[allow(unused_imports, reason = "shim 再导出：各测试文件按需取用")]
pub use bingxi_backend::services::test_common::{
    connect_empty_schema_db, connect_live_db, setup_test_db,
};

//! 面料管理 - Rust 后端服务
//!
//! 本项目使用 SeaORM 1.0 + Axum + Tokio 技术栈
//! 数据库：PostgreSQL 18

pub mod cli;
pub mod config;
pub mod constants;
pub mod database;
pub mod handlers;
// 批次 105 修复：messaging/ 模块已删除
// 原因：messaging/ 是 P9-7 设计阶段的 trait + mock 占位模块，仅在自身测试中被引用，
// 无任何业务代码使用。P11-H2 已用 rskafka 完成真实 Kafka 集成（services/event_kafka.rs），
// 形成重复实现。根据用户新规则和 project_rules.md 第六节"死代码处理规范"删除。
pub mod middleware;
pub mod models;
pub mod observability;
pub mod routes;
pub mod search; // P9-8 Elasticsearch 集成
pub mod services;
pub mod utils;
pub mod websocket; // P3-2 WebSocket 实时通信模块
// P9-6 OpenTelemetry 一体化
pub mod telemetry;

pub use services::auth_service::AuthService;
pub use services::user_service::UserService;
pub use utils::app_state::AppState;
pub mod docs;

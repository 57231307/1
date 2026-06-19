//! 面料管理 - Rust 后端服务
//!
//! 本项目使用 SeaORM 1.0 + Axum + Tokio 技术栈
//! 数据库：PostgreSQL 18

pub mod cli;
pub mod config;
pub mod database;
pub mod handlers;
pub mod messaging; // P9-7 Kafka 集成
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

pub mod cache; // P12 批 1：Redis 缓存层

pub use services::auth_service::AuthService;
pub use services::user_service::UserService;
pub use utils::app_state::AppState;
pub mod docs;

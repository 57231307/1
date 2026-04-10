#![allow(warnings)]
//! 秉羲面料管理 - Rust 后端服务
//!
//! 本项目使用 SeaORM 1.0 + Axum + Tokio 技术栈
//! 数据库：PostgreSQL 18

pub mod config;
pub mod database;
pub mod grpc;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod routes;
pub mod services;
pub mod utils;

pub use services::auth_service::AuthService;
pub use services::user_service::UserService;
pub use utils::app_state::AppState;

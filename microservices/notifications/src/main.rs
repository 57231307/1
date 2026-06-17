// main.rs - notifications 微服务启动入口
// 启动流程：
// 1. 加载环境变量（DATABASE_URL, GRPC_PORT）
// 2. 初始化 tracing 日志
// 3. 连接 PostgreSQL
// 4. 启动 tonic gRPC server
// 5. 监听 SIGTERM/SIGINT 优雅关闭

use anyhow::{Context, Result};
use std::sync::Arc;
use tonic::transport::Server;

pub mod model;
pub mod repository;
pub mod service;

// 引入编译生成的 proto 代码
pub mod proto {
    tonic::include_proto!("notifications");
}

use crate::proto::notification_service_server::NotificationServiceServer;
use crate::service::NotificationServiceImpl;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 加载环境变量
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://erp:erp@localhost:5432/notifications_db".to_string());
    let grpc_port: u16 = std::env::var("GRPC_PORT")
        .unwrap_or_else(|_| "50056".to_string())
        .parse()
        .context("GRPC_PORT 解析失败")?;

    // 2. 初始化 tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("启动 notifications 微服务（端口 {}）", grpc_port);

    // 3. 连接数据库
    let pool = Arc::new(
        repository::create_pool(&database_url)
            .await
            .context("数据库连接失败")?,
    );
    tracing::info!("数据库连接成功");

    // 4. 构造 service
    let service = NotificationServiceImpl::new(pool);
    let addr = format!("0.0.0.0:{}", grpc_port).parse()?;

    // 5. 启动 server
    tracing::info!("gRPC server 监听 {}", addr);
    Server::builder()
        .add_service(NotificationServiceServer::new(service))
        .serve(addr)
        .await
        .context("gRPC server 启动失败")?;

    Ok(())
}

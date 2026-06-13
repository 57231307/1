use sea_orm::{Database, DatabaseConnection, DbErr};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct DatabaseClient {
    pub connection: Arc<DatabaseConnection>,
}

impl DatabaseClient {
    pub async fn connect(connection_string: &str) -> Result<Self, DbErr> {
        let connection = Database::connect(connection_string).await?;
        Ok(Self {
            connection: Arc::new(connection),
        })
    }

    pub fn get_connection(&self) -> Arc<DatabaseConnection> {
        self.connection.clone()
    }

    pub async fn close(&self) -> Result<(), DbErr> {
        // Arc<DatabaseConnection> 不能被 close，因为它是共享的
        // 如果需要关闭连接，应该使用 Arc::try_unwrap 获取所有权
        match Arc::try_unwrap(self.connection.clone()) {
            Ok(conn) => conn.close().await,
            Err(_) => Ok(()), // 如果有其他引用，无法关闭
        }
    }
}

pub async fn create_database_connection(
    connection_string: &str,
    max_connections: u32,
) -> Result<DatabaseConnection, DbErr> {
    let mut opt = sea_orm::ConnectOptions::new(connection_string);
    opt.max_connections(max_connections)
        .min_connections(5)
        .connect_timeout(std::time::Duration::from_secs(30))
        .idle_timeout(std::time::Duration::from_secs(60))
        .sqlx_logging(true)
        .sqlx_logging_level(tracing::log::LevelFilter::Debug);

    Database::connect(opt).await
}

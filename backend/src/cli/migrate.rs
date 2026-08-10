//! 数据库迁移子命令
//!
//! 提供 `migrate run / rollback / status / fresh / refresh / reset` 等迁移子命令。

use clap::Subcommand;
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database};
use std::time::Duration;

/// 数据库迁移子命令枚举
#[derive(Subcommand, Debug)]
pub enum MigrateCommand {
    /// 运行所有未执行的迁移
    Run,
    /// 回滚最后一次迁移
    Rollback,
    /// 撤销所有迁移
    Reset,
    /// 撤销所有迁移并重新运行
    Refresh,
    /// 丢弃所有表并重新运行所有迁移
    Fresh,
    /// 查看迁移状态
    Status,
}

/// 获取数据库连接
async fn get_db_connection() -> Result<sea_orm::DatabaseConnection, Box<dyn std::error::Error>> {
    // 批次 114 P1-5：DATABASE_URL 缺失改为优雅退出 + 友好提示（原 `expect` panic）
    // migrate 是 CLI 子命令，缺失环境变量属于用户配置错误，应清晰提示并退出而非 panic。
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        eprintln!("错误：请设置 DATABASE_URL 环境变量（例如：export DATABASE_URL=postgres://user:pass@host:5432/dbname）");
        std::process::exit(1);
    });

    let mut opt = ConnectOptions::new(db_url);
    opt.max_connections(1)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(8))
        .max_lifetime(Duration::from_secs(8));

    let db = Database::connect(opt).await?;
    Ok(db)
}

/// 数据库迁移子命令入口分发
pub async fn run(cmd: MigrateCommand) -> Result<(), Box<dyn std::error::Error>> {
    let db = get_db_connection().await?;

    match cmd {
        MigrateCommand::Run => {
            println!("开始执行数据库迁移...");
            Migrator::up(&db, None).await?;
            println!("迁移执行完成！");
        }
        MigrateCommand::Rollback => {
            println!("回滚最后一次迁移...");
            Migrator::down(&db, Some(1)).await?;
            println!("回滚完成！");
        }
        MigrateCommand::Reset => {
            println!("撤销所有迁移...");
            Migrator::down(&db, None).await?;
            println!("撤销完成！");
        }
        MigrateCommand::Refresh => {
            println!("撤销所有迁移并重新运行...");
            Migrator::refresh(&db).await?;
            println!("刷新完成！");
        }
        MigrateCommand::Fresh => {
            println!("丢弃所有表并重新运行所有迁移...");
            Migrator::fresh(&db).await?;
            println!("重新初始化完成！");
        }
        MigrateCommand::Status => {
            println!("查看迁移状态...");
            Migrator::status(&db).await?;
        }
    }
    Ok(())
}

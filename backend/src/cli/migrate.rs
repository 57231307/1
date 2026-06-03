//! 数据库迁移子命令占位模块
//!
//! 原 `bin/cli.rs` 中并未显式提供 `migrate run / rollback / status` 等迁移子命令。
//! 数据库迁移相关能力（pg_dump / psql 等）目前由 `util` 模块的 `backup` / `restore`
//! 间接提供；`src/database/migration/` 下的 SQL 文件由应用启动时按 SeaORM 流程执行。
//!
//! 此处保留模块结构以便于后续扩展：例如 `migrate run` 主动执行 `sqlx migrate run`、
//! `migrate status` 查看已执行与未执行迁移等。当前为占位实现。

use clap::Subcommand;

/// 数据库迁移子命令枚举
#[derive(Subcommand, Debug)]
pub enum MigrateCommand {
    /// 占位命令：打印提示，提示用户使用 util 下的 backup/restore 或应用启动自动迁移
    Info,
}

/// 数据库迁移子命令入口分发
pub async fn run(cmd: MigrateCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        MigrateCommand::Info => {
            println!("=== 数据库迁移说明 ===\n");
            println!("当前 CLI 未提供独立的 migrate run / rollback / status 命令。");
            println!("- 数据库备份/恢复请使用：bingxi backup / bingxi restore");
            println!("- 应用启动时 SeaORM 会自动执行 src/database/migration/ 下的 SQL 文件");
            println!("- 后续版本将在此模块添加 run / rollback / status 子命令");
        }
    }
    Ok(())
}

//! CLI 子命令模块
//!
//! 本模块对外暴露统一入口 `Command`（顶层命令枚举）与 `dispatch` 异步分发函数。
//! 各子命令分模块实现：
//! - [`admin`]：管理员操作（密码哈希等）
//! - [`migrate`]：数据库迁移（占位）
//! - [`util`]：服务管理、备份、恢复、升级、清理、配置等工具命令

pub mod admin;
pub mod migrate;
pub mod util;

use clap::Parser;

/// 顶层 CLI 命令枚举
///
/// 使用 `clap` 的"枚举即命令"模式：枚举本身派生 `Parser`，每个变体派生
/// `Subcommand` 后通过 `#[command(subcommand)]` 嵌套到对应子模块的子命令枚举。
#[derive(Parser, Debug)]
#[command(name = "bingxi")]
#[command(author = "Bingxi Team")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Bingxi ERP 系统命令行工具", long_about = None)]
pub enum Command {
    /// 管理员操作
    #[command(subcommand)]
    Admin(admin::AdminCommand),

    /// 数据库迁移
    #[command(subcommand)]
    Migrate(migrate::MigrateCommand),

    /// 工具命令
    #[command(subcommand)]
    Util(util::UtilCommand),
}

/// 顶层命令异步分发函数
///
/// `bin/cli.rs` 中的 `main` 在解析 `Command` 后调用本函数，由本函数按变体
/// 委托到对应子模块的 `run` 函数。
pub async fn dispatch(cmd: Command) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        Command::Admin(c) => admin::run(c).await,
        Command::Migrate(c) => migrate::run(c).await,
        Command::Util(c) => util::run(c).await,
    }
}

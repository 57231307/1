//! Bingxi ERP 命令行工具入口
//!
//! 仅做命令解析与分发，业务实现位于 `bingxi_backend::cli` 子模块：
//! - `admin`   管理员子命令
//! - `migrate` 数据库迁移子命令（占位）
//! - `util`    工具子命令（服务管理、备份、升级、清理等）

use bingxi_backend::cli::{dispatch, Command};
use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmd = Command::parse();
    dispatch(cmd).await
}

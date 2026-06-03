//! 管理员子命令：密码哈希、用户管理、强制登出等
//!
//! 原 cli.rs 中管理员相关的命令仅有 `hash-password`（生成密码哈希）。
//! 后续可在此模块扩展：用户激活、密码重置、强制登出等运维命令。

use clap::Subcommand;

use crate::cli::util::run_cmd;

/// 管理员子命令枚举
#[derive(Subcommand, Debug)]
pub enum AdminCommand {
    /// 生成密码哈希
    HashPassword {
        /// 原始密码
        #[arg(short, long)]
        password: String,
    },
}

/// 管理员子命令入口分发
pub async fn run(cmd: AdminCommand) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        AdminCommand::HashPassword { password } => cmd_hash_password(&password),
    }
    Ok(())
}

fn cmd_hash_password(password: &str) {
    println!("=== 生成密码哈希 ===\n");

    // 使用 Python 生成哈希
    let escaped_password = password.replace('"', "\\\"");
    let python_code = format!(
        r#"
import hashlib
import base64
import os
try:
    from argon2 import PasswordHasher
    ph = PasswordHasher()
    hash = ph.hash("{}")
    print("Argon2 哈希:", hash)
except ImportError:
    salt = os.urandom(32)
    hash = hashlib.pbkdf2_hmac('sha256', '{}'.encode(), salt, 100000)
    print("PBKDF2 哈希:", base64.b64encode(salt + hash).decode())
"#,
        escaped_password, escaped_password
    );

    match run_cmd("python3", &["-c", &python_code]) {
        Ok(hash) => println!("{}", hash),
        Err(e) => println!("[ERROR] 生成失败: {}", e),
    }
}

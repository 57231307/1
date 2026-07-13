//! 备份与恢复子命令实现：Backup / Restore

use super::{get_backup_dir, get_install_dir, require_env, run_cmd, timestamp};
use std::fs;

// 批次 322 v9 复审低危修复：路径校验逻辑已抽取到共享模块 `utils::path_validator`，
// 此处复用，避免与 upgrade.rs 重复维护。测试覆盖见 path_validator 模块。
use crate::utils::path_validator::validate_extracted_paths;

/// 获取 .env 文件路径（可通过 BINGXI_ENV_FILE 环境变量配置）
fn get_env_file_path() -> String {
    std::env::var("BINGXI_ENV_FILE").unwrap_or_else(|_| "/etc/bingxi/.env".to_string())
}

/// 获取 systemd 服务目录（可通过 BINGXI_SYSTEMD_DIR 环境变量配置）
fn get_systemd_dir() -> String {
    std::env::var("BINGXI_SYSTEMD_DIR").unwrap_or_else(|_| "/etc/systemd/system".to_string())
}

/// L4 修复（v8 复审）：函数返回 bool 表示是否成功，便于调用方（如 upgrade）根据结果决定后续流程
/// 返回 true 表示备份成功，false 表示备份失败（错误已在函数内打印）
pub(super) fn cmd_backup(backup_type: &str) -> bool {
    // 批次 323 修复：timestamp() 返回 u64，转为 String 以便传给 compress_backup(&str)
    let ts = timestamp().to_string();
    let backup_dir = format!("{}/{}", get_backup_dir(), ts);

    println!("=== 开始备份 ===\n");
    println!("备份目录: {}", backup_dir);

    // 批次 92 P3-8：原 `let _ = run_cmd(...)` 静默吞错，关键路径失败应中止
    if let Err(e) = run_cmd("mkdir", &["-p", &backup_dir]) {
        println!("[ERROR] 创建备份目录失败，终止备份: {}", e);
        return false;
    }

    // 备份数据库（批次 323 修复：合并嵌套 if 消除 collapsible_if 警告）
    if (backup_type == "database" || backup_type == "all") && !backup_database(&backup_dir) {
        if let Err(e) = run_cmd("rm", &["-rf", &backup_dir]) {
            println!("[WARN] 清理备份目录失败（可忽略）: {}", e);
        }
        return false;
    }

    // 备份文件
    if backup_type == "files" || backup_type == "all" {
        backup_config_files(&backup_dir);
    }

    // 压缩
    let tar_file = format!("{}/backup_{}.tar.gz", get_backup_dir(), ts);
    compress_backup(&tar_file, &ts);

    // 清理临时目录（非关键路径，失败仅告警）
    if let Err(e) = run_cmd("rm", &["-rf", &backup_dir]) {
        println!("[WARN] 清理临时备份目录失败（可忽略）: {}", e);
    }

    println!("\n[OK] 备份完成: {}", tar_file);
    true
}

/// 备份数据库（pg_dump）
///
/// 批次 323 v9 复审低危修复：从 cmd_backup 拆分，保持单一职责。
/// P0-1 修复（v9 复审）：pg_dump 失败必须返回 false，终止备份流程。
///
/// # 返回
/// - `true`：数据库备份成功
/// - `false`：数据库备份失败（错误已打印）
fn backup_database(backup_dir: &str) -> bool {
    println!("\n备份数据库...");
    let db_file = format!("{}/database.sql", backup_dir);
    let db_host = require_env(
        "DATABASE__HOST",
        "请设置数据库主机地址，例如 export DATABASE__HOST=127.0.0.1",
    );
    let db_user = require_env(
        "DATABASE__USERNAME",
        "请设置数据库用户名，例如 export DATABASE__USERNAME=postgres",
    );
    let db_name = require_env(
        "DATABASE__NAME",
        "请设置数据库名称，例如 export DATABASE__NAME=bingxi_erp",
    );

    // P0-1 修复（v9 复审）：数据库是核心数据，pg_dump 失败必须中止备份
    if let Err(e) = run_cmd(
        "pg_dump",
        &["-h", &db_host, "-U", &db_user, "-d", &db_name, "-f", &db_file],
    ) {
        println!("[ERROR] 数据库备份失败，终止备份: {}", e);
        return false;
    }
    println!("[OK] 数据库备份完成");
    true
}

/// 备份配置文件（config.yaml + .env + service 文件）
///
/// 批次 323 v9 复审低危修复：从 cmd_backup 拆分，保持单一职责。
/// 单个文件备份失败不中止（尽量备份剩余文件）。
fn backup_config_files(backup_dir: &str) {
    println!("\n备份配置文件...");
    let config_dir = format!("{}/backend/config.yaml", get_install_dir());
    let env_file = get_env_file_path();
    let service_file = format!("{}/{}.service", get_systemd_dir(), super::SERVICE_NAME);

    // 批次 92 P3-8：cp 失败应记录错误（不中止，尽量备份剩余文件）
    if let Err(e) = run_cmd("cp", &["-r", &config_dir, backup_dir]) {
        println!("[ERROR] 备份 config.yaml 失败: {}", e);
    }
    if let Err(e) = run_cmd("cp", &["-r", &env_file, backup_dir]) {
        println!("[ERROR] 备份 .env 失败: {}", e);
    }
    if let Err(e) = run_cmd("cp", &["-r", &service_file, backup_dir]) {
        println!("[ERROR] 备份 service 文件失败: {}", e);
    }

    println!("[OK] 配置文件备份完成");
}

/// 压缩备份目录并设置安全权限
///
/// 批次 323 v9 复审低危修复：从 cmd_backup 拆分，保持单一职责。
/// 规则 12 合规：设置备份文件权限为 0o600，防止 .env 敏感信息泄露。
fn compress_backup(tar_file: &str, ts: &str) {
    println!("\n压缩备份...");
    if let Err(e) = run_cmd(
        "tar",
        &["-czf", tar_file, "-C", &get_backup_dir(), ts],
    ) {
        println!("[ERROR] 压缩失败: {}", e);
        return;
    }
    // 规则 12 合规：设置备份文件权限为 0o600（仅所有者可读），
    // 防止备份中的 .env（含数据库密码等敏感信息）被其他用户读取
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = fs::set_permissions(tar_file, fs::Permissions::from_mode(0o600)) {
            println!("[WARN] 设置备份文件权限失败（可忽略）: {}", e);
        }
    }
}

/// L4 修复（v8 复审）：函数返回 bool 表示是否成功，便于调用方根据结果决定后续流程
/// 返回 true 表示恢复成功，false 表示恢复失败（错误已在函数内打印）
pub(super) fn cmd_restore(file: &str) -> bool {
    println!("=== 恢复数据 ===\n");
    println!("备份文件: {}", file);

    if !std::path::Path::new(file).exists() {
        println!("[ERROR] 文件不存在: {}", file);
        return false;
    }

    // 生成随机临时目录名，防止符号链接竞争攻击（TOCTOU）
    let temp_dir_owned = format!(
        "{}/bingxi_restore_{}",
        std::env::temp_dir().to_string_lossy(),
        uuid::Uuid::new_v4()
    );
    let temp_dir = temp_dir_owned.as_str();
    // 清理旧临时目录（非关键路径）
    if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
        println!("[WARN] 清理旧临时目录失败（可忽略）: {}", e);
    }
    // 创建临时目录（关键路径）
    if let Err(e) = run_cmd("mkdir", &["-p", temp_dir]) {
        println!("[ERROR] 创建临时目录失败，终止恢复: {}", e);
        return false;
    }

    // M4 修复（v8 复审）：先列出 tar 内容并校验路径，再解压，防止恶意文件在校验前已写入磁盘
    if !validate_tar_contents(file, temp_dir) {
        return false;
    }

    println!("解压备份...");
    if let Err(e) = run_cmd("tar", &["-xzf", file, "-C", temp_dir]) {
        println!("[ERROR] 解压失败: {}", e);
        if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
            println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
        }
        return false;
    }

    // 规则 12 合规：解压后二次校验（canonicalize 解析符号链接），双重防护
    if let Err(e) = validate_extracted_paths(temp_dir) {
        println!("[ERROR] 安全校验失败，终止恢复: {}", e);
        if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
            println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
        }
        return false;
    }

    // 恢复数据库（批次 323 修复：合并嵌套 if 消除 collapsible_if 警告）
    let db_file = format!("{}/database.sql", temp_dir);
    if std::path::Path::new(&db_file).exists() && !restore_database(&db_file) {
        if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
            println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
        }
        return false;
    }

    // 恢复配置
    restore_config_files(temp_dir);

    // 清理临时目录（非关键路径）
    if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
        println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
    }

    println!("\n[OK] 恢复完成，请重启服务: bingxi restart");
    true
}

/// 校验 tar 文件内容（先 tar -tf 列出，再逐行校验路径穿越）
///
/// 批次 323 v9 复审低危修复：从 cmd_restore 拆分，保持单一职责。
/// M4 修复（v8 复审）：先列出 tar 内容并校验路径，再解压，防止恶意文件在校验前已写入磁盘。
///
/// # 返回
/// - `true`：校验通过
/// - `false`：校验失败（错误已打印，临时目录已清理）
fn validate_tar_contents(file: &str, temp_dir: &str) -> bool {
    println!("校验备份文件内容...");
    let tar_list = match run_cmd("tar", &["-tf", file]) {
        Ok(list) => list,
        Err(e) => {
            println!("[ERROR] 列出备份文件内容失败: {}", e);
            if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
                println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
            }
            return false;
        }
    };

    // 解压前校验：检查每个路径不包含 .. 和不以 / 开头（防止 Tar Slip 路径穿越）
    for line in tar_list.lines() {
        let path = line.trim();
        if path.is_empty() || path == "./" {
            continue;
        }
        if path.contains("..") {
            println!("[ERROR] 检测到路径穿越攻击：文件 {} 包含 ..", path);
            if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
                println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
            }
            return false;
        }
        if path.starts_with('/') {
            println!("[ERROR] 检测到绝对路径：文件 {}", path);
            if let Err(e) = run_cmd("rm", &["-rf", temp_dir]) {
                println!("[WARN] 清理临时目录失败（可忽略）: {}", e);
            }
            return false;
        }
    }
    true
}

/// 恢复数据库（psql）
///
/// 批次 323 v9 复审低危修复：从 cmd_restore 拆分，保持单一职责。
/// P1 修复（v9 复审）：psql 恢复失败必须返回 false，终止恢复流程。
///
/// # 返回
/// - `true`：数据库恢复成功
/// - `false`：数据库恢复失败（错误已打印）
fn restore_database(db_file: &str) -> bool {
    println!("\n恢复数据库...");
    let db_host = require_env(
        "DATABASE__HOST",
        "请设置数据库主机地址，例如 export DATABASE__HOST=127.0.0.1",
    );
    let db_user = require_env(
        "DATABASE__USERNAME",
        "请设置数据库用户名，例如 export DATABASE__USERNAME=postgres",
    );
    let db_name = require_env(
        "DATABASE__NAME",
        "请设置数据库名称，例如 export DATABASE__NAME=bingxi_erp",
    );

    // P1 修复（v9 复审）：数据库是核心数据，psql 恢复失败必须中止
    if let Err(e) = run_cmd(
        "psql",
        &["-h", &db_host, "-U", &db_user, "-d", &db_name, "-f", db_file],
    ) {
        println!("[ERROR] 数据库恢复失败，终止恢复: {}", e);
        return false;
    }
    println!("[OK] 数据库恢复完成");
    true
}

/// 恢复配置文件（config.yaml + .env）
///
/// 批次 323 v9 复审低危修复：从 cmd_restore 拆分，保持单一职责。
/// 单个文件恢复失败不中止（记录错误继续恢复剩余文件）。
fn restore_config_files(temp_dir: &str) {
    println!("\n恢复配置文件...");
    for name in &["config.yaml", ".env"] {
        let src = format!("{}/{}", temp_dir, name);
        if std::path::Path::new(&src).exists() {
            let dst = if *name == ".env" {
                get_env_file_path()
            } else {
                format!("{}/backend/{}", get_install_dir(), name)
            };
            // 批次 92 P3-8：cp 失败应记录错误
            if let Err(e) = run_cmd("cp", &[&src, &dst]) {
                println!("[ERROR] 恢复 {} 失败: {}", name, e);
            } else {
                println!("[OK] 恢复: {}", name);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// M8 测试：get_env_file_path 默认返回 /etc/bingxi/.env
    #[test]
    fn test_get_env_file_path_default() {
        std::env::remove_var("BINGXI_ENV_FILE");
        assert_eq!(get_env_file_path(), "/etc/bingxi/.env");
    }

    /// M8 测试：get_env_file_path 从环境变量读取
    #[test]
    fn test_get_env_file_path_from_env() {
        std::env::set_var("BINGXI_ENV_FILE", "/custom/path/.env");
        assert_eq!(get_env_file_path(), "/custom/path/.env");
        std::env::remove_var("BINGXI_ENV_FILE");
    }

    /// M8 测试：get_systemd_dir 默认返回 /etc/systemd/system
    #[test]
    fn test_get_systemd_dir_default() {
        std::env::remove_var("BINGXI_SYSTEMD_DIR");
        assert_eq!(get_systemd_dir(), "/etc/systemd/system");
    }

    /// M8 测试：get_systemd_dir 从环境变量读取
    #[test]
    fn test_get_systemd_dir_from_env() {
        std::env::set_var("BINGXI_SYSTEMD_DIR", "/custom/systemd");
        assert_eq!(get_systemd_dir(), "/custom/systemd");
        std::env::remove_var("BINGXI_SYSTEMD_DIR");
    }

    // 批次 322 v9 复审低危修复：validate_dir_recursive 和 validate_extracted_paths 的
    // 单元测试已迁移到共享模块 utils::path_validator，此处不再重复维护。
}

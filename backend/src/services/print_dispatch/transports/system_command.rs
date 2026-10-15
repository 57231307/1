//! 系统命令传输：把文档投递委托给一条外部命令
//!
//! 以 `sh -c` 执行 `print_targets.command_template`，把占位符 `{file}` / `{copies}` /
//! `{printer}` 替换为临时文件路径、份数与目标符号编码后运行。命令非零退出即失败可见
//! （带上 stderr），成功时取 stdout 去空白作为外部作业号；stdout 为空说明外部命令没有
//! 回执通道，改用可关联任务号的合成标识并记 WARN，绝不谎称拿到设备作业号。
//!
//! 这是本子系统内可确定性端到端测试的通道：用无害的 `echo` 模板即可在不接硬件、不联网
//! 的情况下验证「命令执行 → 解析作业号 → 落库」全链路。

use async_trait::async_trait;
use tokio::process::Command;
use tracing::warn;

use crate::services::print_dispatch::transport::{DispatchError, PrintJob, PrintTransport};

/// 占位符替换：把 `{file}` / `{copies}` / `{printer}` 换成对应值（纯函数，便于确定性断言）。
///
/// 入参为命令模板与三个替换值；模板中未出现的占位符不受影响，已出现的全部替换。
pub fn substitute_placeholders(template: &str, file: &str, copies: i32, printer: &str) -> String {
    template
        .replace("{file}", file)
        .replace("{copies}", &copies.to_string())
        .replace("{printer}", printer)
}

/// 系统命令传输实现，持有命令模板（来自 `print_targets.command_template`）。
pub struct SystemCommandTransport {
    command_template: String,
}

impl SystemCommandTransport {
    pub fn new(command_template: String) -> Self {
        Self { command_template }
    }
}

#[async_trait]
impl PrintTransport for SystemCommandTransport {
    async fn send(&self, job: &PrintJob, _bytes: &[u8]) -> Result<String, DispatchError> {
        let command = substitute_placeholders(
            &self.command_template,
            &job.file_path.to_string_lossy(),
            job.copies,
            &job.target.target_code,
        );

        let output = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output()
            .await
            .map_err(|e| DispatchError::Transport {
                detail: format!("无法启动打印命令（sh -c）: {e}"),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(DispatchError::Transport {
                detail: format!(
                    "打印命令非零退出（退出码 {:?}）: {}",
                    output.status.code(),
                    if stderr.is_empty() {
                        "无 stderr 输出".to_string()
                    } else {
                        stderr
                    }
                ),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if stdout.is_empty() {
            warn!(
                job_code = %job.job_code,
                "打印命令成功但无 stdout 回执，改用合成外部作业号"
            );
            return Ok(format!("syscmd-{}", job.job_code));
        }
        Ok(stdout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 三个占位符应按对应值替换，未出现的占位符不动。
    #[test]
    fn substitutes_all_three_placeholders() {
        let out = substitute_placeholders(
            "lp -P {printer} -# {copies} {file}",
            "/tmp/j.docx",
            3,
            "pool-a",
        );
        assert_eq!(out, "lp -P pool-a -# 3 /tmp/j.docx");
    }

    /// 模板不含占位符时原样返回（不因替换而改写命令）。
    #[test]
    fn leaves_plain_template_untouched() {
        assert_eq!(
            substitute_placeholders("echo hi", "/tmp/a", 1, "x"),
            "echo hi"
        );
    }

    /// 同一占位符多次出现应全部替换。
    #[test]
    fn replaces_repeated_placeholder() {
        let out = substitute_placeholders("{file} {file}", "/p", 1, "q");
        assert_eq!(out, "/p /p");
    }
}

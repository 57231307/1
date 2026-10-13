//! CUPS 传输：经 `lp` 命令行向打印队列提交作业
//!
//! 调用 `lp -d <queue_name> -n <copies> <tmpfile>` 把临时文件投递到指定队列，解析
//! `lp` 标准输出中的「request id is <N>」作为外部作业号。`lp` 不存在或返回非零退出
//! 一律以 [`DispatchError`] 显式上抛，由编排层转为任务失败。

use async_trait::async_trait;
use tokio::process::Command;
use tracing::warn;

use crate::services::print_dispatch::transport::{DispatchError, PrintJob, PrintTransport};

/// `lp` 成功输出解析作业号的纯函数。
///
/// CUPS `lp` 在作业入队后向 stdout 打印形如 `request id is <ID> (N file(s))` 的行，
/// 本函数取短语 `request id is` 之后的第一个空白分隔 token 作为外部作业号；解析不到
/// 返回 `None`（由调用方决定如何如实处置，不在此伪造作业号）。入参为 `lp` 的完整 stdout。
pub fn parse_lp_request_id(stdout: &str) -> Option<String> {
    let phrase = "request id is ";
    let idx = stdout.find(phrase)?;
    let rest = &stdout[idx + phrase.len()..];
    let token = rest.split_whitespace().next()?;
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

/// CUPS 传输实现，持有目标队列名（来自 `print_targets.queue_name`）。
pub struct CupsTransport {
    queue_name: String,
}

impl CupsTransport {
    pub fn new(queue_name: String) -> Self {
        Self { queue_name }
    }
}

#[async_trait]
impl PrintTransport for CupsTransport {
    async fn send(&self, job: &PrintJob, _bytes: &[u8]) -> Result<String, DispatchError> {
        // 走队列文件输入路径：`lp` 读取磁盘文件而非字节流，故使用 job.file_path，忽略 bytes。
        // 说明 lpr 备选：若部署只有 BSD `lpr`（无 `lp`），等价命令为
        //   `lpr -P <queue_name> -# <copies> <tmpfile>`，其成功输出不含 request id，
        //   外部作业号需另行合成；本实现面向 CUPS `lp` 以获取可回查的作业号。
        let output = Command::new("lp")
            .arg("-d")
            .arg(&self.queue_name)
            .arg("-n")
            .arg(job.copies.to_string())
            .arg(&job.file_path)
            .output()
            .await
            .map_err(|e| DispatchError::Transport {
                detail: format!("CUPS lp 命令不可用（队列 {}）: {e}", self.queue_name),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(DispatchError::Transport {
                detail: format!(
                    "CUPS lp 投递失败（队列 {}，退出码 {:?}）: {}",
                    self.queue_name,
                    output.status.code(),
                    if stderr.is_empty() {
                        "无 stderr 输出".to_string()
                    } else {
                        stderr
                    }
                ),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        match parse_lp_request_id(&stdout) {
            Some(id) => Ok(id),
            // lp 已零退出（作业确已入队）但输出无法解析作业号：绝不把已接受的重印作业
            // 误判为失败（否则重试会重复出纸），改用可关联任务号的合成标识并显式记 WARN，
            // 不静默谎称拿到了后端真实作业号。
            None => {
                warn!(
                    job_code = %job.job_code,
                    queue = %self.queue_name,
                    lp_stdout = %stdout.trim(),
                    "CUPS lp 成功入队但未能解析 request id，改用合成外部作业号（后续不可回查真实作业号）"
                );
                Ok(format!("cups-{}", job.job_code))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_request_id() {
        let out = "request id is HP_DeskJet-42 (1 file(s))\n";
        assert_eq!(parse_lp_request_id(out).as_deref(), Some("HP_DeskJet-42"));
    }

    #[test]
    fn returns_none_when_phrase_absent() {
        assert_eq!(parse_lp_request_id("nothing useful here"), None);
    }

    #[test]
    fn returns_none_when_trailing_token_empty() {
        assert_eq!(parse_lp_request_id("request id is "), None);
    }
}

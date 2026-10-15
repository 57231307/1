//! 网络（raw socket / JetDirect 9100）传输
//!
//! 直接向目标主机端口的 TCP 连接写入文档字节，写完即认为投递成功。此类原始套接字通道
//! **没有作业号回执**：设备不会返回可回查的 job id，因此外部作业号由任务号合成
//! （`net-<job_code>`），仅作台账关联用途，不代表设备侧真实作业号。
//!
//! 诚实说明（当前能力的边界）：raw-socket 打印只适用于设备原生可识别的页面/数据流
//! （常见为 PCL/PS/纯文本或设备支持的位图）；本子系统的文档成品是 `.docx`，多数直连
//! 设备无法直接解释 OOXML，本阶段不做 PDF/光栅转换，故 `network` 目标对 `.docx`
//! 的出纸效果依设备而异。需要高保真输出的场景应选用 `cups` / `ipp` / `system_command`
//! 中具备驱动或外部渲染能力的目标。

use std::time::Duration;

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::services::print_dispatch::transport::{DispatchError, PrintJob, PrintTransport};

/// 建立连接前的等待上限：原始套接字无应用层往返，给 TCP 握手留一个明确的短超时，
/// 到点即失败可见，不让派发无限挂起。
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// 单次写操作的等待上限。
const WRITE_TIMEOUT: Duration = Duration::from_secs(30);

/// 网络传输实现，持有目标主机与端口（来自 `print_targets.host` / `port`）。
pub struct NetworkTransport {
    host: String,
    port: u16,
}

impl NetworkTransport {
    pub fn new(host: String, port: u16) -> Self {
        Self { host, port }
    }

    /// 解析并格式化 `host:port` 目标地址（纯函数，便于确定性校验与日志）。
    fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[async_trait]
impl PrintTransport for NetworkTransport {
    async fn send(&self, job: &PrintJob, bytes: &[u8]) -> Result<String, DispatchError> {
        let addr = self.socket_addr();

        // 连接：到点未建立即失败可见，绝不吞成 Ok 让作业停留在“已发起未送达”的假象
        let mut stream =
            match tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(&addr)).await {
                Ok(Ok(s)) => s,
                Ok(Err(e)) => {
                    return Err(DispatchError::Transport {
                        detail: format!("连接网络打印目标 {addr} 失败: {e}"),
                    });
                }
                Err(_) => {
                    return Err(DispatchError::Transport {
                        detail: format!(
                            "连接网络打印目标 {addr} 超时（超过 {} 秒）",
                            CONNECT_TIMEOUT.as_secs()
                        ),
                    });
                }
            };

        // 写入字节并刷新/关闭写半，确保 EOF 送达（JetDirect 依赖连接关闭触发出纸）
        let write_res = tokio::time::timeout(WRITE_TIMEOUT, async {
            stream.write_all(bytes).await?;
            stream.flush().await?;
            // 显式关闭写方向，向设备表达“数据结束”（tokio 的 AsyncWriteExt::shutdown
            // 会优雅关闭写半并触发底层 TcpStream 的 Write shutdown）
            stream.shutdown().await?;
            Ok::<(), std::io::Error>(())
        })
        .await;

        match write_res {
            Ok(Ok(())) => Ok(format!("net-{}", job.job_code)),
            Ok(Err(e)) => Err(DispatchError::Transport {
                detail: format!("向网络打印目标 {addr} 写入失败: {e}"),
            }),
            Err(_) => Err(DispatchError::Transport {
                detail: format!(
                    "向网络打印目标 {addr} 写入超时（超过 {} 秒）",
                    WRITE_TIMEOUT.as_secs()
                ),
            }),
        }
    }
}

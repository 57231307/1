//! 打印派发传输层抽象与工厂
//!
//! 本模块定义把一份已渲染的文档字节投递到具体打印目标的传输契约
//! [`PrintTransport`]、由 `print_targets` 行解析而来的连接上下文 [`ResolvedTarget`]、
//! 单次派发所需的作业快照 [`PrintJob`]、贯穿各传输的失败类型 [`DispatchError`]，
//! 以及按 `kind` 选择具体传输实现的工厂 [`build_transport`]。
//!
//! 取值口径：`ResolvedTarget.kind` 与 `print_targets.kind`、`print_tasks.backend_kind`
//! 共用同一闭合集合 `cups` / `ipp` / `network` / `system_command`（由建表迁移的具名
//! CHECK 约束锁定）；本模块只消费该值，不再引入第二套后端类型常量。任何目标地址
//! （队列名 / 主机 / 端口 / IPP 端点 / 命令模板）都来自数据库行，代码中不含设备地址字面量。

use std::path::PathBuf;

use async_trait::async_trait;

use crate::services::print_dispatch::transports::cups::CupsTransport;
use crate::services::print_dispatch::transports::ipp::IppTransport;
use crate::services::print_dispatch::transports::network::NetworkTransport;
use crate::services::print_dispatch::transports::system_command::SystemCommandTransport;

/// `print_targets.kind` / `print_tasks.backend_kind` 的四种合法后端类型 token。
///
/// 这些取值与迁移中 `chk_print_targets_kind`、`chk_print_tasks_backend_kind` 的取值集
/// 逐项一致，由词表守卫测试锁定；工厂按此集合分派，集合外的值一律显式失败。
pub const KIND_CUPS: &str = "cups";
pub const KIND_IPP: &str = "ipp";
pub const KIND_NETWORK: &str = "network";
pub const KIND_SYSTEM_COMMAND: &str = "system_command";

/// network 传输的默认端口（未配置 `print_targets.port` 时使用）。
///
/// 取 IPP/raw-socket 打印服务的事实标准端口 9100；端口本身是可选连接参数，
/// 具体主机地址仍由数据库目标行提供，不在此硬编码。
pub const DEFAULT_NETWORK_PORT: u16 = 9100;

/// 从 `print_targets` 行解析出的连接上下文（若目标为池，则为选中的单台成员行）。
///
/// 字段与 `print_targets` 各连接列逐一对应，仅保留传输实际所需者：池的 `is_pool` /
/// `pool_members` 已在派发编排里展开为具体成员，进入传输层的 `ResolvedTarget` 恒为
/// 单台目标（`is_pool=false`），因此这里不含池信息。
#[derive(Debug, Clone)]
pub struct ResolvedTarget {
    /// 目标符号编码（`target_code`），作为作业外部标识合成与命令模板 `{printer}` 占位的来源
    pub target_code: String,
    /// 连接种类（`kind`），决定由 [`build_transport`] 选择哪个传输实现
    pub kind: String,
    /// `cups` 所用打印队列名
    pub queue_name: Option<String>,
    /// `network` 所用主机地址
    pub host: Option<String>,
    /// `network` 所用端口（`None` 时传输层取 [`DEFAULT_NETWORK_PORT`]）
    pub port: Option<i32>,
    /// `ipp` 所用端点 URL
    pub endpoint_url: Option<String>,
    /// `system_command` 所用占位符命令模板
    pub command_template: Option<String>,
}

/// 单次派发作业快照：连接目标、份数与已落盘的临时文件路径。
///
/// `bytes` 由传输的 `send` 形参另行传入（raw-socket 与 IPP 直接写字节，cups 与
/// system_command 走临时文件路径），因此这里只携带与「目标 + 份数 + 文件位置」
/// 相关的上下文。
#[derive(Debug, Clone)]
pub struct PrintJob {
    /// 已解析的单台目标连接上下文
    pub target: ResolvedTarget,
    /// 打印份数
    pub copies: i32,
    /// 文档字节已写入的临时文件绝对路径（cups / system_command 以此为输入）
    pub file_path: PathBuf,
    /// 任务业务编码（用于合成无回执通道的外部作业号，并关联日志）
    pub job_code: String,
}

/// 派发链路各失败点的统一错误类型。
///
/// `Display` 文案即写入 `print_tasks.error_message` 的内容（失败可见性由表上的
/// `chk_print_tasks_failed_has_error` 强制：状态为 failed 时该列必非空，故每个失败
/// 分支都必须给出非空文案，本类型不存在无文案的变体）。
#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    /// 目标在 `print_targets` 中不存在、已禁用，或池成员缺失/为空/被禁用。
    /// 文案逐字为「打印目标未配置或未启用: <name>」，供编排层直接落 error_message。
    #[error("打印目标未配置或未启用: {name}")]
    TargetUnresolved { name: String },

    /// `kind` 不在四种受支持后端之内（词表外的历史脏行或误配）。
    #[error("不支持的打印后端类型: {kind}")]
    UnsupportedBackend { kind: String },

    /// 目标行缺少其 `kind` 所必需的连接字段（队列名 / 主机 / 端点 / 命令模板）。
    /// 迁移的 `chk_print_targets_kind_required` 已挡住院内空值，这里是对读到行后
    /// 再次按种类校验的纵深防御，绝不静默用空值继续派发。
    #[error("打印目标缺少必需的连接参数: {field}")]
    MissingTargetField { field: String },

    /// 文档临时文件落盘失败（建目录或写字节报错）。
    #[error("打印临时文件写入失败: {detail}")]
    TempIo { detail: String },

    /// 具体传输实现上报的投递失败，`detail` 为责任传输构造的可读原因
    /// （命令不可用 / 进程退出码与 stderr / HTTP 状态与 IPP 错误码 / 连接错误等）。
    #[error("打印投递失败: {detail}")]
    Transport { detail: String },

    /// 传输在配置时限内未完成（编排层用 `tokio::time::timeout` 到点后构造）。
    #[error("派发超时")]
    Timeout,

    /// 传输内部发生 panic（编排层捕获 `JoinError`），避免任务静默停留 in-flight。
    #[error("打印派发过程发生异常: {detail}")]
    Panic { detail: String },

    /// 乐观锁前置状态不满足：任务已被并发实例推进到其它状态，本实例主动让位。
    #[error("任务状态已被并发更新，本次派发中止以避免重复推进")]
    ConcurrentAdvance,
}

/// 打印传输契约：把一个作业投递到其目标，成功返回后端可回查的外部作业号。
///
/// 返回的 `String` 存入 `print_tasks.external_job_id`；失败返回 [`DispatchError`]，
/// 由编排层转为状态 failed 并写 `error_message`。实现不得吞掉设备/网络/进程错误，
/// 非零退出与传输层错误都必须以 Err 显式上抛。
#[async_trait]
pub trait PrintTransport: Send + Sync {
    async fn send(&self, job: &PrintJob, bytes: &[u8]) -> Result<String, DispatchError>;
}

/// 按目标 `kind` 构造对应传输实现。
///
/// 输入是已解析的单台 [`ResolvedTarget`]；对每个种类先校验其必需连接字段，缺失即
/// [`DispatchError::MissingTargetField`]，绝不构造一个注定失败或静默空转的传输。
pub fn build_transport(target: &ResolvedTarget) -> Result<Box<dyn PrintTransport>, DispatchError> {
    match target.kind.as_str() {
        KIND_CUPS => {
            let queue_name = target
                .queue_name
                .clone()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| DispatchError::MissingTargetField {
                    field: format!("目标 {} 为 cups 但缺少 queue_name", target.target_code),
                })?;
            Ok(Box::new(CupsTransport::new(queue_name)))
        }
        KIND_IPP => {
            let endpoint_url = target
                .endpoint_url
                .clone()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| DispatchError::MissingTargetField {
                    field: format!("目标 {} 为 ipp 但缺少 endpoint_url", target.target_code),
                })?;
            Ok(Box::new(IppTransport::new(endpoint_url)))
        }
        KIND_NETWORK => {
            let host = target
                .host
                .clone()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| DispatchError::MissingTargetField {
                    field: format!("目标 {} 为 network 但缺少 host", target.target_code),
                })?;
            let port = target
                .port
                .filter(|p| *p > 0)
                .map(|p| p as u16)
                .unwrap_or(DEFAULT_NETWORK_PORT);
            Ok(Box::new(NetworkTransport::new(host, port)))
        }
        KIND_SYSTEM_COMMAND => {
            let command_template = target
                .command_template
                .clone()
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| DispatchError::MissingTargetField {
                    field: format!(
                        "目标 {} 为 system_command 但缺少 command_template",
                        target.target_code
                    ),
                })?;
            Ok(Box::new(SystemCommandTransport::new(command_template)))
        }
        other => Err(DispatchError::UnsupportedBackend {
            kind: other.to_string(),
        }),
    }
}

/// 从 `print_targets` 单行实体构造 [`ResolvedTarget`]（池展开为成员行后同样经此转换）。
pub fn resolve_target_from_row(
    row: &crate::models::print_target::Model,
) -> Result<ResolvedTarget, DispatchError> {
    Ok(ResolvedTarget {
        target_code: row.target_code.clone(),
        kind: row.kind.clone(),
        queue_name: row.queue_name.clone(),
        host: row.host.clone(),
        port: row.port,
        endpoint_url: row.endpoint_url.clone(),
        command_template: row.command_template.clone(),
    })
}

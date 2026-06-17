//! 可观测性基础设施
//!
//! 提供分布式追踪、结构化日志、性能指标的统一接入层。
//!
//! ## 模块结构
//!
//! - [`trace_context`]：W3C Trace Context（`traceparent` / `tracestate`）解析与生成
//! - [`span`]：业务域 span 工具函数
//!
//! ## 设计目标
//!
//! 1. **不引入重依赖**：暂不引入 `opentelemetry` / `opentelemetry-otlp` 等大依赖
//! 2. **标准化协议**：使用 W3C Trace Context（与 Jaeger / Tempo / DataDog 等后端兼容）
//! 3. **可演进**：未来引入 OTel SDK 时，只需替换 `trace_context` 的实现，
//!    `span!` 宏的调用点完全不需要改
//!
//! ## 一次请求的 trace 生命周期
//!
//! ```text
//! 客户端 ──HTTP header──> middleware::trace_context ──> handler ──> service ──> db
//!       traceparent=00-aaaa-bbbb-01
//!                              │
//!                              ├─ 解析 / 生成 TraceContext
//!                              ├─ 注入到 tracing::Span（record!）
//!                              ├─ 透传到下游（reqwest / gRPC metadata）
//!                              └─ 在响应头回写 `X-Trace-Id`
//! ```

pub mod config;
pub mod span;
pub mod trace_context;

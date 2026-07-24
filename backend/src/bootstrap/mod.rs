//! 应用启动引导模块（bootstrap）
//!
//! 按 main.rs 启动流程职责拆分，各子模块职责如下：
//! - `infra_bootstrap`：环境变量 / 配置 / 日志 / 数据库连接（基础设施层）
//! - `service_bootstrap`：数据库迁移 / 服务创建 / 后台任务 / AppState 组装（服务层）
//! - `middleware_bootstrap`：CORS / 安全头 / 中间件链配置（中间件层）
//! - `routes_bootstrap`：Setup 模式路由 + 初始化接口 handler（路由层）

pub mod infra_bootstrap;
pub mod middleware_bootstrap;
pub mod routes_bootstrap;
pub mod service_bootstrap;

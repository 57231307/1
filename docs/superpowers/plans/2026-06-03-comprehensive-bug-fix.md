# 2026-06-03 全面修复项目错误与异常

## 排查结果（基线）

- 8 个 handler 在 `missing_handlers.rs` 返回 `功能暂未实现` 错误（P0 真实存在）
- 4 个 handler 在 `missing_handlers.rs` 返回 `vec![]`（P0 用户看不到数据）
- `cli.rs:496,497,498,561,626` 硬编码生产 DB host/user/name（P0 安全）
- 前端 `fabric/index.vue:540,542` 两处 `.catch(() => {})` 吞错（P1）
- 缺 `.env.example` 环境变量文档（P1）

## 修复策略（不简化功能）

### P0-1 会计期间 4 个 handler（用真实 `AccountingPeriodService`）

`missing_handlers.rs` 4 个函数改用 `AccountingPeriodService::new(state.db.clone())` 真实查询 / 创建 / 关闭 / 获取。

### P0-2 MRP 历史 2 个 handler（用真实 `MRPService`）

`get_mrp_history` → `MRPService::list_results()`；`get_mrp_history_detail` → `MRPService::get_results(id)`。

### P0-3 销售用户 1 个 handler

从 `user_handler` 取 role='销售' 的活跃用户列表。

### P0-4 CRM 回收规则 4 个 handler（表缺失）

`crm_recycle_rules` 表和 service 都不存在。最稳妥的"不简化"实现：
- 在 `AppState` 加 `Arc<Mutex<Vec<RecycleRule>>>` 内存存储（init 时填默认 3 条）
- 4 个 handler 改为操作这个内存存储
- 数据库迁移文件 `038_create_crm_recycle_rules.sql` 创建表结构（暂不强制，本期先内存实现）

### P0-5 `cli.rs` 硬编码数据库配置

- 移除 `unwrap_or_else(|_| "39.99.34.194".to_string())` 模式
- 改为强制读取 env var，缺失时直接 eprintln 退出进程

### P1-1 前端吞错

`frontend/src/views/fabric/index.vue:540,542` 改为 toast 提示"加载失败，请重试"

### P1-2 `.env.example`

在仓库根创建 `.env.example`，列出所有必填环境变量

### P1-3 `docs/PROJECT_HEALTH_REPORT.md`

汇总本轮排查发现与修复点

## 验证

- 本地 `cargo fmt --all` + `cargo check`（沙箱 1.92 受 sea-orm 1.94 限制会失败，靠 CI 验证）
- CI/CD #739 完整跑通 = 修复成功

# 批次 95：v4 P3 修复规划（20 项）

**生成时间**：2026-07-03
**关联复审**：`docs/audits/2026-07-03-reaudit-v4.md` 第五节 P3 问题
**修复目标**：修复 20 项 P3 问题 — panic!/unwrap + 分页 clamp + TOCTOU + CLI 吞错 + 前端占位

## 修复项清单

| # | 问题 | 文件 | 类型 | 复杂度 |
|---|------|------|------|--------|
| 1 | failover_service.rs panic! 修复与计划不符 | services/failover_service.rs:134,140 | 改 Result | 中 |
| 2 | finance_report_service.rs .unwrap() | services/finance_report_service.rs:632,636 | 改 ok_or_else | 低 |
| 3-8 | 分页参数未 clamp 上限（30+ 处） | 30+ handler | 加 .clamp(1,100) | 中 |
| 9 | api_gateway_handler.rs key_to_json created_by 占位 0 | handlers/api_gateway_handler.rs:153-154 | 接入真实 user_id | 中 |
| 10 | api_gateway_handler.rs create_api_endpoint 唯一性 TOCTOU | handlers/api_gateway_handler.rs:230-237 | 补 txn + lock | 中 |
| 11 | fixed_asset_service.rs 唯一约束匹配宽泛 | services/fixed_asset_service.rs:313-316 | 收紧匹配 | 低 |
| 12 | cli/util/service.rs cmd_logs 仍吞错 | cli/util/service.rs:125,137,147 | 加日志 | 低 |
| 13 | cli/util/upgrade.rs 关键路径失败不中止 | cli/util/upgrade.rs:126-137,191-216,225-237 | 改 ? 传播 | 中 |
| 14 | .env.example ENV 变量重复定义 | backend/.env.example:40,47 | 去重 | 低 |
| 15 | bpm_service_stub.rs 文件名误导 | services/bpm_service_stub.rs | 重命名/删除 | 低 |
| 16 | routes/v1.rs 占位路由 | routes/v1.rs:1-23 | 清理 | 低 |
| 17 | 前端 sales-price 价格策略占位 | sales-price/composables/useSpProc.ts:61 | 实现 | 中 |
| 18 | 前端 sales-analysis 编辑目标占位 | sales-analysis/composables/useSaProc.ts:9 | 实现 | 中 |
| 19 | 前端 crm/opportunities 查看详情空函数 | crm/opportunities/index.vue:311 | 实现 | 中 |
| 20 | 前端 sales-analysis 图表占位 | sales-analysis/components/SaTrend.vue:22,36 | 实现 | 中 |

## 修复分批

**批次 95（本批次，一次性合并 20 项）**：
- 子批 A：后端 panic!/unwrap + 分页 clamp（项 1, 2, 3-8）
- 子批 B：后端 TOCTOU + CLI + 配置清理（项 9-16）
- 子批 C：前端占位功能（项 17-20）

## 详细修复方案

### 项 1：failover_service.rs panic! 改 Result

- 位置：`backend/src/services/failover_service.rs:134,140`
- 问题：批次 92 P3-7 计划改为 Result/日志，实际仍 panic!，仅加 tracing::error!
- 修复：将 panic! 改为 `return Err(AppError::internal(...))` 或日志 + 降级
- 先读文件确认 panic! 上下文和函数签名

### 项 2：finance_report_service.rs .unwrap() 改 ok_or_else

- 位置：`backend/src/services/finance_report_service.rs:632,636`
- 修复：`.unwrap()` → `.ok_or_else(|| AppError::internal(...))?`

### 项 3-8：分页参数 clamp 上限（30+ 处）

- 位置：30+ handler 中 `page.unwrap_or(1)` 和 `page_size.unwrap_or(10)` 未做 clamp
- 修复：
  - `page.unwrap_or(1)` → `page.unwrap_or(1).max(1)`
  - `page_size.unwrap_or(10)` → `page_size.unwrap_or(10).clamp(1, 100)`
- 搜索模式：`grep -rn "unwrap_or(1)" backend/src/handlers/` 和 `grep -rn "unwrap_or(10)" backend/src/handlers/`
- 注意：部分 handler 已有 clamp（如 warehouse_handler.rs），跳过已修复的

### 项 9：api_gateway_handler.rs key_to_json created_by 占位 0

- 位置：`backend/src/handlers/api_gateway_handler.rs:153-154`
- 修复：接入 AuthContext 注入真实 user_id

### 项 10：api_gateway_handler.rs create_api_endpoint 唯一性 TOCTOU

- 位置：`backend/src/handlers/api_gateway_handler.rs:230-237`
- 修复：补 txn + lock_exclusive 或改用唯一约束 + catch 错误

### 项 11：fixed_asset_service.rs 唯一约束匹配宽泛

- 位置：`backend/src/services/fixed_asset_service.rs:313-316`
- 修复：收紧匹配条件

### 项 12：cli/util/service.rs cmd_logs 仍吞错

- 位置：`backend/src/cli/util/service.rs:125,137,147`
- 修复：加 warn 日志

### 项 13：cli/util/upgrade.rs 关键路径失败不中止

- 位置：`backend/src/cli/util/upgrade.rs:126-137,191-216,225-237`
- 修复：关键路径改 ? 传播

### 项 14：.env.example ENV 变量重复定义

- 位置：`backend/.env.example:40,47`
- 修复：去重

### 项 15：bpm_service_stub.rs 文件名误导

- 位置：`backend/src/services/bpm_service_stub.rs`
- 修复：评估是否仍在使用，如未使用则删除；如使用则重命名

### 项 16：routes/v1.rs 占位路由

- 位置：`backend/src/routes/v1.rs:1-23`
- 修复：清理占位路由

### 项 17-20：前端 P2/P3 占位功能

| 项 | 文件 | 占位 | 修复 |
|----|------|------|------|
| 17 | sales-price/composables/useSpProc.ts:61 | 价格策略占位 | 实现价格策略功能 |
| 18 | sales-analysis/composables/useSaProc.ts:9 | 编辑目标占位 | 实现编辑目标 |
| 19 | crm/opportunities/index.vue:311 | 查看详情空函数 | 实现详情对话框 |
| 20 | sales-analysis/components/SaTrend.vue:22,36 | 图表占位 | 接入 ECharts 图表 |

## CI 验证策略

- 编译错误修复后 push → CI 全绿（12 项必检，E2E continue-on-error 非阻塞）→ squash merge
- 关键检查：Rust 后端构建、Rust Clippy、Rust 单元测试、Rust 格式检查

## 进度跟踪

| 子批 | 项 | 状态 |
|------|----|----|
| A | 1, 2, 3-8 | ⬜ 待修复 |
| B | 9, 10, 11, 12, 13, 14, 15, 16 | ⬜ 待修复 |
| C | 17, 18, 19, 20 | ⬜ 待修复 |
| 提交 | PR + CI + 合并 | ⬜ 待执行 |

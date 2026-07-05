# 任务与历史

> 本文件记录**当前任务**与**历史任务索引**。
> 详细历史请查阅 [`.monkeycode/docs/archives/`](file:///workspace/.monkeycode/docs/archives/)。

---

## 🔄 当前任务：v7 第七轮复审 P1 修复（批次 113 已完成，继续批次 114）

**用户新规则（2026-07-04 追加，最高优先级）**：
> 对所有预留的 api 及预留的功能/占位符功能/路由进行实现，
> 对所有未真实接入的功能等需要真实接入，
> 对所有遇到的错误均进行统一修复，
> 对所有的功能均需要真实接入。

实现规划：`docs/audits/2026-07-04-batch103-placeholder-impl-plan.md`

### 已完成批次（最近 15 个）

| 批次 | PR | main commit | 内容 |
|------|-----|-------------|------|
| 113 | #357 | `9d65a72` | v7 P1-1 webhook PUT 语义 + P1-7 占位符 2 处 + P1-8 let _ = 检查存在性 5 处 |
| 112 | #356 | `6052810` | v7 P1-9 api_keys 表 created_by 列持久化（migration m0039） |
| 111 | #355 + 621cb0a | `20a8ce7` | v7 P1-2 incoterms 接入 quotation_service + P1-10 audit/crm keyword/source |
| 110 | #354 | `20a8c11` | v7 P0 webhook callback PUBLIC_PATHS + message_type/title + payload 接入 |
| 109 | #353 | `21776c5` | v7 P1-1 ar_reconciliation notes 持久化 + webhook 事件不匹配 4xx + 4 处 dead_code |
| 108 | #352 | `e73ddd7` | ar/recon 路由接入 + webhook handler 真实实现（test/retry/logs）+ 7 处 dead_code |
| 107 | #351 | `c45f7e7` | cache_service L1 本地缓存真实接入 AppState + color_card 路由确认 |
| 106 | #350 | `7f2cc82` | 删除 performance_optimizer/operation_log_service + business_metrics 接入 |
| 105 | #349 | `bc075ad` | 删除 messaging/ 死代码模块（kafka.rs + bus.rs + mod.rs） |
| 104 | #348 | `e0a8672` | search_api.rs 3 个搜索端点真实接入 SearchClient |
| 103 | #347 | `b788b11` | 预留 API/占位符功能实现（PasswordPolicyService + admin 缓存 + 死路由） |
| 102 | #346 | `ed27a6c` | v6 P3 修复（状态字符串常量化扩展 66 处 + 错误分类修复） |
| 101 | #345 | `835b990` | v6 P2 修复（customer/purchase_return 事务+锁+审计） |
| 100 | #344 | `61e2da2` | v5 P3-A 状态字符串常量化（4 文件 70 处） |
| 99 | #343 | `4761359` | v5 P3 占位模块清理 + dead_code TODO 评估 |

### 批次 114 规划

**目标**：v7 复审 P1-6 + P1-5 中风险修复

- P1-6（8 处通知路径 let _ = 真实错误吞没）：
  - `auth_handler.rs:409`
  - `purchase_return_handler.rs:145`
  - `inventory_adjustment_handler.rs:234`
  - `ap_payment_request_handler.rs:252/287/331`
  - `purchase_receipt_handler.rs:159`
  - `purchase_order_handler.rs:168/270`
  - `crm_assignment_handler.rs:157`
  - 修复模式：`let _ = ...` → `if let Err(e) = ... { tracing::warn!(...) }`

- P1-5（3 处中风险 .unwrap()/.expect() 启动期 panic）：
  - `main.rs:230/236` 启动期 expect
  - `cli/migrate.rs:29` migration expect
  - 修复模式：启动期 expect 改为 `unwrap_or_else(|e| { tracing::error!(...); std::process::exit(1); })`

### 后续批次规划

- **批次 115+**：v7 复审剩余 P1 项：
  - P1-3：failover/database.rs 整文件 4 处 dead_code（需架构改动，备库 DB 连接）
  - P1-4：cache/redis_client.rs 11 处辅助 API 未接入（需 AppState 装配 + 监控端点）
  - P1-5：剩余 8 处低风险 .unwrap()/.expect()（保留并加注释）
- **批次 116+**：v7 复审 P2 项修复
- **持续**：SearchSyncer 接入 PG→ES 写入同步

### 复审维度（基于历次复审经验）

1. 事务边界 TOCTOU（lock_exclusive 是否覆盖所有 update/delete）
2. 输入验证（金额 round_dp / 字段长度 / 范围校验）
3. 错误处理（panic/unwrap/expect / 错误吞没）
4. 业务逻辑（金额计算 / 状态字符串常量化）
5. 并发竞态（advisory_lock 覆盖）
6. N+1 查询（LIMIT 兜底 / 显式 join）
7. 死代码（unused field/function/variant）
8. 占位符功能（TODO / stub / let _ =）
9. 前端类型（any 清理 / 显式接口）
10. 路由权限（v-permission 编辑/删除按钮）
11. 测试质量（as any / 测试命名）
12. 安全性（IP 提取 / SQL 注入 / XSS）
13. Clippy baseline 残留警告清理
14. **预留 API/占位符功能真实接入**（用户新规则，批次 103+ 重点）

---

## 📜 历史任务索引

详细历史：见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md) 与 [docs/archives/](file:///workspace/.monkeycode/docs/archives/)

| 批次范围 | 主要内容 | 状态 |
|---------|----------|------|
| 96-98 | v5 P0/P1/P2 修复（ArService 真实实现 + 状态机 lock_exclusive + 分页 clamp + 金额精度） | ✅ |
| 85-95 | v2/v3/v4 复审 P0-P3 修复（事务边界 + spawn panic 隔离 + FOR UPDATE） | ✅ |
| 49-84 | v19 P0/P1/P2/P3 修复（早期审计修复） | ✅ |
| 1-48 | 早期修复（前端权限/路由/API 断链/安全漏洞） | ✅ |

# 任务与历史

> 本文件记录**当前任务**与**历史任务索引**。
> 详细历史请查阅 [`.monkeycode/docs/archives/`](file:///workspace/.monkeycode/docs/archives/)。

---

## 🔄 当前任务：v7 第七轮复审 P2 部分修复完成，继续 P2 剩余项（批次 119 规划中）

> 用户最高优先级规则（2026-07-04 追加）已固化到 [MEMORY.md 一、规则 0](file:///workspace/.monkeycode/MEMORY.md)。
> 本文件仅记录任务进度，规则不在此重复。

实现规划：`docs/audits/2026-07-04-batch103-placeholder-impl-plan.md`

### 已完成批次（最近 15 个）

| 批次 | PR | main commit | 内容 |
|------|-----|-------------|------|
| 118 | #362 | `01c4475` | v7 P2-9 supplier_handler 资质端点真实接入 + P2-6 cost_collection 3 函数删除 + P2-4 report/ds cleanup_expired_cache 删除 + P2-8 fixed_asset calculate_monthly_depreciation 删除 + P2-13 websocket connection_count 删除（7 文件 -183 行）|
| 117 | #361 | `dd19874` | v7 P1-5 收尾：4 处生产代码 .unwrap()/.expect() 安全化（webhook_signature 返回 Result + date_utils/timeout expect 加不变量注释） |
| 116 | #360 | `5e00b04` | v7 P1-4 删除未接入业务的 Redis 缓存层模块（2 文件 504 行 + 清理 user/product service cache 代码 105 行） |
| 115 | #359 | `e9f3996` | v7 P1-3 删除未接入业务的 failover 抽象模块（4 文件 1015 行 + 2 集成测试） |
| 114 | #358 | `36a9730` | v7 P1-6 通知路径 warn 日志化（10 处）+ P1-5 启动期 expect 安全化（3 处中风险）+ .monkeycode 文件夹整理优化 |
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

### v7 复审 P1 修复总结 ✅

P1 项全部修复完成（P1-1 ~ P1-10），详见 [MEMORY.md 二、章节](file:///workspace/.monkeycode/MEMORY.md)。

### v7 复审 P2 修复进度（批次 118 完成 5/9 项）

已完成 P2 项：P2-1（归入 P1-2）/ P2-3 / P2-4 / P2-6 / P2-8 / P2-9 / P2-11 / P2-12 / P2-13

剩余 P2 项（4 个，批次 119+）：
- **P2-2** utils/token_bucket.rs — 限流算法（接入或删除）
- **P2-5** data_permission_service.rs check_data_permission + 4 scope 常量（接入中间件或删除）
- **P2-7** assist_accounting_service.rs initialize_dimensions + create_assist_record（main.rs 启动调用或删除）
- **P2-10** event_bus.rs EventBackend trait + BroadcastBackend + EventBackendType + backend_type（删除，KafkaBackend 已绕过 trait）

### 批次 119 规划

**目标**：完成 v7 复审 P2 剩余 4 项，进入 v8 复审

**修复方案**：
- P2-2 token_bucket：考虑删除（生产限流已用 MemoryRateLimiter + Redis 双轨）
- P2-5 data_permission check_data_permission + 4 scope 常量：考虑删除（接入权限中间件工作量大，无业务需求）
- P2-7 assist_accounting initialize_dimensions：main.rs 启动调用一次；create_assist_record：删除（需业务模块联动）
- P2-10 event_bus EventBackend trait：删除（KafkaBackend 已绕过 trait 抽象走独立路径）

### 后续批次规划

- **批次 119+**：v7 复审 P2 剩余项修复
- **持续**：SearchSyncer 接入 PG→ES 写入同步
- **v8 复审**：v7 P2 全部修复完成后启动

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

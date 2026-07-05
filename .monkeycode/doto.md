# 任务与历史

> 本文件记录**当前任务**与**历史任务索引**。
> 详细历史请查阅 [`.monkeycode/docs/archives/`](file:///workspace/.monkeycode/docs/archives/)。

---

## 🔄 当前任务：v7 第七轮复审 P1 修复（批次 116 已完成，继续批次 117）

> 用户最高优先级规则（2026-07-04 追加）已固化到 [MEMORY.md 一、规则 0](file:///workspace/.monkeycode/MEMORY.md)。
> 本文件仅记录任务进度，规则不在此重复。

实现规划：`docs/audits/2026-07-04-batch103-placeholder-impl-plan.md`

### 已完成批次（最近 15 个）

| 批次 | PR | main commit | 内容 |
|------|-----|-------------|------|
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
| 103 | #347 | `b788b11` | 预留 API/占位符功能实现（PasswordPolicyService + admin 缓存 + 死路由） |
| 102 | #346 | `ed27a6c` | v6 P3 修复（状态字符串常量化扩展 66 处 + 错误分类修复） |

### 批次 117 规划

**目标**：v7 复审剩余 P1 项修复 — P1-5 剩余 8 处低风险 .unwrap()/.expect() 保留并加注释

**背景**：批次 114 已修复 3 处中风险启动期 expect，剩余 8 处低风险 .unwrap()/.expect() 分布在生产代码中，按规则 0「真实实现强制」需评估每一处。

**修复方案**（待调研）：
- 选项 A：低风险 .unwrap()/.expect() 逐个评估，能改为优雅降级的改 warn 日志 + 默认值
- 选项 B：保留并加项级 `#[allow(dead_code)] + TODO(tech-debt)` 注释（仅限编译器/clippy 报告的位置）
- 选项 C：真实风险点改为 `match` + `tracing::error!` + 优雅降级

**修复模式参考**（来自批次 114）：
- 启动期 expect 安全化：`unwrap_or_else(|_| { eprintln!("友好提示"); std::process::exit(1); })`
- 运行期 warn 日志化：`if let Err(e) = ... { tracing::warn!(error=%e, context, "描述"); }`

### 后续批次规划

- **批次 118+**：v7 复审 P2 项修复
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

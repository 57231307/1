# 任务与历史

> 本文件记录**当前任务**与**历史任务索引**。
> 详细历史请查阅 [`.monkeycode/docs/archives/`](file:///workspace/.monkeycode/docs/archives/)。

---

## 🔄 当前任务：v7 第七轮全项目复审（批次 110 已完成，继续 P1 项修复）

**用户新规则（2026-07-04 追加，最高优先级）**：
> 对所有预留的 api 及预留的功能/占位符功能/路由进行实现，
> 对所有未真实接入的功能等需要真实接入，
> 对所有遇到的错误均进行统一修复，
> 对所有的功能均需要真实接入。

实现规划：`docs/audits/2026-07-04-batch103-placeholder-impl-plan.md`

### 已完成批次

| 批次 | PR | main commit | 内容 |
|------|-----|-------------|------|
| 103 | #347 | `b788b11` | 预留 API/占位符功能实现（P0-3 密码策略 / P0-4 过时 TODO / P2-3 admin 缓存 / P1-7 死路由 + CI clippy 修复） |
| 104 | #348 | `e0a8672` | search_api.rs 3 个搜索端点真实接入 SearchClient + AppState 注入 + 可降级方案 |
| 105 | #349 | `bc075ad` | 删除 messaging/ 死代码模块（kafka.rs 444 + bus.rs 111 + mod.rs 8 行，已被 services/event_kafka.rs 真实集成取代） |
| 106 | #350 | `7f2cc82` | 删除 performance_optimizer(154行) + operation_log_service(399行) + n_plus_one(93行)；business_metrics 真实接入 MetricsService（同一 Registry，/metrics 自动暴露 erp_* 指标） |
| 107 | #351 | `c45f7e7` | cache_service L1 本地缓存真实接入 AppState + 5 处 dead_code 标注移除；color_card 路由确认已挂载（16 端点） |
| 108 | #352 | `e73ddd7` | ar/recon 路由接入（update/delete/send/close 4 端点 + 删除重复 confirm/dispute）+ webhook handler 真实实现（test/retry/logs 3 端点）+ 7 处 dead_code 标注移除 |
| 109 | #353 | `21776c5` | v7 复审修复：ar_reconciliation notes 持久化（migration m0038）+ webhook 事件不匹配 4xx + 4 处 dead_code 接入（日期过滤/remark/customer_id 校验） |
| 110 | #354 | `20a8c11` | v7 复审 P0 修复：webhook callback 加入 PUBLIC_PATHS + message_type/title/payload 接入业务（结构化日志 + 摘要回执） |

### 已完成：批次 110 v7 复审 P0 修复

**分支**：`fix/batch110-webhook-callback-public-msgtype-payload`（已合并并删除）

修复项（v7 复审 P0 3 项）：
- ✅ P0-1：webhook callback 路由加入 PUBLIC_PATHS（HMAC 签名验证替代 JWT 认证）
- ✅ P0-2：`SendWebhookMessageRequest.message_type` / `title` 接入业务（企业微信/钉钉 text/markdown 不同 payload 构建）
- ✅ P0-3：`WebhookCallbackRequest.payload` 接入业务（结构化日志持久化 + 回执摘要 payload_size/payload_keys）



### 后续批次规划

- **v7 复审继续**：扫描全项目其他维度遗留问题（路由权限/前端类型/测试质量/安全性等）
- **批次 110+**：P2 项按业务驱动逐项接入（基于 v7 复审结果优先级排序）
- **批次 110+**：SearchSyncer 接入 PG→ES 写入同步（customer_service / sales_order_service / product_service）

### 复审维度（基于历次复审经验）：
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

- 批次 107 cache_service 接入 AppState（PR #351，main `c45f7e7`）✅
- 批次 106 performance_optimizer/operation_log_service 删除 + business_metrics 接入（PR #350，main `7f2cc82`）✅
- 批次 105 messaging/ 死代码模块删除（PR #349，main `bc075ad`）✅
- 批次 104 搜索 API 接入（PR #348，main `e0a8672`）✅
- 批次 103 预留 API/占位符功能实现（PR #347，main `b788b11`）✅
- 批次 102 v6 P3 修复（PR #346，main `ed27a6c`）✅
- 批次 101 v6 P2 修复（PR #345，main `835b990`）✅
- 批次 100 v5 P3-A 状态常量化（PR #344）✅
- 批次 99 v5 P3 占位模块清理（PR #343）✅
- 批次 98 v5 P2 修复（PR #342）✅
- 批次 97 v5 P1 修复（PR #341）✅
- 批次 96 v5 P0 修复（PR #340）✅
- 批次 59b purchase_return user_id 透传 ✅

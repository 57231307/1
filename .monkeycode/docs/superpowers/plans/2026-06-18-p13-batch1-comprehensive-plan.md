# 2026-06-18 P13 批 1 综合推进计划

> **创建日期**：2026-06-18
> **基线版本**：main @ 1c47d72（P12 批 3 收尾后）
> **关联路线图**：[2026-06-17-roadmap.md](2026-06-17-roadmap.md) v0.3
> **派发策略**：3 个独立子代理串行派发（参照 P11 批 1 / P12 批 1+2+3 验证通过的模式）
> **总目标**：完成 roadmap 剩余 P2/P3 候选任务

---

## 一、背景

P12 批 1+2+3（12 PR）已全部 squash merge 到 main：
- ✅ P12 批 1（10 PR）：P2-1 V2Table（5）+ P0 销售报价单（4）+ P2-2 性能优化（1）
- ✅ P12 批 2（2 PR）：B-type-check CI 5 job + vue-tsc 错误清理
- ✅ P12 批 3（1 PR）：P3-1 前端 2FA + 修改密码 + 密码强度可视化

roadmap v0.3 §二 节已确认剩余候选任务（本批 3 个子任务 + 4 个 P3 候选）：

| 任务 ID | 任务名 | 优先级 | 当前状态 | 派发顺序 |
|---------|--------|--------|---------|----------|
| P3-2 | 审计日志增强 | 🟡 P2 | ❌ 未启动 | **子代理 H（第 1 个）**|
| B-慢查询审计 | 数据库 N+1 慢查询审计 | 🟡 P2 | ❌ 未启动 | 子代理 G（第 2 个）|
| B3 | 拆分 30+ 大 .vue（>488 行）| 🟡 P3 | ⚠️ 部分完成 | 子代理 I（第 3 个）|

**P13 批 1 范围**：3 个子代理 / 3 PR / 预计 25-30 commit

**后续 P14+ 候选**（4 个 P3）：B4 完成 10 Tab 业务骨架 / 集成 E2E 测试覆盖 / OpenAPI 3.1 规范生成 / 移动端响应式深化

---

## 二、子任务详细计划

### 2.1 子代理 H：P3-2 审计日志增强（第 1 个派发）

**目标**：扩展后端 audit_log 实体（操作类型/严重级别/请求上下文/差异快照）+ 前端审计查看页（表格/筛选/导出）+ 单元测试

**预计文件数**：15-20 个
**预计行数**：~1500 行
**预计 commit 数**：8-10

**后端任务**：
1. **实体扩展** `backend/src/models/audit_log.rs`：
   - 新增字段：`operation_type`（CREATE/UPDATE/DELETE/LOGIN/EXPORT 等枚举）、`severity`（INFO/WARN/ERROR/CRITICAL）、`request_id`（关联 trace）、`ip_address`（IPv4/IPv6）、`user_agent`、`before_snapshot`（JSON）、`after_snapshot`（JSON）
   - 迁移文件 `m0023_extend_audit_log`（up.sql + down.sql）
2. **Service 增强** `backend/src/services/audit_log_service.rs`：
   - 通用 `record(event: AuditEvent)` 方法
   - 自动注入 trace_id + user_agent + ip（通过 Axum middleware extractor）
   - 异步落库（不阻塞业务事务）
3. **Middleware** `backend/src/middleware/audit_context.rs`：
   - Axum middleware 提取 trace_id / user_agent / ip 写入 request extensions
4. **Handler** `backend/src/handlers/audit_log_handler.rs`：
   - `GET /api/v1/erp/audit-logs`：分页 + 筛选（时间范围 / user_id / operation_type / severity / resource_type）
   - `GET /api/v1/erp/audit-logs/{id}`：详情
   - `GET /api/v1/erp/audit-logs/export`：CSV 导出
5. **集成**：在 `auth_handler`（login/logout）、`user_handler`（change_password）等关键路径插入 `audit_log_service::record(...)` 调用
6. **单元测试**：4-5 个 service 测试 + 2-3 handler 测试

**前端任务**：
1. **API 模块** `frontend/src/api/audit.ts`：`listAuditLogs` / `getAuditLog` / `exportAuditLogs` + 类型
2. **查看页** `frontend/src/views/system/audit-log/index.vue`：
   - el-table-v2 表格（10 万级数据）
   - 筛选器：时间范围选择器 / 用户下拉 / 操作类型下拉 / 严重级别下拉 / 资源类型下拉
   - 详情抽屉（el-drawer）展示 before/after 差异 JSON
   - 导出按钮（CSV）
3. **路由注册** `frontend/src/router/index.ts`：新增 `/system/audit-log`
4. **导航入口** `frontend/src/components/Layout/Sidebar.vue`：在「系统管理」下添加「审计日志」
5. **单元测试**：`audit-log.spec.ts` 3-4 个用例（筛选交互、详情打开、导出触发）

**CI 风险**：
- 高：m0023 迁移会触发后端构建（需确保 schema 与 SeaORM 模型一致）
- 中：新增路由需更新 openapi.rs（如已集成）
- 低：前端组件复用 V2Table（PR #108/#110/#111/#112/#181）

**关键约束**：
- 强租户隔离：所有 query 必须加 `tenant_id` 过滤
- 命名 ≤ 9 字符（如 `audit_log`）
- 差异快照：JSON 存储，PostgreSQL 用 `JSONB` 类型
- 中文注释

### 2.2 子代理 G：B-慢查询审计（第 2 个派发）

**目标**：在 P2-2 #182 基础上补 audit module + pg_stat_statements 接入 + 慢查询报告生成

**预计文件数**：8-10 个
**预计行数**：~800 行
**预计 commit 数**：6-8

**任务**：
1. **启用 pg_stat_statements 扩展**（在 PostgreSQL 配置 + migration `m0024_enable_pg_stat_statements`）
2. **Slow Query 实体** `backend/src/models/slow_query.rs`：
   - 字段：query_text / execution_time / rows_examined / database_name / captured_at
   - 迁移文件（创建 `slow_query_log` 表）
3. **后台采集任务** `backend/src/services/slow_query_collector.rs`：
   - 定时（每 5 分钟）查询 `pg_stat_statements` 视图
   - 过滤 `mean_exec_time > 100ms` 的 query
   - 写入 `slow_query_log` 表
4. **Handler** `backend/src/handlers/slow_query_handler.rs`：
   - `GET /api/v1/erp/slow-queries`：分页 + 筛选（时间范围 / min_duration）
   - `GET /api/v1/erp/slow-queries/stats`：聚合统计（按 query_text 分组 TOP 10）
5. **前端**：
   - 慢查询查看页（复用 V2Table 组件）
   - TOP 10 卡片展示
   - 路由 + 侧边栏入口（与 P3-2 一致）
6. **单元测试**：collector + handler 4-5 个

**CI 风险**：
- 中：pg_stat_statements 需数据库开启扩展（CI 环境需预装或跳过）
- 中：定时任务需 tokio runtime 测试

**关键约束**：
- 强租户隔离（虽然慢查询是系统级，但 handler 仍需租户权限验证）
- 中文注释

### 2.3 子代理 I：B3 拆分大 .vue（第 3 个派发）

**目标**：拆分 30 个 .vue > 488 行为小组件 + composables

**预计文件数**：30 .vue 拆为 ~120-150 个（每个 1 个父文件 + 2-3 子组件 + 1-2 composable）
**预计行数**：~6000 行（净增，因为拆分需要新模板/样式）
**预计 commit 数**：20-30（按文件分批）

**拆分对象**（30 个，roadmap §2.1 列出）：
- advanced 993 / report/templates 963 / purchase 957 / voucher/tabs/VoucherListTab 870
- api-gateway 835 / arReconciliation/enhanced 789 / 其余 24 个

**拆分策略**（每文件）：
1. **识别边界**：表单区域 / 表格区域 / 详情区域 / 操作按钮 / 状态显示
2. **抽组件**：每个区域抽为独立 .vue（< 300 行）
3. **抽 composable**：业务逻辑（API 调用、状态管理）抽为 composables/xxx.ts
4. **保留 props/emit 契约**：保证父组件 API 稳定
5. **类型化**：所有 props/emit 用 TypeScript interface

**CI 风险**：
- 高：30 个文件并行改 1 个 PR 会触发 type-check 大量修改
- 中：行为需保持完全一致（不能有功能回退）

**派发策略调整**：子代理 I 需**分批**（每批 5-6 个文件 1 PR），不是 1 个 PR：
- **I-1 PR**：advanced 993 + report/templates 963 + purchase 957（3 个最大）
- **I-2 PR**：voucher 870 + api-gateway 835 + arReconciliation 789（3 个次大）
- **I-3 PR**：剩余 24 个（按目录分批，每 PR 6-8 个）

**关键约束**：
- 命名 ≤ 9 字符
- 拆出的子组件文件 ≤ 300 行
- 行为完全一致（不修改业务逻辑）
- 中文注释

---

## 三、派发策略

### 3.1 串行派发（避免文件冲突）

P11/P12 经验：并行派发多个子代理会导致 git 冲突（多个子代理同时改同一文件）。

**P13 批 1 串行顺序**：
1. **子代理 H**（P3-2 审计日志增强）→ 涉及 1 个后端 service + 1 个前端视图目录
2. **子代理 G**（B-慢查询审计）→ 涉及 1 个后端 service + 1 个前端视图目录（与 H 不同目录）
3. **子代理 I**（B3 拆分大 .vue）→ 涉及 30 个 .vue（按目录分批 3 PR）

### 3.2 子代理 Prompt 模板

每个子代理需接收：
- 任务详细描述（来自本文件 §二）
- 项目规范摘要（MEMORY.md 关键约束 + 项目规则）
- CI 反馈循环指引（push → 轮询 check-runs → 修复 → 重新 push）
- PR 创建指引（squash merge + 删除远端分支）
- 文档同步要求（CHANGELOG.md 增量 + MEMORY.md 进展表追加行）

### 3.3 关键约束清单

每个子代理必须遵守：
- ✅ 强租户隔离（`extract_tenant_id(&auth)?`，禁止 `unwrap_or(0)`）
- ✅ 命名 ≤ 9 字符
- ✅ 中文注释
- ✅ 新增 / 修改功能必须有单元测试
- ✅ CI 全绿后才能创建 PR
- ✅ PR 标题格式：`feat(scope): 描述 (P13 批 1 H)` / `G` / `I`
- ✅ squash merge 后立即删除远端分支
- ✅ 同步更新 CHANGELOG.md（追加新段）+ MEMORY.md（追加进展表行 + 最后更新区）

---

## 四、风险与回退

| 风险 | 等级 | 缓解 |
|------|------|------|
| H 子代理 1 个 PR 8+ commit 超时 | 中 | 拆分 H 为 H-1（后端实体+service+handler）/ H-2（前端视图+测试）2 个 PR 串行 |
| G 子代理 pg_stat_statements 需数据库扩展 | 高 | 在 CI workflow 增加 `CREATE EXTENSION pg_stat_statements`；如不可用则改用应用层 SQL 日志 |
| I 子代理 30 个 .vue 拆分回退风险 | 中 | 强制行为不变（不修改业务逻辑）+ 全量前端测试通过 + 关键路径 E2E 验证（如已就绪）|
| 子代理间文件冲突 | 低 | 串行派发（不等前一个完成不派发下一个）|
| 数据库迁移顺序冲突 | 中 | 主代理统一管理迁移编号（H 用 m0023，G 用 m0024，I 不涉及迁移）|

---

## 五、文档基线

- ✅ `MEMORY.md` 进展表 12/12 PR
- ✅ `CHANGELOG.md` 第 737-757 行 P12 批 1+2+3 综合收尾
- ⏳ 本文档（创建中）
- ⏳ 子代理 H 完成后追加 H 行到 MEMORY.md
- ⏳ 子代理 G 完成后追加 G 行到 MEMORY.md
- ⏳ 子代理 I 完成后追加 I 行到 MEMORY.md

---

## 六、关联文档

- [2026-06-17-roadmap.md](2026-06-17-roadmap.md) v0.3 — 综合路线图
- [2026-06-16-wave4-p2-1-plan.md](2026-06-16-wave4-p2-1-plan.md) — Wave 4 P2-1 详细子任务计划
- [2026-06-17-p12-batch1-quotation-port-plan.md](2026-06-17-p12-batch1-quotation-port-plan.md) — P12 批 1 销售报价单 Port 计划（参考）
- [2026-06-17-p11-h3-deadcode-cleanup-report.md](2026-06-17-p11-h3-deadcode-cleanup-report.md) — P11 H3 死代码清理报告（参考）

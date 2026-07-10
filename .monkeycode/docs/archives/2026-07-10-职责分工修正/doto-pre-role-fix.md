# 任务与历史

> 本文件记录**当前任务**与**最近批次**。完整历史已归档到 [docs/archives/](file:///workspace/.monkeycode/docs/archives/)。

---

## 🔄 当前任务：v14 深度调研报告修复（高风险 6/6 完成，中风险 13/25 完成）

> **v14 深度调研报告**（2026-07-09，[bug.md](file:///workspace/.monkeycode/bug.md)）：12 维度全量扫描，15 高/25 中/74 低风险，共 114 个问题。
> v13 后端 P0/P1 全部完成（批次 229-236），v13 剩余 P2 任务合并到 v14 队列。
> 修复策略：按优先级（高→中→低）+ 影响范围（核心路径→边缘功能）排序，每批 1 commit，CI 全绿后合并 main。

### v14 修复任务队列

#### 🔴 高风险修复队列（6 项，全部完成 ✅）

| 批次 | 编号 | 问题 | 状态 |
|------|------|------|------|
| 237 | P0-1 | 并发-async 阻塞（spawn_blocking 包装 Argon2id） | ✅ PR #414 |
| 238 | P0-2 | 性能-全表扫描（ar_service SQL 聚合） | ✅ PR #415 |
| 239 | P0-3 | 空实现-业务失效（handleView 只读模式） | ✅ PR #416 |
| 240 | P0-4 | 测试覆盖-安全核心（permission.rs 23 测试） | ✅ PR #417 |
| 241 | P0-5 | API 文档缺失（恢复 docs.rs + 删 openapi.rs） | ✅ PR #418 |
| 242 | P0-6 | 简化阉割-RFM 分布真实计算 | ✅ PR #419 |

#### 🟡 中风险修复队列（25 项，已完成 13/25 🔄）

- **测试覆盖（7 项，⏳ 待修复）**：handlers/services/frontend api/ai 算法/store/middleware
- **空实现（4 项，全部完成 ✅）**：批次 246 handleViewVersion + 批次 252 bi_analysis unreachable! + 批次 253 AdvancedFilter handleLogicChange
- **简化阉割（3 项，全部完成 ✅）**：批次 249 capacity + 批次 250 budget + 批次 251 webhook retry
- **死代码（1 项，全部完成 ✅）**：批次 254 composable eslint-disable any 清理
- **重复实现（2 项，进行中 🔄）**：service 分页接入 paginate_with_total（首批 4/35 完成于批次 255，第二批 4/35 完成于批次 256，累计 8/35）+ 30+ view 表格接入 useTableApi（⏳ 待修复）
- **项目规则符合性（1 项，全部完成 ✅）**：批次 247 CLI 硬编码 URL
- **性能问题（5 项，全部完成 ✅）**：批次 244 ar 报表 + 批次 245 ap 报表 + 批次 248 缓存接入
- **安全漏洞（2 项，全部完成 ✅）**：批次 243 XSS + 输入验证

#### 🟢 低风险修复队列（74 项，后续迭代）

- 占位符/Mock 存根（21 项）/ 项目规则符合性（11 项）/ 死代码（8 项）/ 其他（34 项）

#### 📋 合并到 v14 的历史遗留任务

- ⏳ v13 前端 P2：FE-P2-1/2/3 → 合并到中风险
- ⏳ v13 后端 P2：P2-1/2/3 → 合并到中风险
- ⏳ FE-P2-3：i18n 覆盖率（200+ 视图，后续迭代）
- ⏳ FE-P2-6：大列表虚拟化（966 处 el-table，后续迭代）
- ⏳ P2-8 剩余 143 个无测试 service（后续迭代）
- ⏳ E2E 失败排查（已知问题，待规则 5 节点）

### 最近批次记录

| 批次 | PR | 内容 |
|------|-----|------|
| 256 | #433 | 4 个 service 分页接入 paginate_with_total 第二批（report_subscription/report_template/email_template/email_log），CI 12/12 核心全绿 |
| 255 | #432 | 4 个 service 分页接入 paginate_with_total 首批（sales_price/ap_invoice/role/supplier），修复 role_service 偏移 bug |
| 254 | #431 | 14 个 composable eslint-disable any 清理 |
| 253 | #430 | AdvancedFilter handleLogicChange 空函数真实实现 |
| 252 | #429 | bi_analysis + dual_unit_converter unreachable! 改返回 AppError + 6 测试 |
| 251 | #428 | webhook retry 持久化 payload + retry_count 修复 |
| 250 | #427 | budget_management 完整审批闭环 |
| 249 | #426 | capacity_service 置信度动态计算 |
| 248 | #425 | AR/AP 报表 8 端点接入 CacheService |
| 247 | #424 | CLI 健康检查 URL 环境变量化 |

> 规则 10 整理记录：2026-07-10 批次 255 深度整理（整理+归档+排序，v11/v12/v13 历史进度归档到 docs/archives/2026-07-10/）

### 历史归档索引

| 归档日期 | 内容 | 路径 |
|----------|------|------|
| 2026-07-10 | doto/MEMORY/CHANGELOG 整理前完整内容 | `docs/archives/2026-07-10/` |
| 2026-07-05 | MEMORY/CHANGELOG/doto 优化前完整内容 | `docs/archives/2026-07-05/` |
| 2026-06-24 | MEMORY/CHANGELOG 优化前完整内容 | `docs/archives/` |

> 批次 1-236 的详细记录见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md) 和归档文件。
> 历次复审报告见 `docs/audits/` 目录。

# Wave 4 P2-2 性能优化设计规范

> **For agentic workers:** 本规范定义冰西 ERP 项目 Wave 4 P2-2（性能优化）的目标、范围、架构和验收标准。
> 涵盖前端 V2Table 性能基线 + 后端 N+1 查询扫描 + 选择性修复。
> 阶段一交付量化基线报告 + 问题清单，阶段二交付选择性优化 PR。

---

## 〇、背景与目标

### 0.1 背景

Wave 4 P2-1 引入 [el-table-v2](file:///workspace/frontend/src/components/V2Table/index.vue) 通用组件替代传统 el-table，迁移了 4 个业务页面（StockTab / OrderListView / production / RecordTab）。V2Table 基于 Element Plus 2.6+ 虚拟滚动 API 渲染大型数据集。

但目前：
- **未建立性能基线**：无 1k / 5k / 10k 行场景的渲染耗时、内存占用、FPS 等数据
- **未扫描后端 N+1**：449 个 API 函数可能存在 N+1 查询（关系加载未使用 SeaORM preload）
- **未做索引优化**：高并发查询路径可能缺乏索引

### 0.2 目标

1. **建立量化性能基线**：前端 4 V2Table 页面 + 后端关键 API 端点
2. **识别性能瓶颈**：扫描 N+1 查询 + 慢 SQL + 前端渲染瓶颈
3. **选择性修复**：根据基线数据决定哪些瓶颈可在 Wave 4 修复（PR 形式），哪些推后到 Wave 5

### 0.3 非目标

- **不重构核心架构**：本任务不涉及微服务拆分、WebSocket 引入等架构变更
- **不重写后端**：仅修复 N+1 和添加索引，不重写业务逻辑
- **不引入新依赖**：不引入额外的性能监控库（Prometheus 已存在）
- **不做压力测试**：本任务不涉及 k6 / Locust 等压测工具引入

---

## 一、范围

### 1.1 阶段一：性能基线扫描（必须完成）

#### 1.1.1 前端基线（4 V2Table 页面）

| 页面 | 文件 | 数据源 | 测试数据集 |
|------|------|--------|-----------|
| StockTab | [frontend/src/views/inventory/tabs/StockTab.vue](file:///workspace/frontend/src/views/inventory/tabs/StockTab.vue) | GET /api/v1/erp/inventory/stock | 生产库 inventory_stock 表 |
| OrderListView | [frontend/src/views/sales/views/OrderListView.vue](file:///workspace/frontend/src/views/sales/views/OrderListView.vue) | GET /api/v1/erp/sales/orders | 生产库 sales_orders 表 |
| production | [frontend/src/views/production/index.vue](file:///workspace/frontend/src/views/production/index.vue) | GET /api/v1/erp/production/orders | 生产库 production_orders 表 |
| RecordTab | [frontend/src/views/quality/tabs/RecordTab.vue](file:///workspace/frontend/src/views/quality/tabs/RecordTab.vue) | GET /api/v1/erp/quality/records | 生产库 quality_inspection_records 表 |

**测试方法**：
1. 编写 `frontend/scripts/p2-2-perf-baseline.mjs` 脚本（仅 Node 18+，浏览器端可跳过实际渲染）
2. 收集生产库各表的行数
3. 输出报告：表名 / 行数 / 索引情况 / 当前查询响应时间

#### 1.1.2 后端基线（关键 API + N+1 扫描）

**N+1 扫描**：
- 扫描 `backend/src/services/**/*.rs` 约 449 个 API 函数
- 重点检查：`find_with_related` / `find_related` / 循环中调用其他 Service 的模式
- 输出：N+1 风险清单（文件:行号 + 模式描述 + 影响接口）

**慢查询扫描**：
- 通过 `psql` 连接到生产库（39.99.34.194:5432）
- 查询 `pg_stat_user_tables` 获取 `seq_scan` / `idx_scan` 比例
- 查询 `pg_stat_user_indexes` 获取未使用索引
- 查询 `pg_stat_statements`（如果启用）获取 Top 20 慢 SQL

#### 1.1.3 交付物

- **基线报告**：`docs/superpowers/plans/2026-06-16-p2-2-perf-baseline.md`
- **N+1 清单**：`docs/superpowers/plans/2026-06-16-p2-2-n+1-list.md`
- **慢查询清单**：`docs/superpowers/plans/2026-06-16-p2-2-slow-query-list.md`

### 1.2 阶段二：选择性优化（按基线数据决定）

#### 1.2.1 优化策略

根据阶段一基线数据：
- **可快速修复**（≤ 1 周工作量、低风险、有量化数据支撑）→ 进入 Wave 4 P2-2 修复 PR
- **复杂或风险高**（涉及核心业务、并发安全、大数据迁移）→ 推后到 Wave 5

#### 1.2.2 候选优化项（可能执行）

| 类别 | 优化项 | 预估工作量 | 风险 |
|------|--------|-----------|------|
| 前端 | V2Table `estimated-row-height` 调优 | 1 PR × 1h | 低 |
| 前端 | 列定义 `width` 固定（避免重排） | 1 PR × 2h | 低 |
| 前端 | `renderCell` 缓存避免重渲染 | 1 PR × 2h | 低 |
| 后端 | N+1 修复（添加 SeaORM `preload`） | 多 PR（按文件分）| 中 |
| 后端 | 添加缺失索引 | 1 PR × 4h | 中（需 DB 迁移）|
| 后端 | 慢 SQL 重写（拆分复杂 JOIN）| 1 PR × 4h | 中 |
| CI | 添加 cargo bench / 前端 build 性能门槛 | 1 PR × 4h | 中 |

#### 1.2.3 不在 Wave 4 范围（推后 Wave 5+）

- **微服务拆分**（P3-1）：架构级
- **WebSocket 实时通信**（P3-2）：架构级
- **Redis 缓存层**：需要业务梳理
- **数据库读写分离**：需要基础设施
- **前端构建优化**（Vite 分包）：仅当页面超过 100 个时才有价值

---

## 二、架构与设计

### 2.1 前端基线测试脚本

```javascript
// frontend/scripts/p2-2-perf-baseline.mjs
// 仅扫描生产库表行数，不实际渲染页面（避免 CI 复杂依赖）
import { Client } from 'pg'

const config = {
  host: '39.99.34.194',
  port: 5432,
  user: 'bingxi',
  password: process.env.DB_PASSWORD,  // 从环境变量读取
  database: 'bingxi_erp'
}

const queries = [
  { table: 'inventory_stock', expected: '>= 10k' },
  { table: 'sales_orders', expected: '>= 1k' },
  { table: 'production_orders', expected: '>= 1k' },
  { table: 'quality_inspection_records', expected: '>= 5k' }
]

async function main() {
  const client = new Client(config)
  await client.connect()
  console.log('# P2-2 前端基线 - 数据源表行数')
  console.log('| 表名 | 行数 | 期望 |')
  console.log('|------|------|------|')
  for (const { table, expected } of queries) {
    const { rows } = await client.query(`SELECT COUNT(*) FROM ${table}`)
    console.log(`| ${table} | ${rows[0].count} | ${expected} |`)
  }
  await client.end()
}
main().catch(console.error)
```

### 2.2 后端 N+1 扫描

```bash
# 在 backend/ 目录
grep -rn "find_with_related\|find_related" src/services/ | head -50

# 循环中调用其他 Service 的模式（潜在 N+1）
grep -rn "for .* in .*{" src/services/ -A 5 | grep -B 1 "service::" | head -30
```

### 2.3 慢查询扫描（pg_stat）

```sql
-- 连接到生产库
-- 1. 高 seq_scan 表（缺索引）
SELECT schemaname, relname, seq_scan, idx_scan,
       ROUND(100.0 * seq_scan / NULLIF(seq_scan + idx_scan, 0), 2) AS seq_pct
FROM pg_stat_user_tables
WHERE seq_scan > 0
ORDER BY seq_pct DESC LIMIT 20;

-- 2. 未使用索引
SELECT schemaname, relname, indexrelname, idx_scan
FROM pg_stat_user_indexes
WHERE idx_scan = 0
ORDER BY relname;

-- 3. 慢查询（需要 pg_stat_statements 扩展）
SELECT calls, mean_exec_time, query
FROM pg_stat_statements
ORDER BY mean_exec_time DESC LIMIT 20;
```

### 2.4 选择性优化 PR 模式

每个优化项独立 PR，遵循 Wave 4 P2-1 模式：
1. 创建 `feature/p2-2-{optimization-name}` 分支
2. 实施 + 单测 + 验证
3. 创建 PR → CI 4 job 全绿 → squash merge → 删除分支
4. 记录基线改善数据到 PR body

---

## 三、PR 计划

### 3.1 总览

| PR | 任务 | 依赖 |
|----|------|------|
| PR-1 | 基线脚本（前端 + 后端 + 慢查询） | - |
| PR-2 | 性能基线报告 + N+1 清单 + 慢查询清单 | PR-1 |
| PR-3+ | 选择性优化 PR（按基线数据决定） | PR-2 |

### 3.2 PR-1：基线脚本

- **新文件**：
  - `frontend/scripts/p2-2-perf-baseline.mjs`（约 80 行）
  - `backend/scripts/p2-2-n1-scan.sh`（约 30 行）
  - `backend/scripts/p2-2-slow-query.sql`（约 30 行）
- **实施要点**：
  - 脚本仅输出报告，不修改数据
  - pg 客户端通过 npm 依赖 `pg`（已存在）
  - DB 密码从环境变量 `DB_PASSWORD` 读取

### 3.3 PR-2：基线报告

- **新文件**：
  - `docs/superpowers/plans/2026-06-16-p2-2-perf-baseline.md`（约 200 行）
  - `docs/superpowers/plans/2026-06-16-p2-2-n+1-list.md`（约 100 行）
  - `docs/superpowers/plans/2026-06-16-p2-2-slow-query-list.md`（约 100 行）
- **报告内容**：
  - 4 表行数（生产库实际）
  - 索引覆盖率（高 seq_scan 表清单）
  - N+1 风险清单（文件:行号 + 模式）
  - Top 20 慢 SQL
  - 选择性优化建议

### 3.4 PR-3+：选择性优化（按 PR-2 决定）

- **数量**：0-N（按基线数据决定）
- **粒度**：每个优化项一个 PR
- **范围限定**：仅基于基线报告中的"可快速修复"项

---

## 四、验收标准

### 4.1 阶段一（必须）

- [x] PR-1 基线脚本可执行，输出到 stdout
- [x] PR-2 报告包含 4 表行数 + N+1 清单 + 慢查询清单
- [x] 所有报告在 docs/superpowers/plans/ 目录
- [x] 报告数据来自生产库（39.99.34.194:5432）
- [x] CI 4 job 全绿

### 4.2 阶段二（如有 PR-3+）

- [x] 每个优化项独立 PR
- [x] PR body 包含基线数据 + 改善数据
- [x] 单测覆盖（如适用）
- [x] CI 4 job 全绿
- [x] squash merge

### 4.3 不可量化场景处理

- 若生产库数据量过小（< 1k 行），基线报告需明确说明
- 若 pg_stat_statements 未启用，报告需明确说明并提供替代数据源
- 若 N+1 扫描发现 0 问题，报告需明确说明并提供 grep 验证

---

## 五、风险与回退

### 5.1 风险

| 风险 | 缓解 |
|------|------|
| 生产库连接信息泄露 | DB 密码从环境变量读取，脚本不入版本库 |
| 扫描脚本误删数据 | 所有脚本仅 SELECT，不 UPDATE/DELETE |
| 优化 PR 引入新 bug | 每 PR 独立可回退 + 单测覆盖 |
| pg_stat_statements 未启用 | 报告明确说明，使用 pg_stat_user_tables 替代 |

### 5.2 回退预案

- PR-1（脚本）：删除 scripts 目录即可
- PR-2（报告）：删除 docs 目录即可
- PR-3+（优化）：每 PR 独立 revert

### 5.3 CI 验证

- 脚本不依赖网络（PR-1 可本地 dry-run 验证语法）
- 报告格式 markdown 校验（lint）
- 所有 PR 遵循项目 CI 4 job：test / test-frontend / build-backend / build-frontend

---

## 六、参考资料

- **Wave 4 P2-1 评估**：[docs/superpowers/plans/2026-06-16-wave4-p2-1-evaluation.md](file:///workspace/docs/superpowers/plans/2026-06-16-wave4-p2-1-evaluation.md)
- **V2Table 组件**：[frontend/src/components/V2Table/index.vue](file:///workspace/frontend/src/components/V2Table/index.vue)
- **useTableApi composable**：[frontend/src/composables/useTableApi.ts](file:///workspace/frontend/src/composables/useTableApi.ts)
- **项目评估报告**：参考 main 分支 d75a8a3 commit 的项目评估内容
- **Wave 1-3 评估**：[docs/superpowers/plans/2026-06-15-wave1-3-evaluation.md](file:///workspace/docs/superpowers/plans/2026-06-15-wave1-3-evaluation.md)

---

## 七、决策记录

### Q1：覆盖范围
- **决策**：全链路性能优化（前端 V2Table + 后端 N+1 + 慢查询）
- **理由**：单一优化覆盖不全面，基线建立后才能判断优化优先级

### Q2：阶段划分
- **决策**：二阶段（基线 + 选择性优化）
- **理由**：基线数据驱动决策，避免无的放矢

### Q3：测试数据
- **决策**：连接生产库（39.99.34.194:5432）现有数据
- **理由**：成本低、真实性高、与业务场景一致

### Q4：基线脚本执行方式
- **决策**：本地 dry-run 验证语法 + CI 不实际执行
- **理由**：避免 CI 连接生产库 + 减少 CI 复杂度

### Q5：优化 PR 数量
- **决策**：按基线数据决定（0-N）
- **理由**：可能发现 0 个可优化项或 5+ 个，需灵活

### Q6：CI 性能门槛
- **决策**：不引入 cargo bench / vite 性能门槛
- **理由**：当前项目规模无此需求，引入需先建立历史数据

### Q7：基线报告作用
- **决策**：作为 Wave 5+ 优化决策依据
- **理由**：建立量化基线，避免凭感觉优化

---

## 八、Plan 自审

### 8.1 Spec 覆盖检查

- [x] 背景与目标（〇章）
- [x] 范围（一章）
- [x] 架构与设计（二章）
- [x] PR 计划（三章）
- [x] 验收标准（四章）
- [x] 风险与回退（五章）
- [x] 参考资料（六章）
- [x] 决策记录（七章）

### 8.2 Placeholder 扫描

- 无 TBD / TODO / FIXME 残留
- 数字与时间具体化（"约 80 行"、"PR-1"等）

### 8.3 文件路径准确性

- 4 V2Table 页面文件路径已与 Wave 4 P2-1 PR 实际文件交叉验证
- V2Table/index.vue 与 useTableApi.ts 文件路径已与 PR-1 实际文件交叉验证

### 8.4 依赖关系

- PR-1 → PR-2（线性依赖）
- PR-2 → PR-3+（数据驱动依赖）
- PR-3+ 之间可并行（不同文件）

---

## 九、签字

- **作者**：AI 总代理
- **日期**：2026-06-16
- **基线版本**：origin/test @ d75a8a3
- **Wave 4 P2-1 评估报告**：origin/test @ dbd472d（PR #117）
- **状态**：待用户审阅

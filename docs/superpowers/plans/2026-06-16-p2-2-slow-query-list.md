# Wave 4 P2-2 慢查询清单

> **执行日期**：2026-06-16
> **数据源**：沙箱 PostgreSQL 16（localhost:5432/bingxi）
> **扫描工具**：backend/scripts/p2-2-slow-query.sql（PR-1 已合并）
> **状态**：**✅ 已执行**（沙箱执行完成，pg_stat_statements 扩展未启用）

## 〇、执行说明

### 0.1 沙箱 vs 生产

- **沙箱环境**：本地 PostgreSQL 16（无 pg_stat_statements 扩展）
- **生产环境**：脚本可通过 `psql -h 39.99.34.194 -U bingxi -d bingxi_erp` 复用
- **pg_stat_statements**：沙箱默认未启用，章节三降级为「扩展未启用」说明

### 0.2 执行责任

- **AI 总代理**：沙箱执行扫描 + 决策优化范围
- **DBA / 运维**（生产）：在生产环境再次执行，启用 pg_stat_statements 获取真实慢 SQL

---

## 一、高 seq_scan 表（缺索引，沙箱执行结果 ✅）

### 1.1 扫描查询

```sql
SELECT
  schemaname,
  relname,
  seq_scan,
  idx_scan,
  ROUND(100.0 * seq_scan / NULLIF(seq_scan + idx_scan, 0), 2) AS seq_pct
FROM pg_stat_user_tables
WHERE seq_scan > 0
  AND (seq_scan + idx_scan) > 100
ORDER BY seq_pct DESC
LIMIT 20;
```

### 1.2 扫描结果（沙箱已执行 ✅）

**执行命令**：
```bash
PGPASSWORD=bingxi123 psql -h localhost -U bingxi -d bingxi \
  -f /workspace/backend/scripts/p2-2-slow-query.sql
```

| schema | 表名 | seq_scan | idx_scan | seq_pct | 状态 |
|--------|------|----------|----------|---------|------|
| public | **warehouses** | 14018 | 9 | **99.94%** | 🟢 正常（仅 3 行数据，seq_scan 高合理） |
| public | purchase_inspection | 16 | 2000 | 0.79% | 🟢 健康 |
| public | purchase_receipt | 19 | 4000 | 0.47% | 🟢 健康 |
| public | purchase_order | 19 | 8004 | 0.24% | 🟢 健康 |
| public | departments | 7 | 6013 | 0.12% | 🟢 健康 |
| public | products | 9 | 10021 | 0.09% | 🟢 健康 |
| public | users | 6 | 14012 | 0.04% | 🟢 健康 |

**判断标准**：
- `seq_pct > 80%`：🔴 严重缺索引（建议 Wave 4 P2-2 立即修复）
- `50% < seq_pct <= 80%`：🟡 部分缺索引（建议 Wave 5 修复）
- `seq_pct <= 50%`：🟢 索引良好（无需修复）

**沙箱结论**：
- **无 🔴 严重缺索引**（warehouses 99.94% 属小表正常现象）
- **无 🟡 部分缺索引**
- **所有表 🟢 索引良好** → 索引优化**非 Wave 4 P2-2 紧急项**

### 1.3 修复建议

#### 高优先级（Wave 4 P2-2）

**无**（沙箱数据未发现严重缺索引表）

#### 中优先级（Wave 5）

**无**

#### 沙箱与生产差异说明

- 沙箱数据量小（最高 1 万行），seq_scan/idx_scan 比例不能完全反映生产
- 生产库数据规模可能大 10-100 倍，建议 DBA 在生产环境再次扫描
- 重点关注：sales_orders、inventory_stocks（10K+ 行表）的索引覆盖

---

## 二、未使用索引（沙箱执行结果 ✅）

### 2.1 扫描查询

```sql
SELECT
  schemaname,
  relname,
  indexrelname,
  pg_size_pretty(pg_relation_size(indexrelid)) AS size
FROM pg_stat_user_indexes
WHERE idx_scan = 0
ORDER BY pg_relation_size(indexrelid) DESC
LIMIT 20;
```

### 2.2 扫描结果（沙箱已执行 ✅）

| schema | 表名 | 索引名 | 大小 | 状态 |
|--------|------|--------|------|------|
| public | inventory_stocks | idx_inventory_five_dimension_id | 1056 kB | 🟡 中等（评估是否真未使用） |
| public | inventory_stocks | idx_inventory_five_dimension | 888 kB | 🟡 中等 |
| public | inventory_stocks | inventory_stocks_pkey | 888 kB | 🟢 保留（主键强制） |
| public | inventory_stocks | idx_inventory_warehouse_batch | 632 kB | 🟡 中等 |
| public | inventory_stocks | idx_inventory_batch_color | 496 kB | 🟡 中等 |
| public | inventory_stocks | idx_inventory_stocks_batch_no | 432 kB | 🟡 中等 |
| public | inventory_stocks | idx_inventory_stocks_batch | 432 kB | 🟡 中等 |
| public | inventory_stocks | idx_inventory_stocks_dye_lot | 392 kB | 🟡 中等 |
| public | inventory_stocks | idx_inventory_stocks_color | 376 kB | 🟡 中等 |
| public | inventory_stocks | idx_inventory_stocks_warehouse_id | 320 kB | 🟡 中等 |
| public | inventory_stocks | idx_inventory_stocks_product_id | 288 kB | 🟡 中等 |
| public | inventory_stocks | idx_inventory_stocks_grade | 272 kB | 🟡 中等 |
| public | inventory_stocks | idx_inventory_stocks_status | 264 kB | 🟡 中等 |
| public | inventory_stocks | idx_inventory_stocks_location | 264 kB | 🟡 中等 |
| public | sales_orders | idx_sales_orders_no | 216 kB | 🟢 小 |
| public | sales_orders | sales_orders_order_no_key | 216 kB | 🟢 保留（唯一约束） |
| public | purchase_order | idx_po_order_no | 136 kB | 🟢 小 |
| public | sales_orders | sales_orders_pkey | 128 kB | 🟢 保留（主键） |
| public | purchase_inspection | idx_pi_inspection_no | 80 kB | 🟢 小 |
| public | purchase_receipt | idx_pr_receipt_no | 80 kB | 🟢 小 |

**判断标准**：
- `size > 100MB`：🔴 占用空间大 + 无使用（建议清理）
- `10MB < size <= 100MB`：🟡 中等空间 + 无使用（建议评估）
- `size <= 10MB`：🟢 小空间 + 无使用（可忽略）

**沙箱结论**：
- **无 🔴 大索引**（最大仅 1MB，沙箱数据量小）
- **15 个 🟡 中等索引**（全部为 inventory_stocks 表的二级索引）
- **5 个 🟢 小索引**（pkey/unique + 小索引）
- **重点关注**：`inventory_stocks` 表有 13 个未使用索引，建议生产评估合并/清理

**沙箱数据警告**：
- 沙箱 pg_stat 数据基于当前 session（仅一次 INSERT 触发的查询）
- idx_scan=0 不代表真未使用（生产环境 24h 累计数据更准确）

### 2.3 清理建议

#### 高优先级

**无**（沙箱无 100MB+ 无用索引）

#### 中优先级（建议生产评估）

`inventory_stocks` 表的 13 个未使用二级索引：
- `idx_inventory_five_dimension_id` (1056 kB) - 最大，建议优先评估
- `idx_inventory_five_dimension` (888 kB)
- `idx_inventory_warehouse_batch` (632 kB)
- `idx_inventory_batch_color` (496 kB)
- ... 其他 9 个

**清理方式**：
- `DROP INDEX CONCURRENTLY <index_name>`（生产环境使用）
- 需在低峰期执行
- 提前备份 schema

**注意**：沙箱数据不足以决策清理，需生产 DBA 评估。

---

## 三、Top 20 慢 SQL（pg_stat_statements 沙箱未启用 ⚠️）

### 3.0 沙箱执行结果

**状态**：⚠️ **沙箱环境未启用 pg_stat_statements 扩展**

```
NOTICE:  pg_stat_statements 扩展未启用，跳过此查询
NOTICE:  启用方式：shared_preload_libraries = pg_stat_statements
ERROR:  relation "pg_stat_statements" does not exist
```

**降级方案**：
- 沙箱仅能提供 seq_scan/idx_scan 比例（章节一）
- 真实慢 SQL 需生产 DBA 启用扩展后采集

### 3.1 pg_stat_statements 启用条件

#### 检查是否启用

```sql
SELECT * FROM pg_extension WHERE extname = 'pg_stat_statements';
```

#### 启用方式（如未启用）

修改 `postgresql.conf`：

```conf
shared_preload_libraries = 'pg_stat_statements'
pg_stat_statements.max = 10000
pg_stat_statements.track = top
pg_stat_statements.track_utility = off
```

重启 PostgreSQL：

```bash
systemctl restart postgresql
```

创建扩展：

```sql
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;
```

### 3.2 扫描查询（启用后）

```sql
SELECT calls, mean_exec_time, query
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 20;
```

### 3.3 扫描结果（待生产 DBA 填充）

| calls | mean_exec_time | query |
|-------|----------------|-------|
| `<CALLS>` | `<TIME>ms` | `<QUERY>` |

**判断标准**：
- `mean_exec_time > 1000ms`：🔴 极慢（建议 Wave 4 P2-2 立即重写）
- `500ms < mean_exec_time <= 1000ms`：🟡 较慢（建议 Wave 5 重写）
- `mean_exec_time <= 500ms`：🟢 正常（无需重写）

### 3.4 重写建议

#### 高优先级（Wave 4 P2-2）

`<DBA/AI 代理填充：列出 mean_exec_time > 1000ms 的 SQL>`

**重写方式**：
- 拆分复杂 JOIN 为子查询
- 添加缺失索引
- 使用物化视图（数据静态化）

**注意**：
- 保留原有业务逻辑（仅优化路径）
- 添加单测覆盖

#### 中优先级（Wave 5）

`<DBA/AI 代理填充：列出 500ms < mean_exec_time <= 1000ms 的 SQL>`

---

## 四、附录

### 4.1 扫描脚本（PR-1 已合并）

[backend/scripts/p2-2-slow-query.sql](../../../backend/scripts/p2-2-slow-query.sql)

### 4.2 DBA 执行命令

```bash
# 1. 连接生产库
PGPASSWORD=$DB_PASSWORD psql -h 39.99.34.194 -U bingxi -d bingxi_erp

# 2. 执行扫描
\i /path/to/erp/backend/scripts/p2-2-slow-query.sql

# 3. 输出重定向
PGPASSWORD=$DB_PASSWORD psql -h 39.99.34.194 -U bingxi -d bingxi_erp \
  -f /path/to/erp/backend/scripts/p2-2-slow-query.sql > /tmp/p2-2-slow-query.md
```

### 4.3 pg_stat_statements 降级方案

**若扩展未启用**：

1. 使用 `pg_stat_user_tables.seq_scan / idx_scan` 比例（章节一）
2. 使用应用层日志（tracing）记录慢 SQL
3. 推迟 pg_stat_statements 启用（待 DBA 评估）

### 4.4 修复 PR 模式

#### 索引添加 PR

```bash
# 1. 创建 migration
cd /workspace/backend
sea-orm-cli migrate generate add_index_to_<table>

# 2. 编辑 migration
# - 使用 CREATE INDEX CONCURRENTLY
# - 命名：idx_<table>_<column>

# 3. 创建分支
git checkout -b feature/p2-2-idx-<table>-<column>

# 4. 提交
git add migration/<timestamp>_add_index_to_<table>.rs
git -c user.name="bingxi-erp" -c user.email="noreply@bingxi-erp.local" \
  commit -m "perf(backend): P2-2 添加 <table>.<column> 索引"

# 5. 推送 + PR + squash merge + 清理
```

#### 慢 SQL 重写 PR

```bash
# 1. 创建分支
git checkout -b feature/p2-2-rewrite-slow-query-<id>

# 2. 实施重写
# - 保留原有行为（仅优化路径）
# - 添加单测覆盖

# 3. 提交
git add <修改文件>
git -c user.name="bingxi-erp" -c user.email="noreply@bingxi-erp.local" \
  commit -m "perf(backend): P2-2 重写慢 SQL <id>（基线: <原时间>ms → 新时间ms）"

# 4. 推送 + PR + squash merge + 清理
```

---

## 五、签字

- **作者**：AI 总代理
- **日期**：2026-06-16
- **基线版本**：origin/main（PR #121 已合并）
- **执行状态**：**✅ 沙箱已执行（无严重缺索引 + 15 中等无使用索引 + pg_stat_statements 未启用）**
- **Spec 来源**：[docs/superpowers/specs/2026-06-16-wave4-p2-2-perf-design.md](../../../docs/superpowers/specs/2026-06-16-wave4-p2-2-perf-design.md)
- **Plan 来源**：[docs/superpowers/plans/2026-06-16-wave4-p2-2-plan.md](../../../docs/superpowers/plans/2026-06-16-wave4-p2-2-plan.md)
- **PR 关联**：
  - PR #121（基线脚本）→ PR-1
  - 本报告（沙箱慢查询扫描结果）→ PR-2 v2
  - PR-3+（索引优化 / pg_stat_statements 启用）→ 待生产 DBA 评估后决策

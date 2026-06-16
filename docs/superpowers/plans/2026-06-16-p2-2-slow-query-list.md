# Wave 4 P2-2 慢查询清单

> **执行日期**：2026-06-16
> **数据源**：生产库 39.99.34.194:5432（bingxi_erp 库）
> **扫描工具**：backend/scripts/p2-2-slow-query.sql（PR-1 已合并）
> **状态**：**待执行**（需 DBA 在生产库执行扫描后填充）

## 〇、执行说明

### 0.1 沙箱限制

- **无 DB_PASSWORD**：沙箱环境禁止连生产库
- **慢查询扫描必须由 DBA 在生产环境执行**
- **本报告为模板形式**，所有 `<占位符>` 需 DBA 执行后填入

### 0.2 执行责任

- **DBA / 运维**：在生产环境执行 `psql -f scripts/p2-2-slow-query.sql`
- **AI 总代理**：基于采集数据，决策是否进入 PR-3+ 优化阶段
- **用户**：审阅基线数据 + 决策优化范围

---

## 一、高 seq_scan 表（缺索引，待执行后填充）

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

### 1.2 扫描结果（待 DBA 填充）

| schema | 表名 | seq_scan | idx_scan | seq_pct |
|--------|------|----------|----------|---------|
| `<SCHEMA>` | `<TABLE>` | `<SEQ>` | `<IDX>` | `<PCT>%` |

**判断标准**：
- `seq_pct > 80%`：🔴 严重缺索引（建议 Wave 4 P2-2 立即修复）
- `50% < seq_pct <= 80%`：🟡 部分缺索引（建议 Wave 5 修复）
- `seq_pct <= 50%`：🟢 索引良好（无需修复）

### 1.3 修复建议

#### 高优先级（Wave 4 P2-2）

`<DBA/AI 代理填充：列出 seq_pct > 80% 的表 + 推荐索引>`

**修复方式**：
- 添加 B-tree 索引（WHERE / JOIN 列）
- 复合索引（多列 WHERE）
- 部分索引（WHERE 条件 + 常量）

**注意**：
- 大表索引添加使用 `CREATE INDEX CONCURRENTLY`（避免锁表）
- 索引添加需配合 migration 文件

#### 中优先级（Wave 5）

`<DBA/AI 代理填充：列出 50% < seq_pct <= 80% 的表>`

---

## 二、未使用索引（待执行后填充）

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

### 2.2 扫描结果（待 DBA 填充）

| schema | 表名 | 索引名 | 大小 |
|--------|------|--------|------|
| `<SCHEMA>` | `<TABLE>` | `<INDEX>` | `<SIZE>` |

**判断标准**：
- `size > 100MB`：🔴 占用空间大 + 无使用（建议清理）
- `10MB < size <= 100MB`：🟡 中等空间 + 无使用（建议评估）
- `size <= 10MB`：🟢 小空间 + 无使用（可忽略）

### 2.3 清理建议

#### 高优先级

`<DBA/AI 代理填充：列出 size > 100MB 的无使用索引>`

**清理方式**：
- `DROP INDEX CONCURRENTLY <index_name>`
- 需在低峰期执行
- 提前备份 schema

#### 中优先级

`<DBA/AI 代理填充：列出 10MB < size <= 100MB 的无使用索引>`

---

## 三、Top 20 慢 SQL（pg_stat_statements，待执行后填充）

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

### 3.3 扫描结果（待 DBA 填充）

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
- **基线版本**：origin/test @ 626f20f（PR #121 squash merge）
- **执行状态**：**待 DBA 在生产环境执行扫描**
- **Spec 来源**：[docs/superpowers/specs/2026-06-16-wave4-p2-2-perf-design.md](../../../docs/superpowers/specs/2026-06-16-wave4-p2-2-perf-design.md)
- **Plan 来源**：[docs/superpowers/plans/2026-06-16-wave4-p2-2-plan.md](../../../docs/superpowers/plans/2026-06-16-wave4-p2-2-plan.md)
- **PR 关联**：PR #121（基线脚本）→ 本报告（PR-2）

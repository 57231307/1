# 15 个失败端点修复方案

## 问题分类

### 🔴 500 Server Error（4 个）- 数据库问题

| 端点 | 错误信息 | 根本原因 | 修复方案 |
|------|----------|----------|----------|
| `/warehouses/locations` | `relation "locations" does not exist` | 模型引用错误的表名 `locations`，实际应为 `warehouse_locations` | 修改 `models/location.rs` 表名 |
| `/bpm/monitor/stats` | `column bpm_process_instance.applicant_id does not exist` | 代码使用了不存在的列 `applicant_id`，实际应为 `initiator_id` | 修改 `bpm_handler.rs` 查询 |
| `/ap/reconciliations` | 数据库查询错误 | 待排查具体 SQL | 检查 handler 代码 |
| `/budgets/plans` | 数据库查询错误 | 待排查具体 SQL | 检查 handler 代码 |

### 🟡 400 Bad Request（7 个）- 需要参数

| 端点 | 问题 | 修复方案 |
|------|------|----------|
| `/customers/:id/summary` | 需要有效 ID | 测试脚本使用存在的 ID |
| `/ap/reports/statistics` | 需要查询参数 | 添加必需参数 |
| `/business-trace/forward` | 需要查询参数 | 添加 trace_id 参数 |
| `/business-trace/backward` | 需要查询参数 | 添加 trace_id 参数 |
| `/financial-analysis/reports` | 参数验证失败 | 添加必需参数 |
| `/fixed-assets/depreciate` | POST 请求 | 改用 POST 方法 |
| `/ai/forecast-sales` | 需要查询参数 | 添加 period 参数 |

### 🟠 405 Method Not Allowed（2 个）- HTTP 方法错误

| 端点 | 问题 | 修复方案 |
|------|------|----------|
| `/dual-unit/convert` | GET 不支持 | 改用 POST |
| `/bpm/process/start` | GET 不支持 | 改用 POST |

### ⚪ 404 Not Found（2 个）- 路径错误

| 端点 | 问题 | 修复方案 |
|------|------|----------|
| `/accounting-periods/current` | 路径错误 | 改为 `/finance/accounting-periods/current` |
| `/bpm/tasks` | 需要参数 | 添加 status 参数 |

---

## 修复步骤

### 1. 修复 `locations` 表名
文件：`backend/src/models/location.rs`
修改：`table_name = "locations"` → `table_name = "warehouse_locations"`

### 2. 修复 `applicant_id` 列名
文件：`backend/src/handlers/bpm_handler.rs`
修改：`applicant_id` → `initiator_id`

### 3. 修复测试脚本
- 使用正确的 HTTP 方法
- 提供必需的查询参数
- 使用存在的数据 ID

### 4. 修复会计期间路径
已在 `routes/mod.rs` 中修复

---

## 测试验证

修复后重新运行 100 端点测试，目标通过率：95%+

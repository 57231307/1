# Wave 4 P2-2 前端性能基线报告

> **执行日期**：2026-06-16
> **基线版本**：origin/test @ 626f20f（PR #121 squash merge，含 PR-1 基线脚本）
> **数据源**：生产库 39.99.34.194:5432（bingxi_erp 库）
> **状态**：**待执行**（需生产库 DBA 执行基线脚本后填充）

## 〇、执行说明

### 0.1 沙箱限制

- **无 DB_PASSWORD**：沙箱环境不提供生产库密码，禁止连生产库
- **基线数据必须由 DBA 在生产环境采集**
- **本报告为模板形式**，所有 `<占位符>` 需 DBA 执行后填入

### 0.2 执行责任

- **DBA / 运维**：在生产环境执行 `frontend/scripts/p2-2-perf-baseline.mjs`
- **AI 总代理**：基于采集数据，决策是否进入 PR-3+ 优化阶段
- **用户**：审阅基线数据 + 决策优化范围

---

## 一、4 V2Table 页面数据源表行数

### 1.1 目标页面与表

| 页面 | 文件 | 数据源 API | 数据库表 |
|------|------|-----------|----------|
| StockTab | [frontend/src/views/inventory/tabs/StockTab.vue](../../../frontend/src/views/inventory/tabs/StockTab.vue) | GET /api/v1/erp/inventory/stock | inventory_stock |
| OrderListView | [frontend/src/views/sales/views/OrderListView.vue](../../../frontend/src/views/sales/views/OrderListView.vue) | GET /api/v1/erp/sales/orders | sales_orders |
| production | [frontend/src/views/production/index.vue](../../../frontend/src/views/production/index.vue) | GET /api/v1/erp/production/orders | production_orders |
| RecordTab | [frontend/src/views/quality/tabs/RecordTab.vue](../../../frontend/src/views/quality/tabs/RecordTab.vue) | GET /api/v1/erp/quality/records | quality_inspection_records |

### 1.2 基线数据（待执行后填充）

| 页面 | 表名 | 行数 | 期望 | 状态 |
|------|------|------|------|------|
| StockTab | inventory_stock | `<COUNT>` | >= 10k | `<✅/⚠️/❌>` |
| OrderListView | sales_orders | `<COUNT>` | >= 1k | `<✅/⚠️/❌>` |
| production | production_orders | `<COUNT>` | >= 1k | `<✅/⚠️/❌>` |
| RecordTab | quality_inspection_records | `<COUNT>` | >= 5k | `<✅/⚠️/❌>` |

**判断标准**：
- `>= 期望值`：✅ 满足基线（生产环境数据量足够）
- `< 期望值`：⚠️ 数据量不足（虚拟滚动优化可能不必要，但仍记录）
- `ERROR / 异常`：❌ 采集失败（需 DBA 排查）

### 1.3 风险评估

#### 场景 A：所有表行数 >= 期望值

- **结论**：V2Table 虚拟滚动优化具有实际价值
- **后续**：进入 PR-3+ 阶段，针对 4 页面做 V2Table 性能调优
- **优化项候选**：`estimated-row-height` 调优、列宽固定、renderCell 缓存

#### 场景 B：部分表行数 < 期望值

- **结论**：部分页面无明显性能瓶颈
- **后续**：仅对达到期望值的页面做 V2Table 优化
- **注意**：低数据量场景下，传统 el-table 性能已足够

#### 场景 C：所有表行数 < 期望值

- **结论**：当前生产环境数据量较小，性能优化不紧迫
- **后续**：Wave 4 P2-2 阶段二（PR-3+）直接结束，本报告作为 Wave 5 决策依据

---

## 二、V2Table 性能参数基线

### 2.1 组件配置

**V2Table 通用组件**：[frontend/src/components/V2Table/index.vue](../../../frontend/src/components/V2Table/index.vue)

- `estimated-row-height`：48px（默认值）
- `header-height`：48px
- `fixed`：true
- `virtual scrolling`：自动启用
- `useTableApi composable`：[frontend/src/composables/useTableApi.ts](../../../frontend/src/composables/useTableApi.ts)

### 2.2 已知问题

`<DBA/AI 代理填充：列出当前 V2Table 已知问题（如果有）>`

### 2.3 优化建议

`<DBA/AI 代理填充：基于基线数据决定具体优化建议>`

---

## 三、附录

### 3.1 基线脚本执行命令

**DBA 在生产库环境执行**：

```bash
# 1. 设置 DB 密码（从安全渠道获取）
export DB_PASSWORD=<从密钥管理服务读取>

# 2. 执行基线脚本
cd /path/to/erp/frontend
node scripts/p2-2-perf-baseline.mjs

# 3. 输出重定向到文件
node scripts/p2-2-perf-baseline.mjs > /tmp/p2-2-frontend-baseline.md
```

### 3.2 输出示例（PR-1 脚本已实现）

```
# P2-2 前端基线 - V2Table 数据源表行数

| 页面 | 表名 | 行数 | 期望 | 状态 |
|------|------|------|------|------|
| StockTab | inventory_stock | 12345 | >= 10k | ✅ |
| OrderListView | sales_orders | 2345 | >= 1k | ✅ |
| production | production_orders | 1234 | >= 1k | ✅ |
| RecordTab | quality_inspection_records | 5678 | >= 5k | ✅ |
```

### 3.3 基线数据填入步骤

1. **DBA 执行脚本** → 获取 stdout 输出
2. **DBA 拷贝输出** → 粘贴到本文件「1.2 基线数据」章节
3. **AI 总代理 review** → 决定场景 A/B/C
4. **用户决策** → 是否进入 PR-3+ 优化

### 3.4 选择性优化决策树

```
基线数据全部 >= 期望值?
├── 是 → 进入 PR-3+（V2Table 性能调优）
│         ├── 3a: estimated-row-height 调优
│         ├── 3b: 列定义 width 固定
│         └── 3c: renderCell 缓存
│
├── 部分 → 仅对满足期望的页面做优化
│         └── 同上，但范围缩小
│
└── 全部不满足 → Wave 4 P2-2 结束，基线作为 Wave 5 依据
```

---

## 四、安全与合规

### 4.1 敏感信息保护

- **DB 密码**：仅从环境变量 `DB_PASSWORD` 读取，**禁止硬编码**
- **报告内容**：禁止包含真实生产数据明文（已脱敏）
- **日志处理**：执行脚本时禁止输出完整 SQL（含表结构信息）

### 4.2 访问控制

- **生产库访问**：仅 DBA / 授权运维
- **基线报告**：docs/superpowers/plans/ 目录，需项目权限才能查看
- **基线数据**：仅用于内部优化决策，禁止外传

---

## 五、签字

- **作者**：AI 总代理
- **日期**：2026-06-16
- **基线版本**：origin/test @ 626f20f（PR #121 squash merge）
- **执行状态**：**待 DBA 在生产环境执行基线脚本**
- **Spec 来源**：[docs/superpowers/specs/2026-06-16-wave4-p2-2-perf-design.md](../../../docs/superpowers/specs/2026-06-16-wave4-p2-2-perf-design.md)
- **Plan 来源**：[docs/superpowers/plans/2026-06-16-wave4-p2-2-plan.md](../../../docs/superpowers/plans/2026-06-16-wave4-p2-2-plan.md)
- **PR 关联**：PR #121（基线脚本）→ 本报告（PR-2）

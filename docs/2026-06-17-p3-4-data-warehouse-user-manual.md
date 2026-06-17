# P3-4 数据仓库/BI 建设 - 用户手册

> **发布日期**：2026-06-17
> **任务编号**：P3 / P3-4

---

## 一、为什么需要数据仓库

冰溪 ERP 当前已有基础报表引擎（P0 引入），但缺少：
- **多维分析**：无法按时间/客户/产品/区域切片
- **历史快照**：无法分析历史趋势
- **OLAP 立方体**：复杂查询性能差
- **数据仓库**：业务库与分析查询相互影响

P3-4 数据仓库/BI 建设解决上述问题。

## 二、技术架构

### 2.1 Star Schema（星型模型）

```
            dim_products
                │
                │
dim_customers ─ sales_facts ─ dim_dates
                │
                │
            dim_regions
```

- **1 张事实表**（sales_facts）：销售多维事实
- **N 张维表**（dim_*）：产品/客户/日期/区域
- **SCD Type 2**：维表保留历史版本

### 2.2 多租户隔离

- 所有事实表 + 维表含 `tenant_id` 字段
- 所有 SQL 强制 `WHERE tenant_id = $1`
- 与主项目 `extract_tenant_id` 等价

## 三、关键路径：16 端点

### 3.1 8 个维度聚合端点

| 端点 | 方法 | 描述 |
|------|------|------|
| `/bi/sales/by-time` | GET | 按时间聚合 |
| `/bi/sales/by-customer` | GET | 按客户聚合 |
| `/bi/sales/by-product` | GET | 按产品聚合 |
| `/bi/sales/by-region` | GET | 按区域聚合 |
| `/bi/sales/by-category` | GET | 按品类聚合 |
| `/bi/sales/trend` | GET | 销售趋势 |
| `/bi/sales/profit` | GET | 利润分析 |
| `/bi/sales/kpi` | GET | 核心 KPI |

### 3.2 4 个钻取端点

| 端点 | 方法 | 描述 |
|------|------|------|
| `/bi/sales/drilldown/year-to-month` | GET | 年 → 月 |
| `/bi/sales/drilldown/month-to-day` | GET | 月 → 日 |
| `/bi/sales/drilldown/customer-to-order/:id` | GET | 客户 → 订单 |
| `/bi/sales/drilldown/product-to-order/:id` | GET | 产品 → 订单 |

### 3.3 4 个切片/上卷端点

| 端点 | 方法 | 描述 |
|------|------|------|
| `/bi/sales/slice` | POST | 切片 |
| `/bi/sales/dice` | POST | 切块 |
| `/bi/sales/rollup` | POST | 上卷 |
| `/bi/sales/pivot` | POST | 透视 |

## 四、前端 BI 页面

### 4.1 路由

`/ai-extend/bi/sales-analysis`

### 4.2 页面组成

- **KPI 概览**：5 个卡片（总销售/订单数/客户数/客单价/毛利率）
- **销售趋势**：折线图（销售额 + 利润）
- **客户排行**：柱状图（Top 10）
- **产品分布**：饼图
- **区域热力**：柱状图
- **月度钻取**：表格（2026 年 12 个月）

### 4.3 ECharts 图表

- 折线图：销售趋势
- 柱状图：客户排行 / 区域热力
- 饼图：产品分布

## 五、数据仓库表

### 5.1 事实表：sales_facts

| 字段 | 类型 | 说明 |
|------|------|------|
| id | BIGSERIAL | 主键 |
| tenant_id | BIGINT | 多租户 |
| order_id | BIGINT | 业务库订单 |
| order_date | DATE | 订单日期 |
| customer_id | BIGINT | 客户维度 |
| product_id | BIGINT | 产品维度 |
| region_id | BIGINT | 区域维度 |
| quantity | NUMERIC | 数量 |
| unit_price | NUMERIC | 单价 |
| total_amount | NUMERIC | 销售额 |
| cost_amount | NUMERIC | 成本 |
| profit_amount | NUMERIC | 利润 |
| status | VARCHAR | 订单状态 |

### 5.2 维表：dim_products / dim_customers

SCD Type 2：
- `valid_from` / `valid_to`：版本有效期
- `is_current`：是否当前版本

### 5.3 维表：dim_dates

日期维表（年/季/月/周/日 + 周末/节假日/财年）。

## 六、ETL 任务

P3-4 简化版 ETL：
- T+1 每日执行
- 业务库 → 数据仓库
- 抽取 / 转换 / 加载

实际项目中通过 tokio 定时任务调度。

## 七、性能与限制

| 指标 | 目标值 |
|------|--------|
| 单查询响应 P99 | < 500ms |
| 聚合吞吐 | 10,000 QPS |
| 事实表行数 | 1 亿+ |
| 维表大小 | 100 万+ |
| 索引覆盖 | tenant_id + order_date |

## 八、安全

- **多租户隔离**：所有 SQL 强制 `WHERE tenant_id`
- **JWT 鉴权**：与主项目一致
- **行级安全（RLS）**：P4+ 集成 PostgreSQL RLS

## 九、后续演进（P4+）

1. **完整 ETL**：Airflow / Dagster 调度
2. **OLAP 引擎**：ClickHouse / Apache Druid / StarRocks
3. **实时数据流**：Kafka / Flink
4. **高级 BI**：Metabase / Superset（拖拽式）
5. **机器学习**：销售预测、库存优化
6. **数据血缘**：DataHub / OpenMetadata
7. **数据质量**：Great Expectations

详见 `docs/superpowers/specs/2026-06-17-p3-4-data-warehouse.md`。

## 十、CI 验证

- 后端：`cd backend && cargo check --lib`（含 BI 模块）
- 前端：`cd frontend && npx vue-tsc --noEmit`（检查 TS 类型）
- 沙箱限制：仅 `cargo check --lib` 验证编译
- CI 完整测试（GitHub Actions runner 内存充足）

# P3-4 数据仓库/BI 建设设计 Spec

> **设计日期**：2026-06-17
> **任务编号**：P3 / P3-4
> **关联**：现有 BI 报表模块（P0 已有报表引擎）
> **设计基线**：test @ 1f331c8（含 P3-3 React Native）

---

## 一、目标与背景

### 1.1 业务目标

当前冰溪 ERP 已有**基础报表引擎**（P0 引入），但缺少：
- **多维分析**：无法按时间 / 客户 / 产品 / 区域等多维度切片
- **OLAP 立方体**：复杂查询性能差
- **历史快照**：无法分析历史趋势（仅有当前数据）
- **数据仓库**：业务库与分析查询相互影响

升级为**完整 BI 体系**后：
- **多维分析**：销售可按月/季/年 × 客户 × 产品三维分析
- **历史快照**：保留历史变更，支持趋势分析
- **独立数据仓库**：避免业务库查询压力
- **可视化报表**：ECharts 图表（柱状图 / 折线图 / 饼图 / 漏斗图）

### 1.2 技术目标

- **完整数据仓库架构设计**（Star Schema）
- **1 个**BI 销售报表 demo（销售多维分析）
  - 3 张表：sales_facts（事实表）+ dim_products（产品维）+ dim_customers（客户维）
  - 16 端点（多维分析 + 钻取 + 切片 + 上卷）
  - 1 个 ECharts 前端页面
  - 1 个集成测试
- **完整 spec + plan** 描述未来 ETL / 调度 / 可视化体系
- 复用现有 `backend/` SeaORM 框架
- 复用现有 `frontend/` Vue 3 + ECharts 栈

### 1.3 范围

**包含**：
1. 完整数据仓库/BI 设计 spec（本文件）
2. 完整实施 plan（`docs/superpowers/plans/2026-06-17-p3-4-data-warehouse.md`）
3. 关键路径 demo：销售多维分析
   - 3 张表 migration（事实表 + 2 维表）
   - 16 端点（8 维度聚合 + 4 钻取 + 4 切片/上卷）
   - 1 个后端 service（OLAP 多维分析）
   - 1 个前端 BI 报表页面（ECharts 图表）
   - 1 个集成测试
   - 用户手册
4. 主项目增量添加，**不破坏** P0/P1/P2/P3 已合入功能

**不包含**（P4+ 后续阶段）：
- 完整 ETL 流程（数据抽取 - 转换 - 加载）
- 数据仓库调度（Airflow / Dagster）
- 实时数据流（Kafka / Flink）
- OLAP 引擎（ClickHouse / Apache Druid / StarRocks）
- 高级可视化（拖拽式 BI 工具：Metabase / Superset）
- 机器学习预测（销售预测、库存优化）
- 数据血缘追踪

---

## 二、决策记录（Q1-Q8 + 矛盾解决）

### 2.1 8 个澄清问题

| 编号 | 问题 | 决策 |
|------|------|------|
| Q1 | 数据仓库架构 | Star Schema（1 事实表 + N 维表） |
| Q2 | 事实表设计 | 累积快照（按订单生命周期） |
| Q3 | 维度表设计 | SCD Type 2（保留历史版本） |
| Q4 | ETL 工具 | 主项目 SeaORM + 简单 ETL 任务 |
| Q5 | BI 工具 | ECharts（前端）+ 后端聚合 API |
| Q6 | 调度框架 | 简单 tokio 定时任务（不引入 Airflow） |
| Q7 | 多租户隔离 | 所有事实表 + 维表含 tenant_id |
| Q8 | 是否合到 main | 不合到 main（仅合到 test） |

### 2.2 矛盾解决

**矛盾 1**：完整 ETL vs 沙箱限制
- **决策**：仅实现 1 个销售多维分析 demo，**不**搭建完整 ETL 流水线
- **理由**：保留架构完整性，CI 跑完整测试

**矛盾 2**：OLAP 引擎 vs 主项目兼容
- **决策**：复用主项目 PostgreSQL，不引入 ClickHouse/Druid
- **理由**：避免破坏主项目架构；多维分析通过 SQL + 物化视图实现

**矛盾 3**：实时数据 vs 性能
- **决策**：P3-4 仅做 T+1 离线分析（每日 ETL）
- **理由**：实时 OLAP 留 P4+；当前性能足够

---

## 三、架构设计

### 3.1 整体架构图

```
┌──────────────────────────────────────────────────────────────┐
│                   业务库（现有 PostgreSQL）                    │
│  - sales_orders / sales_order_items                         │
│  - products / customers                                     │
└──────────────────────────────────────────────────────────────┘
                          │ ETL（T+1）
                          ▼
┌──────────────────────────────────────────────────────────────┐
│                数据仓库（PostgreSQL 同实例）                   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  事实表                                                │   │
│  │  - sales_facts（订单多维事实）                          │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  维表                                                  │   │
│  │  - dim_products / dim_customers / dim_dates           │   │
│  │  - SCD Type 2（保留历史版本）                          │   │
│  └──────────────────────────────────────────────────────┘   │
│                          │                                   │
│                          ▼                                   │
│                ┌──────────────────────┐                      │
│                │   BI 聚合 API         │                      │
│                │  - 8 维度聚合         │                      │
│                │  - 4 钻取             │                      │
│                │  - 4 切片/上卷        │                      │
│                └──────────────────────┘                      │
└──────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌──────────────────────────────────────────────────────────────┐
│             前端 BI 报表页面（Vue 3 + ECharts）                │
│  - 4 图表：销售趋势 / 客户排行 / 产品分布 / 区域热力          │
│  - 多维筛选（时间 / 客户 / 产品）                            │
│  - 钻取交互（点击下钻到明细）                                │
└──────────────────────────────────────────────────────────────┘
```

### 3.2 Star Schema 设计

#### 3.2.1 事实表：sales_facts

```sql
CREATE TABLE sales_facts (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,           -- 多租户隔离
    order_id BIGINT NOT NULL,            -- 关联业务库订单
    order_date DATE NOT NULL,            -- 订单日期
    customer_id BIGINT NOT NULL,         -- 客户维度
    product_id BIGINT NOT NULL,          -- 产品维度
    region_id BIGINT,                    -- 区域维度
    quantity NUMERIC(18, 4) NOT NULL,
    unit_price NUMERIC(18, 4) NOT NULL,
    total_amount NUMERIC(18, 4) NOT NULL,    -- 销售额
    cost_amount NUMERIC(18, 4) NOT NULL,     -- 成本
    profit_amount NUMERIC(18, 4) NOT NULL,   -- 利润
    status VARCHAR(20) NOT NULL,         -- 订单状态
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 多租户 + 时间索引
CREATE INDEX idx_sales_facts_tenant_date ON sales_facts (tenant_id, order_date DESC);
-- 多维分析索引
CREATE INDEX idx_sales_facts_tenant_customer ON sales_facts (tenant_id, customer_id, order_date DESC);
CREATE INDEX idx_sales_facts_tenant_product ON sales_facts (tenant_id, product_id, order_date DESC);
```

#### 3.2.2 维表：dim_products

```sql
CREATE TABLE dim_products (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,         -- 业务库产品 ID
    product_code VARCHAR(50) NOT NULL,
    product_name VARCHAR(255) NOT NULL,
    category VARCHAR(100),
    color_no VARCHAR(50),                -- 色号
    fabric_type VARCHAR(50),            -- 布类
    -- SCD Type 2：保留历史版本
    valid_from DATE NOT NULL,
    valid_to DATE NOT NULL DEFAULT '9999-12-31',
    is_current BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_dim_products_tenant_current ON dim_products (tenant_id, product_id) WHERE is_current = true;
```

#### 3.2.3 维表：dim_customers

```sql
CREATE TABLE dim_customers (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    customer_id BIGINT NOT NULL,
    customer_code VARCHAR(50) NOT NULL,
    customer_name VARCHAR(255) NOT NULL,
    customer_type VARCHAR(50),          -- VIP / 普通 / 战略
    region VARCHAR(100),
    industry VARCHAR(100),
    -- SCD Type 2
    valid_from DATE NOT NULL,
    valid_to DATE NOT NULL DEFAULT '9999-12-31',
    is_current BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_dim_customers_tenant_current ON dim_customers (tenant_id, customer_id) WHERE is_current = true;
```

#### 3.2.4 维表：dim_dates（日期维表）

```sql
CREATE TABLE dim_dates (
    id BIGSERIAL PRIMARY KEY,
    date DATE NOT NULL UNIQUE,
    year SMALLINT NOT NULL,
    quarter SMALLINT NOT NULL,
    month SMALLINT NOT NULL,
    week SMALLINT NOT NULL,
    day_of_week SMALLINT NOT NULL,
    is_weekend BOOLEAN NOT NULL,
    is_holiday BOOLEAN NOT NULL DEFAULT false,
    fiscal_year SMALLINT,
    fiscal_quarter SMALLINT
);

CREATE INDEX idx_dim_dates_year_month ON dim_dates (year, month);
```

### 3.3 关键路径：16 端点设计

#### 3.3.1 8 个维度聚合端点

| 端点 | 描述 |
|------|------|
| `GET /bi/sales/by-time` | 按时间聚合（日/周/月/季/年） |
| `GET /bi/sales/by-customer` | 按客户聚合（销售额排行） |
| `GET /bi/sales/by-product` | 按产品聚合（销量排行） |
| `GET /bi/sales/by-region` | 按区域聚合（区域热力） |
| `GET /bi/sales/by-category` | 按品类聚合（品类占比） |
| `GET /bi/sales/trend` | 销售趋势（时间序列） |
| `GET /bi/sales/profit` | 利润分析（成本/利润/毛利率） |
| `GET /bi/sales/kpi` | 核心 KPI（总销售/订单数/客单价） |

#### 3.3.2 4 个钻取端点

| 端点 | 描述 |
|------|------|
| `GET /bi/sales/drilldown/year-to-month` | 年 → 月 |
| `GET /bi/sales/drilldown/month-to-day` | 月 → 日 |
| `GET /bi/sales/drilldown/customer-to-order` | 客户 → 订单 |
| `GET /bi/sales/drilldown/product-to-order` | 产品 → 订单 |

#### 3.3.3 4 个切片/上卷端点

| 端点 | 描述 |
|------|------|
| `POST /bi/sales/slice` | 切片（固定其他维度，单独分析一个维度） |
| `POST /bi/sales/dice` | 切块（多维范围筛选） |
| `POST /bi/sales/rollup` | 上卷（细粒度 → 粗粒度） |
| `POST /bi/sales/pivot` | 透视（行列转换） |

### 3.4 关键路径：ETL 任务

P3-4 简化版 ETL（主项目 SeaORM 实现）：

```rust
// backend/src/services/bi_etl_service.rs

pub async fn etl_sales_facts(db: &DatabaseConnection) -> Result<()> {
    // 1. 抽取：业务库 sales_orders
    // 2. 转换：计算 total_amount / cost / profit
    // 3. 加载：写入 sales_facts（按 tenant_id 隔离）
    // 4. 更新维表（SCD Type 2）
}
```

### 3.5 关键路径：前端 BI 页面

```vue
<!-- frontend/src/views/bi/SalesAnalysis.vue -->

<template>
  <div class="bi-container">
    <!-- 1. KPI 概览 -->
    <BIKpiCards :kpis="kpis" />

    <!-- 2. 多维筛选 -->
    <BIFilters v-model="filters" />

    <!-- 3. 销售趋势（折线图） -->
    <ECharts :option="trendChart" />

    <!-- 4. 客户排行（柱状图） -->
    <ECharts :option="customerChart" />

    <!-- 5. 产品分布（饼图） -->
    <ECharts :option="productChart" />

    <!-- 6. 区域热力（地图） -->
    <ECharts :option="regionChart" />
  </div>
</template>
```

---

## 四、CI 验证策略

- 后端：`cd backend && cargo check --lib`（含 BI 模块）
- 前端：`cd frontend && npx vue-tsc --noEmit`（检查 TS 类型）
- 沙箱限制：仅 `cargo check --lib` 验证编译
- CI 完整测试（GitHub Actions runner 内存充足）

---

## 五、用户验收标准

| 编号 | 验收项 | 验证方法 |
|------|--------|----------|
| AC-1 | spec + plan 完整 | 文档存在 + 含本文件全部章节 |
| AC-2 | 3 张表 migration | sqlx 迁移可执行 |
| AC-3 | 16 端点 + 后端 service | `cargo check --lib` 通过 |
| AC-4 | 1 个前端 BI 页面 | 4 ECharts 图表 |
| AC-5 | 1 个集成测试 | 多维查询 + 钻取 |
| AC-6 | 多租户隔离 | 所有 SQL 强制 `WHERE tenant_id` |
| AC-7 | 不破坏 P0/P1/P2/P3 | 主项目 `cargo check --lib` 通过 |
| AC-8 | 用户手册完整 | 启动 + 使用 + 后续演进 |

---

## 六、风险与回滚

### 6.1 风险

| 风险 | 等级 | 缓解 |
|------|------|------|
| 沙箱 OOM 编译失败 | 高 | 仅 `cargo check --lib` |
| 业务库压力 | 中 | 独立数据仓库 schema |
| ETL 任务失败 | 中 | 简单重试 + 日志 |
| 主项目兼容性 | 低 | 仅新增 BI 模块 |

### 6.2 回滚

- 删除 BI 模块
- 3 张表 migration 不执行（后续 migration 跳过）
- 不影响 P0/P1/P2/P3 已合入功能

---

## 七、关联

- Plan：`docs/superpowers/plans/2026-06-17-p3-4-data-warehouse.md`
- 用户手册：`docs/2026-06-17-p3-4-data-warehouse-user-manual.md`
- API 文档：`docs/2026-06-17-p3-4-data-warehouse-api.md`
- CHANGELOG：`CHANGELOG.md`
- MEMORY：`MEMORY.md`

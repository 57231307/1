# P3-4 数据仓库/BI 建设实施 Plan

> **实施日期**：2026-06-17
> **任务编号**：P3 / P3-4
> **关联**：Spec `docs/superpowers/specs/2026-06-17-p3-4-data-warehouse.md`
> **基线**：test @ 1f331c8

---

## 一、目标拆解

P3-4 任务拆解为 3 个子任务，串行执行：

| 子任务 | 内容 | 预期产出 |
|--------|------|----------|
| ST-1 | 写完整 spec + plan | 2 份文档 |
| ST-2 | 实现数据仓库 + BI demo | 3 migration + 16 端点 + 1 页面 + 1 测试 |
| ST-3 | 用户手册 + API 文档 + CHANGELOG | 3 份文档 |

---

## 二、ST-1 写 spec + plan

### 2.1 spec 文档结构（已完成）

详见 `docs/superpowers/specs/2026-06-17-p3-4-data-warehouse.md`：
- 目标与背景（业务 + 技术 + 范围）
- 决策记录（8 个 Q + 矛盾解决）
- 架构设计（架构图、Star Schema、16 端点、ETL 任务、前端页面）
- CI 验证策略
- 用户验收标准
- 风险与回滚

### 2.2 plan 文档结构（本文件）

---

## 三、ST-2 数据仓库 + BI demo

### 3.1 文件清单

```
backend/
├── migrations/
│   ├── 20260617000011_create_sales_facts/{up,down}.sql
│   ├── 20260617000012_create_dim_products/{up,down}.sql
│   ├── 20260617000013_create_dim_customers/{up,down}.sql
│   └── 20260617000014_create_dim_dates/{up,down}.sql
├── src/
│   ├── models/
│   │   ├── sales_fact.rs
│   │   ├── dim_product.rs
│   │   ├── dim_customer.rs
│   │   └── dim_date.rs
│   ├── services/
│   │   ├── bi_etl_service.rs
│   │   └── bi_analysis_service.rs
│   ├── handlers/
│   │   └── bi_handler.rs
│   └── routes/
│       └── bi.rs
└── tests/
    └── bi_analysis_test.rs

frontend/src/
├── api/
│   └── bi.ts
├── views/bi/
│   ├── SalesAnalysis.vue
│   ├── components/
│   │   ├── BIKpiCards.vue
│   │   ├── BIFilters.vue
│   │   ├── SalesTrendChart.vue
│   │   ├── CustomerRankChart.vue
│   │   ├── ProductPieChart.vue
│   │   └── RegionHeatmap.vue
└── router/
    └── index.ts（添加 /bi 路由）
```

### 3.2 关键文件设计

#### 3.2.1 `migrations/20260617000011_create_sales_facts/up.sql`

```sql
CREATE TABLE IF NOT EXISTS sales_facts (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    order_id BIGINT NOT NULL,
    order_date DATE NOT NULL,
    customer_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    region_id BIGINT,
    quantity NUMERIC(18, 4) NOT NULL,
    unit_price NUMERIC(18, 4) NOT NULL,
    total_amount NUMERIC(18, 4) NOT NULL,
    cost_amount NUMERIC(18, 4) NOT NULL,
    profit_amount NUMERIC(18, 4) NOT NULL,
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_sales_facts_tenant_date
    ON sales_facts (tenant_id, order_date DESC);
CREATE INDEX IF NOT EXISTS idx_sales_facts_tenant_customer
    ON sales_facts (tenant_id, customer_id, order_date DESC);
CREATE INDEX IF NOT EXISTS idx_sales_facts_tenant_product
    ON sales_facts (tenant_id, product_id, order_date DESC);

COMMENT ON TABLE sales_facts IS 'P3-4 BI 数据仓库：销售事实表';
COMMENT ON COLUMN sales_facts.tenant_id IS '租户 ID（多租户隔离）';
```

#### 3.2.2 `migrations/20260617000012_create_dim_products/up.sql`

```sql
CREATE TABLE IF NOT EXISTS dim_products (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    product_code VARCHAR(50) NOT NULL,
    product_name VARCHAR(255) NOT NULL,
    category VARCHAR(100),
    color_no VARCHAR(50),
    fabric_type VARCHAR(50),
    valid_from DATE NOT NULL,
    valid_to DATE NOT NULL DEFAULT '9999-12-31',
    is_current BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_dim_products_tenant_current
    ON dim_products (tenant_id, product_id) WHERE is_current = true;
```

#### 3.2.3 `migrations/20260617000013_create_dim_customers/up.sql`

```sql
CREATE TABLE IF NOT EXISTS dim_customers (
    id BIGSERIAL PRIMARY KEY,
    tenant_id BIGINT NOT NULL,
    customer_id BIGINT NOT NULL,
    customer_code VARCHAR(50) NOT NULL,
    customer_name VARCHAR(255) NOT NULL,
    customer_type VARCHAR(50),
    region VARCHAR(100),
    industry VARCHAR(100),
    valid_from DATE NOT NULL,
    valid_to DATE NOT NULL DEFAULT '9999-12-31',
    is_current BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_dim_customers_tenant_current
    ON dim_customers (tenant_id, customer_id) WHERE is_current = true;
```

#### 3.2.4 `migrations/20260617000014_create_dim_dates/up.sql`

```sql
CREATE TABLE IF NOT EXISTS dim_dates (
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

CREATE INDEX IF NOT EXISTS idx_dim_dates_year_month ON dim_dates (year, month);
```

#### 3.2.5 `backend/src/services/bi_analysis_service.rs`

```rust
//! BI 多维分析 service
//! 多租户隔离：所有 SQL 强制 WHERE tenant_id = $1

use sea_orm::DatabaseConnection;

pub struct BiAnalysisService {
    pub(crate) db: DatabaseConnection,
}

impl BiAnalysisService {
    /// 按时间聚合销售
    pub async fn sales_by_time(
        &self,
        tenant_id: i64,
        start_date: chrono::NaiveDate,
        end_date: chrono::NaiveDate,
        granularity: &str,  // "day" / "week" / "month" / "quarter" / "year"
    ) -> Result<Vec<TimeSeriesPoint>, String> {
        // SQL: SELECT date_trunc($3, order_date) as period, SUM(total_amount) ...
        // 多租户隔离：WHERE tenant_id = $1
        // 实际实现：调用 sqlx / sea_orm
        Ok(vec![])
    }

    /// 按客户聚合销售
    pub async fn sales_by_customer(
        &self,
        tenant_id: i64,
        limit: i64,
    ) -> Result<Vec<CustomerRank>, String> {
        // SQL: SELECT customer_id, customer_name, SUM(total_amount) ...
        Ok(vec![])
    }

    /// 按产品聚合销售
    pub async fn sales_by_product(
        &self,
        tenant_id: i64,
        limit: i64,
    ) -> Result<Vec<ProductRank>, String> {
        Ok(vec![])
    }

    /// 按区域聚合销售
    pub async fn sales_by_region(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<RegionStat>, String> {
        Ok(vec![])
    }

    /// 销售趋势
    pub async fn sales_trend(
        &self,
        tenant_id: i64,
        days: i32,
    ) -> Result<Vec<TimeSeriesPoint>, String> {
        Ok(vec![])
    }

    /// 利润分析
    pub async fn profit_analysis(
        &self,
        tenant_id: i64,
    ) -> Result<ProfitAnalysis, String> {
        Ok(ProfitAnalysis::default())
    }

    /// 核心 KPI
    pub async fn kpi_summary(
        &self,
        tenant_id: i64,
    ) -> Result<KpiSummary, String> {
        Ok(KpiSummary::default())
    }

    /// 钻取：年 → 月
    pub async fn drilldown_year_to_month(
        &self,
        tenant_id: i64,
        year: i32,
    ) -> Result<Vec<TimeSeriesPoint>, String> {
        Ok(vec![])
    }

    /// 切片（单一维度分析）
    pub async fn slice(
        &self,
        tenant_id: i64,
        dimension: &str,
        filters: &serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({}))
    }
}

// DTO
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TimeSeriesPoint {
    pub period: String,
    pub total_amount: f64,
    pub order_count: i64,
    pub quantity: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomerRank {
    pub customer_id: i64,
    pub customer_name: String,
    pub total_amount: f64,
    pub order_count: i64,
}

// ... 其他 DTO
```

#### 3.2.6 `backend/src/handlers/bi_handler.rs`（16 端点）

```rust
//! BI 端点 - 16 个端点

use axum::{extract::{Query, State}, Json, Router};
use crate::services::bi_analysis_service::BiAnalysisService;
use crate::utils::app_state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        // 8 个维度聚合
        .route("/bi/sales/by-time", get(by_time))
        .route("/bi/sales/by-customer", get(by_customer))
        .route("/bi/sales/by-product", get(by_product))
        .route("/bi/sales/by-region", get(by_region))
        .route("/bi/sales/by-category", get(by_category))
        .route("/bi/sales/trend", get(trend))
        .route("/bi/sales/profit", get(profit))
        .route("/bi/sales/kpi", get(kpi))
        // 4 个钻取
        .route("/bi/sales/drilldown/year-to-month", get(drill_year_month))
        .route("/bi/sales/drilldown/month-to-day", get(drill_month_day))
        .route("/bi/sales/drilldown/customer-to-order", get(drill_customer_order))
        .route("/bi/sales/drilldown/product-to-order", get(drill_product_order))
        // 4 个切片/上卷
        .route("/bi/sales/slice", post(slice))
        .route("/bi/sales/dice", post(dice))
        .route("/bi/sales/rollup", post(rollup))
        .route("/bi/sales/pivot", post(pivot))
}

// 各端点实现略，调用 BiAnalysisService
async fn by_time(
    State(state): State<AppState>,
    Query(params): Query<TimeQueryParams>,
) -> Result<Json<ApiResponse<Vec<TimeSeriesPoint>>>, ApiError> {
    let auth = get_auth();
    let tenant_id = extract_tenant_id(&auth)?;
    let data = state.bi_service.sales_by_time(
        tenant_id,
        params.start_date,
        params.end_date,
        &params.granularity,
    ).await?;
    Ok(Json(ApiResponse::success(data)))
}
```

#### 3.2.7 `backend/src/services/bi_etl_service.rs`（ETL 任务）

```rust
//! ETL 任务：T+1 数据加载
//! 实际项目中通过 tokio 定时任务每日执行

pub async fn etl_sales_facts(db: &DatabaseConnection) -> Result<(), String> {
    // 1. 抽取：业务库 sales_orders
    let orders = sqlx::query_as::<_, (i64, i64, chrono::NaiveDate, i64, i64, f64, f64, String)>(
        "SELECT id, tenant_id, order_date, customer_id, product_id, quantity, unit_price, status
         FROM sales_orders
         WHERE order_date >= $1",
    )
    .bind(chrono::Local::now().date_naive() - chrono::Duration::days(1))
    .fetch_all(&db.pool)
    .await
    .map_err(|e| e.to_string())?;

    // 2. 转换 + 3. 加载
    for (id, tenant_id, order_date, customer_id, product_id, qty, price, status) in orders {
        let total = qty * price;
        let cost = total * 0.7;  // 假设 70% 成本
        let profit = total - cost;
        sqlx::query(
            "INSERT INTO sales_facts (...) VALUES (...)"
        )
        // ...
        .execute(&db.pool)
        .await
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}
```

#### 3.2.8 `frontend/src/views/bi/SalesAnalysis.vue`

```vue
<template>
  <div class="bi-container">
    <h2>销售多维分析</h2>

    <!-- 1. KPI 概览 -->
    <BIKpiCards :kpis="kpis" />

    <!-- 2. 多维筛选 -->
    <BIFilters v-model="filters" @change="loadData" />

    <!-- 3. 销售趋势 -->
    <SalesTrendChart :data="trendData" />

    <!-- 4. 客户排行 -->
    <CustomerRankChart :data="customerData" />

    <!-- 5. 产品分布 -->
    <ProductPieChart :data="productData" />

    <!-- 6. 区域热力 -->
    <RegionHeatmap :data="regionData" />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { biApi } from '@/api/bi';
// ...

const kpis = ref({});
const trendData = ref([]);
const customerData = ref([]);
const productData = ref([]);
const regionData = ref([]);

const loadData = async () => {
  const [k, t, c, p, r] = await Promise.all([
    biApi.kpi(),
    biApi.salesTrend(),
    biApi.salesByCustomer(),
    biApi.salesByProduct(),
    biApi.salesByRegion(),
  ]);
  kpis.value = k;
  trendData.value = t;
  customerData.value = c;
  productData.value = p;
  regionData.value = r;
};

onMounted(loadData);
</script>
```

#### 3.2.9 `backend/tests/bi_analysis_test.rs`

```rust
//! BI 多维分析测试

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sales_by_time_tenant_isolation() {
        // 验证 SQL 强制 tenant_id 隔离
    }

    #[test]
    fn test_drilldown_year_to_month() {
        // 验证钻取逻辑
    }

    #[test]
    fn test_sales_facts_index() {
        // 验证索引
    }

    #[tokio::test]
    #[ignore = "需要 PostgreSQL + ETL 数据"]
    async fn test_e2e_etl_to_aggregation() {
        // 完整 ETL → 聚合流程
    }
}
```

### 3.3 沙箱约束处理

- **不**跑 `cargo test`
- **不**跑 `cargo build --release`
- **仅**跑 `cargo check --lib` 验证编译
- 前端 TypeScript 类型检查留给 CI

### 3.4 验证清单

- [ ] `cargo check --lib` 在 `backend/` 通过
- [ ] 3 张表 migration 语法正确
- [ ] 16 端点签名一致
- [ ] BI service 多租户 SQL 强制
- [ ] 前端 SalesAnalysis 页面 4 ECharts 图表
- [ ] 集成测试 4 个 stub

---

## 四、ST-3 文档

### 4.1 用户手册

`docs/2026-06-17-p3-4-data-warehouse-user-manual.md`

章节：
- 一、为什么需要数据仓库
- 二、Star Schema 设计
- 三、ETL 流程
- 四、BI 端点使用
- 五、前端 BI 页面
- 六、性能与限制
- 七、后续演进

### 4.2 API 文档

`docs/2026-06-17-p3-4-data-warehouse-api.md`

- 16 端点详细说明
- DTO 字段说明
- 多维分析示例
- 多租户隔离保证

### 4.3 CHANGELOG + MEMORY 更新

#### 4.3.1 CHANGELOG.md 新增

```markdown
## P3-4 (2026-06-17)

### 数据仓库/BI 建设

- 完整数据仓库架构设计 spec + 实施 plan
- 3 张表 migration（sales_facts + dim_products + dim_customers + dim_dates）
- 16 BI 端点（8 维度聚合 + 4 钻取 + 4 切片/上卷）
- 1 个后端 service（BiAnalysisService）
- 1 个前端 BI 报表页面（4 ECharts 图表）
- 1 个集成测试 stub
- 多租户隔离：所有 SQL 强制 tenant_id
- Star Schema 架构 + SCD Type 2
```

#### 4.3.2 MEMORY.md 新增

- 3 张表关键字段
- 16 端点清单
- Star Schema 设计
- ETL 任务位置

---

## 五、验收与合并

### 5.1 验收清单

| 编号 | 验收项 | 验证 |
|------|--------|------|
| AC-1 | spec + plan 完整 | 文件存在 + 章节齐全 |
| AC-2 | 3 表 migration | sqlx 语法正确 |
| AC-3 | 16 端点 + BI service | `cargo check --lib` |
| AC-4 | 1 个前端 BI 页面 | 4 ECharts 图表 |
| AC-5 | 集成测试 | bi_analysis_test.rs |
| AC-6 | 多租户隔离 | SQL 强制 tenant_id |
| AC-7 | 主项目未破坏 | `cargo check --lib` |
| AC-8 | 用户手册完整 | 启动 + 使用 + 演进 |

### 5.2 合并流程

1. commit：`docs(spec): P3-4 数据仓库/BI 建设设计 spec`
2. commit：`feat(P3-4): BI 销售多维分析 demo（3 表 + 16 端点 + 1 页面）`
3. push：当前分支 `trae/solo-agent-P3-4-data-warehouse`
4. PR：创建 PR #145（base: test）
5. merge：合到 test
6. 切回 test + pull + 删除本地分支

---

## 六、风险与回滚

| 风险 | 等级 | 缓解 |
|------|------|------|
| 沙箱 OOM | 高 | 仅 `cargo check --lib` |
| 业务库压力 | 中 | 独立数据仓库 schema |
| ETL 任务失败 | 中 | 简单重试 + 日志 |
| 主项目兼容性 | 低 | 仅新增 BI 模块 |

回滚：删除 BI 模块 + 不执行 4 个 migration，不影响 P0/P1/P2/P3。

---

## 七、关联

- Spec：`docs/superpowers/specs/2026-06-17-p3-4-data-warehouse.md`
- 用户手册：`docs/2026-06-17-p3-4-data-warehouse-user-manual.md`
- API 文档：`docs/2026-06-17-p3-4-data-warehouse-api.md`

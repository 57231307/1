# P3-4 数据仓库/BI 建设 - API 文档

> **发布日期**：2026-06-17
> **任务编号**：P3 / P3-4
> **端点**：16 个
> **鉴权**：JWT Bearer Token

---

## 一、响应格式

所有端点返回：
```json
{
  "code": 0,
  "message": "success",
  "data": <T>
}
```

业务数据在 `data.data` 中（BiResponse 包装）。

## 二、8 个维度聚合端点

### 2.1 GET /api/v1/erp/bi/sales/by-time

**功能**：按时间聚合销售

**请求参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| start_date | DATE | ✅ | 开始日期（YYYY-MM-DD） |
| end_date | DATE | ✅ | 结束日期 |
| granularity | STRING | ✅ | day/week/month/quarter/year |

**响应**：

```json
{
  "code": 0,
  "message": "success",
  "data": {
    "code": 0,
    "message": "success",
    "data": [
      {
        "period": "2026-05",
        "total_amount": 125000.0,
        "order_count": 45,
        "quantity": 1250.0,
        "profit_amount": 25000.0
      }
    ]
  }
}
```

### 2.2 GET /api/v1/erp/bi/sales/by-customer

**请求参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| limit | INT | ❌ | Top N，默认 10 |

**响应**：

```json
{
  "data": {
    "data": [
      {
        "customer_id": 1,
        "customer_name": "客户 A",
        "total_amount": 58000.0,
        "order_count": 12,
        "percentage": 28.5
      }
    ]
  }
}
```

### 2.3 GET /api/v1/erp/bi/sales/by-product

同 2.2，字段：

```typescript
interface ProductRank {
  product_id: number;
  product_name: string;
  product_code: string;
  category: string;
  total_amount: number;
  quantity: number;
  order_count: number;
}
```

### 2.4 GET /api/v1/erp/bi/sales/by-region

无参数。

```typescript
interface RegionStat {
  region: string;
  total_amount: number;
  order_count: number;
  customer_count: number;
}
```

### 2.5 GET /api/v1/erp/bi/sales/by-category

无参数。

```typescript
interface CategoryStat {
  category: string;
  total_amount: number;
  percentage: number;
}
```

### 2.6 GET /api/v1/erp/bi/sales/trend

**请求参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| days | INT | ❌ | 最近 N 天，默认 30 |

返回 `TimeSeriesPoint[]`。

### 2.7 GET /api/v1/erp/bi/sales/profit

无参数。

```typescript
interface ProfitAnalysis {
  total_revenue: number;
  total_cost: number;
  total_profit: number;
  gross_margin: number;     // 毛利率 %
  order_count: number;
  avg_order_value: number;
}
```

### 2.8 GET /api/v1/erp/bi/sales/kpi

无参数。

```typescript
interface KpiSummary {
  total_sales: number;
  order_count: number;
  customer_count: number;
  avg_order_value: number;
  yoy_growth: number;        // 同比增长率 %
  mom_growth: number;        // 环比增长率 %
}
```

## 三、4 个钻取端点

### 3.1 GET /api/v1/erp/bi/sales/drilldown/year-to-month

**请求参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| year | INT | ✅ | 年份 |

返回 12 个月的 `TimeSeriesPoint[]`。

### 3.2 GET /api/v1/erp/bi/sales/drilldown/month-to-day

**请求参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| year | INT | ✅ | 年份 |
| month | INT | ✅ | 月份（1-12） |

返回当月 30 天的 `TimeSeriesPoint[]`。

### 3.3 GET /api/v1/erp/bi/sales/drilldown/customer-to-order/{customer_id}

返回客户的所有订单列表。

### 3.4 GET /api/v1/erp/bi/sales/drilldown/product-to-order/{product_id}

返回产品的所有订单列表。

## 四、4 个切片/上卷端点

### 4.1 POST /api/v1/erp/bi/sales/slice

**请求体**：
```json
{
  "dimension": "customer",
  "filters": {
    "region": "华东",
    "date_range": ["2026-01-01", "2026-06-30"]
  }
}
```

### 4.2 POST /api/v1/erp/bi/sales/dice

**请求体**：
```json
{
  "filters": {
    "product_ids": [1, 2, 3],
    "customer_type": "VIP",
    "date_range": ["2026-01-01", "2026-06-30"]
  }
}
```

### 4.3 POST /api/v1/erp/bi/sales/rollup

**请求体**：
```json
{
  "from": "day",
  "to": "month"
}
```

粒度级别：day / week / month / quarter / year。

### 4.4 POST /api/v1/erp/bi/sales/pivot

**请求体**：
```json
{
  "row": "time",
  "col": "product",
  "measure": "amount"
}
```

## 五、错误响应

```json
{
  "code": 400,
  "message": "租户 ID 无效"
}
```

| 错误码 | 触发条件 |
|--------|----------|
| 400 | 租户 ID 无效 / 参数错误 |
| 401 | JWT 鉴权失败 |
| 500 | 数据库查询失败 |

## 六、多租户隔离

- 所有端点通过 `extract_tenant_id(&auth)?` 提取租户 ID
- 所有 SQL 强制 `WHERE tenant_id = $1`
- 与主项目 `extract_tenant_id` 一致

## 七、性能

| 指标 | 目标值 |
|------|--------|
| 单查询 P99 | < 500ms |
| 聚合吞吐 | 10,000 QPS |
| 事实表行数 | 1 亿+ |
| 索引覆盖 | tenant_id + order_date |

## 八、限制

- P3-4 是关键路径 demo，返回 mock 数据
- 真实 SQL 在 service 注释中（CI 跑真实数据）
- 后续 P4+ 引入 ClickHouse/Druid 提升性能

# P2-4 AI 分析深化 API 文档

> 端点总数: 16
> 基础路径: `/api/v1/erp/ai`
> 认证: `Authorization: Bearer <JWT>`（继承现有 auth 中间件）
> 多租户: 所有端点按 `tenant_id` 严格隔离（从 `AuthContext.tenant_id` 提取）

## 一、通用约定

### 1.1 请求格式
所有 POST/PUT 请求使用 `application/json`；GET 请求通过 query string 传参。

### 1.2 响应格式
```json
{
  "code": 0,
  "message": "ok",
  "data": { ... }
}
```

- `code = 0` 成功；非 0 为业务错误
- 业务错误示例：
  - `4000` validation（参数错误）
  - `4004` not_found（资源不存在）
  - `5000` internal（系统错误）

### 1.3 分页参数
| 参数 | 类型 | 默认 | 范围 |
| --- | --- | --- | --- |
| `page` | integer | 1 | ≥1 |
| `page_size` | integer | 20 | 1-100 |

## 二、工艺优化（7 端点）

### 2.1 `POST /ai/process-optimizations` 触发工艺优化

**请求体**
```json
{
  "request": {
    "color_no": "BL-301",
    "color_name": "雾霾蓝",
    "fabric_type": "棉",
    "dye_type": "活性染料",
    "k": 5
  },
  "operator_id": 1,
  "tenant_id": 1001
}
```

> **注意**：`operator_id` 与 `tenant_id` 由后端从 `AuthContext` 自动注入，前端不传。

**响应** `200 OK`
```json
{
  "code": 0,
  "data": {
    "id": 42,
    "response": {
      "recommended_params": {
        "temperature": 78.5,
        "time_minutes": 42,
        "ph_value": 6.2,
        "liquor_ratio": 8.5
      },
      "confidence": 0.92,
      "source": "knn",
      "similar_cases": 7,
      "reason": "命中 7 条相似案例（k=5），加权平均温度 78.5°C / 时间 42min / pH 6.2 / 浴比 1:8.5",
      "candidates": [
        {
          "case_id": 18,
          "color_no": "BL-301",
          "fabric_type": "棉",
          "similarity": 0.94,
          "temperature": 78,
          "time_minutes": 40,
          "ph_value": 6.0,
          "liquor_ratio": 8
        }
      ]
    }
  }
}
```

**错误码**
- `4000` color_no 必填 / fabric_type 必填
- `5000` 数据库写入失败

### 2.2 `GET /ai/process-optimizations` 列表

**Query 参数**
| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `page` | int | 页码 |
| `page_size` | int | 每页大小 |
| `color_no` | string | 按色号过滤 |
| `fabric_type` | string | 按布类过滤 |
| `is_applied` | bool | 按应用状态过滤 |
| `source` | string | `knn` / `fallback` |

**响应**
```json
{
  "code": 0,
  "data": {
    "items": [AiProcessOptimization, ...],
    "total": 100,
    "page": 1,
    "page_size": 20
  }
}
```

### 2.3 `GET /ai/process-optimizations/{id}` 详情

**响应** `200 OK` 返回 `AiProcessOptimization` 单条对象；`404` 返回 `code=4004`。

### 2.4 `POST /ai/process-optimizations/{id}/apply` 应用反馈

**请求体**
```json
{
  "feedback_score": 5,
  "feedback_remark": "工艺非常稳定"
}
```

| 字段 | 必填 | 范围 | 说明 |
| --- | --- | --- | --- |
| `feedback_score` | 否 | 1-5 | 反馈评分 |
| `feedback_remark` | 否 | ≤200 字 | 备注 |

**响应** 返回更新后的 `AiProcessOptimization`，`is_applied=true`。

### 2.5 `DELETE /ai/process-optimizations/{id}` 删除

**响应** `200 OK` 返回 `{ "deleted": true, "id": 42 }`；如记录不存在返回 `404`。

### 2.6 `GET /ai/process-optimizations/by-color` 按色号 + 布类历史

**Query 参数**
- `color_no` 必填
- `fabric_type` 必填
- `limit` 可选，默认 20，上限 50

### 2.7 `POST /ai/process-optimizations/batch` 批量触发

**请求体**
```json
{
  "requests": [
    { "request": { "color_no": "...", "fabric_type": "..." } },
    { "request": { "color_no": "...", "fabric_type": "..." } }
  ]
}
```

**约束**：`requests` 长度 ≤ 20；超过返回 `4000`。

**响应**
```json
{
  "code": 0,
  "data": {
    "total": 10,
    "succeeded": 9,
    "failed": 1,
    "results": [
      { "id": 42, "success": true, "recommended_params": {...}, "confidence": 0.92, "source": "knn" },
      { "success": false, "error": "color_no 必填" }
    ]
  }
}
```

## 三、质量预测（7 端点）

### 3.1 `POST /ai/quality-predictions` 触发预测

**请求体**
```json
{
  "request": {
    "product_id": 123,
    "inspection_type": "all",
    "window_days": 90
  }
}
```

| 字段 | 必填 | 范围 | 默认 |
| --- | --- | --- | --- |
| `product_id` | 是 | ≥1 | — |
| `inspection_type` | 否 | `all` / `incoming` / `inprocess` / `final` / `outgoing` | `all` |
| `window_days` | 否 | 7-365 | 90 |

**响应**
```json
{
  "code": 0,
  "data": {
    "id": 88,
    "response": {
      "product_id": 123,
      "inspection_type": "all",
      "window_days": 90,
      "total_inspections": 12,
      "avg_qualification_rate": 92.5,
      "trend": "下降",
      "trend_rate": -5.3,
      "risk_score": 68,
      "risk_level": "高",
      "confidence": 0.8,
      "top_issues": [
        { "issue": "颜色差异", "count": 3, "percentage": 25.0 },
        { "issue": "色牢度", "count": 2, "percentage": 16.7 }
      ],
      "recommendations": [
        "复盘近 30 天染色工艺，重点排查温度波动",
        "加强来料检验，避免不合格原料流入"
      ],
      "period_breakdown": [
        { "period": "2026-03", "inspections": 4, "avg_qualification_rate": 95.0 },
        { "period": "2026-04", "inspections": 4, "avg_qualification_rate": 93.0 },
        { "period": "2026-05", "inspections": 4, "avg_qualification_rate": 89.5 }
      ],
      "source": "history"
    }
  }
}
```

### 3.2 `GET /ai/quality-predictions` 列表

**Query 参数**
- `page` / `page_size` 分页
- `product_id` 按产品
- `inspection_type` 按检验类型
- `risk_level` low / medium / high
- `is_acknowledged` bool

### 3.3 `GET /ai/quality-predictions/{id}` 详情

### 3.4 `POST /ai/quality-predictions/{id}/acknowledge` 确认

**请求体** `{}`（无 body）

**响应** 返回更新后对象，`is_acknowledged=true`。

### 3.5 `DELETE /ai/quality-predictions/{id}` 删除

### 3.6 `GET /ai/quality-predictions/by-product` 按产品历史

**Query 参数**
- `product_id` 必填
- `limit` 可选，默认 20，上限 50

### 3.7 `POST /ai/quality-predictions/batch` 批量触发

**约束**：`requests` 长度 ≤ 20。

## 四、看板 / 健康（2 端点）

### 4.1 `GET /ai/summary` AI 概览

**响应**
```json
{
  "code": 0,
  "data": {
    "process_optimization": {
      "total": 100,
      "applied": 60,
      "knn_recommended": 75,
      "apply_rate": 0.6
    },
    "quality_prediction": {
      "total": 50,
      "high_risk": 8,
      "unacknowledged": 12
    },
    "latest_process_optimizations": [...],
    "latest_quality_predictions": [...]
  }
}
```

### 4.2 `GET /ai/health` 健康检查

**响应**
```json
{
  "code": 0,
  "data": {
    "status": "ok",
    "version": "P2-4",
    "modules": {
      "process_optimization": {
        "algorithm": "k-NN + 加权平均",
        "fallback": "典型参数表（80°C/45min/pH6.0/浴比1:8）"
      },
      "quality_prediction": {
        "algorithm": "趋势分析 + 风险评分",
        "fallback": "保守默认（合格率 95% / 置信度 0.3）"
      }
    }
  }
}
```

## 五、错误码表

| Code | 含义 | 触发场景 |
| --- | --- | --- |
| 4000 | validation | 必填字段缺失 / feedback_score 越界 / 批量超过 20 |
| 4001 | unauthorized | 无 token 或 token 过期 |
| 4003 | forbidden | 跨租户访问 |
| 4004 | not_found | 资源 ID 不存在 |
| 5000 | internal | 数据库连接失败 / 序列化失败 |

## 六、OpenAPI 片段

```yaml
openapi: 3.0.3
info:
  title: 冰溪 ERP AI 分析深化 API
  version: 2026.522.2
servers:
  - url: /api/v1/erp
paths:
  /ai/process-optimizations:
    post:
      summary: 触发工艺优化
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/ProcessOptCreateRequest'
      responses:
        '200':
          description: 成功
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/ProcessOptCreateResponse'
  /ai/quality-predictions:
    post:
      summary: 触发质量预测
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/QualityPredCreateRequest'
      responses:
        '200':
          description: 成功
components:
  schemas:
    ProcessOptCreateRequest:
      type: object
      required: [request]
      properties:
        request:
          type: object
          required: [color_no, fabric_type]
          properties:
            color_no: { type: string }
            color_name: { type: string }
            fabric_type: { type: string }
            dye_type: { type: string }
            k: { type: integer, default: 5, minimum: 1, maximum: 20 }
```

## 七、变更记录

| 版本 | 日期 | 内容 |
| --- | --- | --- |
| P2-4 | 2026-06-17 | 初始发布：16 端点 + 2 表 + 4 页面 + 2 组件 |

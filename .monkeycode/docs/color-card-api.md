# 色卡仓储管理 API 文档

> P0-4 色卡仓储管理模块完整 API 参考
> 创建时间: 2026-06-17
> 关联 spec: `docs/superpowers/specs/2026-06-16-color-card-design.md`

---

## 1. 通用说明

### 1.1 基础信息
- **基础路径**：`/api/v1/erp/color-cards`
- **认证方式**：JWT Bearer Token（与系统其他端点一致）
- **多租户隔离**：所有端点强制通过 `extract_tenant_id` 获取当前租户，跨租户访问自动拒绝

### 1.2 通用响应格式
```json
{
  "code": 0,
  "message": "success",
  "data": { ... }
}
```

### 1.3 错误码
| HTTP | 业务错误 | 说明 |
|------|----------|------|
| 400 | validation | 参数校验失败 |
| 404 | not_found | 资源不存在 |
| 409 | conflict | 业务冲突（如编号重复、状态不允许） |
| 500 | database | 数据库错误 |

---

## 2. 色卡 CRUD 端点

### 2.1 列出色卡

**请求**：`GET /api/v1/erp/color-cards`

**查询参数**：
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| page | int | 否 | 页码，默认 1 |
| page_size | int | 否 | 每页条数，默认 20，最大 200 |
| card_type | string | 否 | 过滤类型：PANTONE / CNCS / CUSTOM |
| season | string | 否 | 过滤季节：2024SS / 2024AW 等 |
| status | string | 否 | 过滤状态：active / archived / lost |
| keyword | string | 否 | 按色卡名称模糊搜索 |

**响应示例**：
```json
{
  "code": 0,
  "data": {
    "items": [
      {
        "id": 1,
        "card_no": "PANTONE-TPX-2024-SS",
        "card_name": "2024 春夏 PANTONE 色卡",
        "card_type": "PANTONE",
        "season": "2024SS",
        "brand": "自有品牌",
        "total_colors": 1865,
        "status": "active",
        "cover_image_url": "https://example.com/cover.jpg",
        "created_at": "2026-06-17T08:00:00Z"
      }
    ],
    "total": 12,
    "page": 1,
    "page_size": 20
  }
}
```

### 2.2 创建色卡

**请求**：`POST /api/v1/erp/color-cards`

**请求体**：
```json
{
  "card_no": "PANTONE-TPX-2024-SS",
  "card_name": "2024 春夏 PANTONE 色卡",
  "card_type": "PANTONE",
  "season": "2024SS",
  "brand": "自有品牌",
  "description": "2024 春夏国际通用色卡",
  "cover_image_url": "https://example.com/cover.jpg"
}
```

**响应**：200 + ColorCardListItem

### 2.3 色卡详情

**请求**：`GET /api/v1/erp/color-cards/:id`

**响应**：
```json
{
  "code": 0,
  "data": {
    "id": 1,
    "card_no": "PANTONE-TPX-2024-SS",
    "card_name": "2024 春夏 PANTONE 色卡",
    "card_type": "PANTONE",
    "season": "2024SS",
    "brand": "自有品牌",
    "total_colors": 1865,
    "status": "active",
    "description": "2024 春夏国际通用色卡",
    "cover_image_url": "https://example.com/cover.jpg",
    "items": [
      {
        "id": 1,
        "color_code": "18-1664 TPX",
        "color_name": "番茄红",
        "rgb_r": 220,
        "rgb_g": 50,
        "rgb_b": 50,
        "cmyk_c": 0,
        "cmyk_m": 77.27,
        "cmyk_y": 77.27,
        "cmyk_k": 13.73,
        "lab_l": 47.5,
        "lab_a": 65.2,
        "lab_b": 38.1,
        "pantone_code": "18-1664 TPX",
        "hex_value": "#DC3232",
        "sequence": 1
      }
    ],
    "created_at": "2026-06-17T08:00:00Z",
    "updated_at": "2026-06-17T08:00:00Z"
  }
}
```

### 2.4 更新色卡

**请求**：`PUT /api/v1/erp/color-cards/:id`

**请求体**（所有字段可选）：
```json
{
  "card_name": "新名称",
  "description": "新描述",
  "cover_image_url": "新 URL"
}
```

**注意**：仅 `active` 状态可更新。

### 2.5 归档色卡

**请求**：`DELETE /api/v1/erp/color-cards/:id`

**请求体**：
```json
{ "reason": "季节已过" }
```

**注意**：归档为软删除，状态变为 `archived`，不可再编辑。

---

## 3. 色号 CRUD 端点

### 3.1 列出色号

**请求**：`GET /api/v1/erp/color-cards/:id/items?page=1&page_size=100`

**响应**：`PagedResponse<ColorItemInfo>`

### 3.2 添加色号

**请求**：`POST /api/v1/erp/color-cards/:id/items`

**请求体**：
```json
{
  "color_code": "18-1664 TPX",
  "color_name": "番茄红",
  "rgb_r": 220,
  "rgb_g": 50,
  "rgb_b": 50,
  "cmyk_c": 0,
  "cmyk_m": 77.27,
  "cmyk_y": 77.27,
  "cmyk_k": 13.73,
  "lab_l": 47.5,
  "lab_a": 65.2,
  "lab_b": 38.1,
  "pantone_code": "18-1664 TPX",
  "cncs_code": "S 1050-Y90R",
  "hex_value": "#DC3232",
  "sequence": 1
}
```

**注意**：`cmyk_*` 和 `lab_*` 字段可选，未传则自动从 RGB 计算。

### 3.3 批量导入色号

**请求**：`POST /api/v1/erp/color-cards/:id/items/batch`

**请求体**：
```json
{
  "items": [
    {
      "color_code": "18-1664 TPX",
      "color_name": "番茄红",
      "rgb_r": 220,
      "rgb_g": 50,
      "rgb_b": 50,
      "hex_value": "#DC3232"
    },
    {
      "color_code": "17-1463 TPX",
      "color_name": "柑橘橙",
      "rgb_r": 250,
      "rgb_g": 130,
      "rgb_b": 50,
      "hex_value": "#FA8232"
    }
  ]
}
```

**响应**：
```json
{
  "code": 0,
  "data": {
    "success_count": 1000,
    "failed_count": 0,
    "errors": [],
    "total_colors": 1865
  }
}
```

### 3.4 更新色号

**请求**：`PUT /api/v1/erp/color-cards/:id/items/:item_id`

请求体同「添加色号」。

### 3.5 删除色号

**请求**：`DELETE /api/v1/erp/color-cards/:id/items/:item_id`

**响应**：`{ "code": 0, "data": null }`

---

## 4. 借出 / 归还 / 遗失 端点

### 4.1 借出色卡

**请求**：`POST /api/v1/erp/color-cards/borrow`

**请求体**：
```json
{
  "color_card_id": 1,
  "customer_id": 100,
  "borrowed_by": 5,
  "expected_return_at": "2026-07-01T18:00:00Z",
  "purpose": "客户选色",
  "notes": "已电话沟通"
}
```

**业务规则**：
- `expected_return_at` 不能晚于借出时间 + 30 天
- `borrowed_by` 默认当前用户

**响应**：200 + BorrowRecordInfo（status=borrowed）

### 4.2 归还色卡

**请求**：`POST /api/v1/erp/color-cards/return/:record_id`

**请求体**：
```json
{
  "actual_return_at": "2026-06-25T14:00:00Z",
  "notes": "完好归还"
}
```

**响应**：200 + BorrowRecordInfo（status=returned）

### 4.3 登记遗失

**请求**：`POST /api/v1/erp/color-cards/lost/:record_id`

**请求体**：
```json
{
  "compensation_amount": 500.0,
  "notes": "客户在展会遗失"
}
```

**业务规则**：
- `compensation_amount` 必须 > 0
- 状态联动：色卡状态变为 `lost`

**响应**：200 + BorrowRecordInfo（status=lost）

### 4.4 标记损坏

**请求**：`POST /api/v1/erp/color-cards/damaged/:record_id`

**请求体**：
```json
{
  "compensation_amount": 200.0,
  "notes": "色片脱落"
}
```

### 4.5 借出历史

**请求**：`GET /api/v1/erp/color-cards/borrow-records`

**查询参数**：
| 参数 | 类型 | 说明 |
|------|------|------|
| color_card_id | int | 按色卡过滤 |
| customer_id | int | 按客户过滤 |
| status | string | borrowed / returned / lost / damaged |
| from_date | string | 起始时间（ISO 8601）|
| to_date | string | 结束时间（ISO 8601）|
| page | int | 页码 |
| page_size | int | 每页条数 |

---

## 5. 扫码 / 导出 端点

### 5.1 扫码查询

**请求**：`GET /api/v1/erp/color-cards/scan/:code`

**响应**：
```json
{
  "code": 0,
  "data": {
    "color_item": {
      "id": 1,
      "color_code": "18-1664 TPX",
      "color_name": "番茄红",
      "rgb_r": 220,
      "rgb_g": 50,
      "rgb_b": 50,
      "lab_l": 47.5,
      "lab_a": 65.2,
      "lab_b": 38.1,
      "hex_value": "#DC3232"
    },
    "color_card_no": "PANTONE-TPX-2024-SS",
    "color_card_name": "2024 春夏 PANTONE 色卡",
    "recipe_summary": {
      "id": 5,
      "recipe_name": "番茄红 染色配方",
      "fabric_type": "棉",
      "color_no": "18-1664",
      "temperature": 80.0,
      "time_minutes": 45
    },
    "price_summary": {
      "id": 3,
      "base_price": 35.5,
      "currency": "CNY",
      "effective_from": "2026-01-01",
      "customer_level": "VIP"
    }
  }
}
```

### 5.2 导出色卡 CSV

**请求**：`GET /api/v1/erp/color-cards/export/:id`

**响应**：`text/csv` 文件，文件名 `color-card-{card_no}.csv`

CSV 字段：`color_code,color_name,rgb_r,rgb_g,rgb_b,hex_value,cmyk_c,cmyk_m,cmyk_y,cmyk_k,lab_l,lab_a,lab_b,pantone_code,cncs_code,custom_code`

---

## 6. curl 示例

### 6.1 创建色卡
```bash
curl -X POST http://localhost:8080/api/v1/erp/color-cards \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "card_no": "TEST-2026-SS",
    "card_name": "测试色卡",
    "card_type": "CUSTOM",
    "season": "2026SS"
  }'
```

### 6.2 批量导入色号
```bash
curl -X POST http://localhost:8080/api/v1/erp/color-cards/1/items/batch \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "items": [
      {
        "color_code": "TEST-001",
        "color_name": "测试色 1",
        "rgb_r": 255, "rgb_g": 0, "rgb_b": 0,
        "hex_value": "#FF0000"
      }
    ]
  }'
```

### 6.3 借出色卡
```bash
curl -X POST http://localhost:8080/api/v1/erp/color-cards/borrow \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "color_card_id": 1,
    "customer_id": 100,
    "purpose": "客户选色"
  }'
```

### 6.4 扫码查询
```bash
curl -X GET http://localhost:8080/api/v1/erp/color-cards/scan/18-1664%20TPX \
  -H "Authorization: Bearer $TOKEN"
```

---

## 7. 行业规则参考

| 规则 | 标准 | 说明 |
|------|------|------|
| RGB | 0-255 | 标准 RGB 色彩空间 |
| CMYK | 0-100% | 印刷 CMYK 色彩空间 |
| CIELab L | 0-100 | CIE 1976 亮度 |
| CIELab a/b | -128 ~ 127 | CIE 1976 色度坐标 |
| ΔE 色差 | ≤ 3.0 | CIE76 公式，行业标准 |
| PANTONE | - | 国际通用色卡体系 |
| CNCS | GB/T 15608-2006 | 中国颜色体系 |

---

> 文档版本：v1.0 | 2026-06-17 | P0-4 色卡仓储管理 API

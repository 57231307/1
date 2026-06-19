# P0-4 色卡仓储管理 - 19 个测试场景

## 一、色卡 CRUD 场景（5 个）

### 场景 1.1：创建色卡
- **操作**：POST /api/v1/erp/color-cards
- **请求体**：`{card_no, card_name, card_type, season, brand, description}`
- **预期**：返回 200 + 色卡 ID，状态为 active

### 场景 1.2：列出色卡（分页）
- **操作**：GET /api/v1/erp/color-cards?page=1&page_size=20
- **预期**：返回分页结果，包含 total / items / page / page_size

### 场景 1.3：查看色卡详情
- **操作**：GET /api/v1/erp/color-cards/:id
- **预期**：返回色卡基本信息 + items 列表

### 场景 1.4：更新色卡
- **操作**：PUT /api/v1/erp/color-cards/:id
- **预期**：仅 active 状态可更新，返回更新后的色卡

### 场景 1.5：归档色卡
- **操作**：DELETE /api/v1/erp/color-cards/:id
- **预期**：色卡状态变为 archived

## 二、色号 CRUD 场景（4 个）

### 场景 2.1：添加色号
- **操作**：POST /api/v1/erp/color-cards/:id/items
- **请求体**：`{color_code, color_name, rgb_r, rgb_g, rgb_b, hex_value}`
- **预期**：自动计算 CMYK + Lab，色卡 total_colors +1

### 场景 2.2：批量导入色号
- **操作**：POST /api/v1/erp/color-cards/:id/items/batch
- **请求体**：`{items: [{...}, {...}]}`
- **预期**：1000 条 < 5 秒，返回 success_count / failed_count

### 场景 2.3：列出色号
- **操作**：GET /api/v1/erp/color-cards/:id/items
- **预期**：按 sequence 排序返回色号列表

### 场景 2.4：删除色号
- **操作**：DELETE /api/v1/erp/color-cards/:id/items/:item_id
- **预期**：色号删除，色卡 total_colors -1

## 三、借出 / 归还 / 遗失 场景（5 个）

### 场景 3.1：借出色卡
- **操作**：POST /api/v1/erp/color-cards/borrow
- **请求体**：`{color_card_id, customer_id, expected_return_at, purpose}`
- **预期**：创建借出记录（status=borrowed）

### 场景 3.2：归还色卡
- **操作**：POST /api/v1/erp/color-cards/return/:record_id
- **请求体**：`{actual_return_at, notes}`
- **预期**：借出记录 status → returned

### 场景 3.3：登记遗失（含赔付）
- **操作**：POST /api/v1/erp/color-cards/lost/:record_id
- **请求体**：`{compensation_amount: 500, notes: "客户遗失"}`
- **预期**：借出记录 status → lost + 赔付金额 + 色卡状态 → lost

### 场景 3.4：标记损坏
- **操作**：POST /api/v1/erp/color-cards/damaged/:record_id
- **预期**：借出记录 status → damaged

### 场景 3.5：借出历史查询
- **操作**：GET /api/v1/erp/color-cards/borrow-records?customer_id=1&status=returned
- **预期**：按条件过滤的借出历史

## 四、扫码 / 导出 场景（3 个）

### 场景 4.1：扫码查询色号
- **操作**：GET /api/v1/erp/color-cards/scan/18-1664
- **预期**：返回色号完整信息（RGB/CMYK/Lab/配方/价格）

### 场景 4.2：导出色卡 CSV
- **操作**：GET /api/v1/erp/color-cards/export/:id
- **预期**：下载 CSV 文件，包含所有色号

### 场景 4.3：色彩空间转换
- **操作**：添加色号时自动从 RGB 计算 CMYK 和 Lab
- **预期**：CMYK 各通道 0-100，Lab L 0-100 a/b -128~127

## 五、边界与异常场景（2 个）

### 场景 5.1：多租户隔离
- **操作**：租户 A 创建色卡，租户 B 尝试访问
- **预期**：租户 B 返回 404 色卡不存在

### 场景 5.2：借出时间校验
- **操作**：预计归还时间 > 借出时间 + 30 天
- **预期**：返回 400「预计归还时间不能超过借出时间 + 30 天」

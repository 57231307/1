# 新增功能文档 - BOM/MRP/产能/缺料模块

## 1. BOM 管理模块

### 1.1 功能概述
BOM（物料清单）管理模块提供产品配方的完整管理能力，支持多层级 BOM、版本控制和默认 BOM 设置。

### 1.2 后端 API

| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/v1/erp/boms` | POST | 创建 BOM |
| `/api/v1/erp/boms` | GET | BOM 列表（支持分页、筛选） |
| `/api/v1/erp/boms/:id` | GET | BOM 详情 |
| `/api/v1/erp/boms/:id` | PUT | 更新 BOM |
| `/api/v1/erp/boms/:id` | DELETE | 删除 BOM（软删除） |
| `/api/v1/erp/boms/:id/copy` | POST | 复制 BOM |
| `/api/v1/erp/boms/:id/set-default` | PUT | 设为默认 BOM |
| `/api/v1/erp/boms/tree` | GET | BOM 树形结构 |

### 1.3 核心特性
- **版本管理**: 每个产品支持多个 BOM 版本，版本号按 product_id 自增
- **默认 BOM**: 每个产品只能有一个默认 BOM，设置时自动取消其他默认
- **软删除**: 删除操作将状态设为 INACTIVE，不物理删除数据
- **BOM 明细**: 支持物料、数量、单位、损耗率配置

### 1.4 前端页面
- 路径: `/bom`
- 功能: BOM 列表、新增/编辑表单、明细查看、复制、设为默认

---

## 2. MRP 物料需求计算

### 2.1 功能概述
MRP（Material Requirements Planning）引擎根据销售订单或预测需求，结合 BOM 展开和库存状况，自动计算物料需求并生成采购/生产建议。

### 2.2 后端 API

| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/v1/erp/mrp/calculate` | POST | 触发 MRP 计算 |
| `/api/v1/erp/mrp/results` | GET | 查询计算结果 |
| `/api/v1/erp/mrp/requirements` | GET | 物料需求清单 |
| `/api/v1/erp/mrp/convert-orders` | POST | 转为采购/生产订单 |

### 2.3 计算逻辑
1. **毛需求计算**: 根据产品需求量 × BOM 用量展开
2. **净需求计算**: 毛需求 - 可用库存（在手量 + 在途量 - 安全库存）
3. **计划订单生成**: 按净需求生成建议采购/生产数量

### 2.4 前端页面
- 路径: `/mrp` (计算) / `/mrp/history` (历史记录)
- 功能: 产品多选、参数配置、结果展示、转订单

---

## 3. 产能分析

### 3.1 功能概述
产能分析模块帮助识别生产瓶颈，评估工作中心负荷，优化生产排程。

### 3.2 后端 API

| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/v1/erp/capacity/overview` | GET | 产能概览统计 |
| `/api/v1/erp/capacity/work-centers` | GET | 工作中心列表 |
| `/api/v1/erp/capacity/load-analysis` | GET | 负荷分析 |

### 3.3 核心指标
- **负荷率**: 已排产工时 / 总产能工时
- **状态分类**: 正常(<80%)、繁忙(80-100%)、超负荷(>100%)
- **瓶颈识别**: 自动标识超负荷工作中心

### 3.4 前端页面
- 路径: `/capacity`
- 功能: ECharts 负荷趋势图、工作中心表格、瓶颈识别

---

## 4. 缺料预警

### 4.1 功能概述
实时监控库存与生产需求对比，自动检测缺料风险并按严重度分级预警。

### 4.2 后端 API

| 端点 | 方法 | 描述 |
|------|------|------|
| `/api/v1/erp/material-shortage/alerts` | GET | 缺料预警列表 |
| `/api/v1/erp/material-shortage/check` | POST | 手动触发检查 |
| `/api/v1/erp/material-shortage/summary` | GET | 缺料汇总 |

### 4.3 严重度分级
- **严重**: 库存 < 需求 × 50%
- **高度**: 库存 < 需求 × 75%
- **中等**: 库存 < 需求
- **低度**: 库存 >= 需求但接近安全库存

### 4.4 前端页面
- 路径: `/material-shortage`
- 功能: 预警列表、严重度统计、手动触发检查

---

## 5. 编译和构建状态

### 5.1 后端
```bash
cd backend && cargo check
# 0 errors, warnings only
```

### 5.2 前端
```bash
cd frontend && npm run build
# built successfully
```

---

## 6. 数据库迁移

已创建迁移脚本 `021_add_currency_fields.sql`:
- sales_orders: currency_code, exchange_rate
- purchase_orders: currency_code, exchange_rate
- ar_invoices: currency_code, exchange_rate, base_amount
- ap_invoices: currency_code, exchange_rate, base_amount

---

## 7. PR 链接

https://github.com/57231307/1/pull/new/260520-feat-bom-mrp-capacity-material-shortage

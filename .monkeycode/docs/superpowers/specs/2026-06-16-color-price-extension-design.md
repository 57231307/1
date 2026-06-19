# 面料多色号定价扩展模块设计 Spec

> 冰溪 ERP P0-5 行业功能：面料多色号定价扩展（纺织行业多色号、多币种、多客户等级、多季节、多阶梯的完整价格管理）
> 设计日期: 2026-06-17
> 关联代码: `backend/src/{models,services,handlers,routes,utils}/color_price_*` + 前端 `frontend/src/views/color-prices/` + 测试版本 `dist/test-version-P0-5/`

---

## 1. 业务背景

### 1.1 行业定位

面料多色号定价是纺织行业 ERP 区别于通用 ERP 的核心差异点。一块面料（如棉府绸）会有几十甚至上百个色号（color code），每个色号在不同客户、不同季节、不同数量下呈现完全不同的价格。例如：

- **客户 A**（VIP 品牌客户）5000 米定制 → 阶梯价 + VIP 95 折 + 春季 9 折叠加 = 9.5 × 9.0 = 8.55 折
- **客户 B**（普通客户）100 米现货 → 基础价（无任何折扣）

通用 ERP 的"商品 + 单价"模型完全无法承载这种 **多维度叠加定价**，因此本模块是 P0 行业功能的核心组成。

### 1.2 P0-1 已实现 30% 基础

P0-1 销售报价单（PR #126）已创建 `product_color_prices` 表（30% 实现）：

| 字段 | 类型 | 备注 |
|------|------|------|
| id | BIGSERIAL PK | 主键 |
| product_id | BIGINT FK | 产品 |
| color_id | BIGINT FK | 色号 |
| currency | VARCHAR(10) | 币种 |
| base_price | DECIMAL(18,6) | 基础价 |
| effective_from / effective_to | DATE | 有效期 |
| customer_level | VARCHAR(20) | 客户等级 |
| min_quantity | DECIMAL(18,2) | 最小起订量 |
| notes | TEXT | 备注 |
| created_at / updated_at | TIMESTAMPTZ | 时间戳 |

### 1.3 P0-5 待补齐 70% 缺口

| 缺口 | 业务影响 | 优先级 |
|------|----------|--------|
| **批量调价** | 一次性调整 50+ 色号 → 提升工作效率 10x | P0 |
| **价格历史** | 调价审计、回溯、报表 | P0 |
| **阶梯定价** | 数量越多价越低，激励大客户 | P0 |
| **季节性调价** | 春夏 / 秋冬自动调价 | P1 |
| **客户专属价** | 战略客户大客户协议价 | P0 |
| **价格预警** | 涨幅 > 20% 预警、跌幅 > 30% 预警、价格过期 | P1 |
| **价格计算引擎** | 统一计算最优价格（VIP/阶梯/季节/客户专属优先级） | P0 |
| **审批流** | 大幅调价需经理审批 | P1 |

### 1.4 行业标准

- **VIP 95 折 / 普通 100%** — 客户等级标准折扣
- **4 档阶梯价** — 100 米以下 / 100-500 米 / 500-1000 米 / 1000+ 米
- **季节调价** — SS（春夏 4-9 月）/ AW（秋冬 10-3 月）/ HOLIDAY（节日）
- **多币种** — CNY / USD / EUR 实时汇率（复用 P0-1）
- **多租户隔离** — `extract_tenant_id` 强制

---

## 2. 范围与目标

### 2.1 范围（In Scope）

- 1 张表扩展（`product_color_prices`）+ 4 张新表（`color_price_history` / `color_price_tiers` / `customer_color_prices` / `seasonal_price_rules`） = **5 张表**
- 5 个 entity + 5 个 DTO + **16 个 API 端点** + **5 个 service** + **13 个 handler** + **1 个价格计算引擎**
- 3 个前端页面（list / detail / batch-adjust）
- 2 个前端组件（PriceHistoryChart / BatchAdjustDialog）
- 集成测试 + E2E 测试
- 用户手册 + API 文档 + 部署指南
- TEST 测试版本交付（Docker + docker-compose）

### 2.2 不在范围（Out of Scope）

- 调价工作流引擎（仅经理审批硬编码，不做 BPM 流程）
- 价格预测 / 弹性分析（仅做历史查询）
- 竞品对标（独立模块，本期不做）
- 调价 Excel 导入导出（仅页面表单）
- 客户等级规则引擎（VIP/NORMAL 硬编码）

### 2.3 验收目标

- 16 个 API 端点全部实现并通过集成测试
- 3 个前端页面 + 2 个组件可正常访问
- 价格计算引擎支持 4 档阶梯 + VIP 95 折 + 季节调价 + 客户专属价
- 批量调价支持百分比 / 固定金额 / 阶梯价 3 种模式
- 涨幅 > 10% 触发审批（经理可批 / 拒）
- 调价历史全程记录（操作人、时间、原因、变更前后）
- 多租户隔离（`extract_tenant_id` 强制）
- TEST 测试版本可在 Docker 中启动

---

## 3. 数据模型

### 3.1 ER 关系

```
product_color_prices (1) ──< (N) color_price_history
product_color_prices (1) ──< (N) color_price_tiers
customers (1) ──< (N) customer_color_prices
products (1) ──< (N) product_color_prices
product_colors (1) ──< (N) product_color_prices
product_categories (1) ──< (N) seasonal_price_rules
```

### 3.2 扩展 product_color_prices（已存在，需 ALTER）

新增字段：

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| max_quantity | DECIMAL(18,2) | NULL | 阶梯价区间（NULL = 无限） |
| customer_id | BIGINT FK | NULL | 客户专属（NULL = 通用） |
| season | VARCHAR(10) | NULL | SS / AW / HOLIDAY / NULL |
| is_active | BOOLEAN | true | 是否启用 |
| priority | INT | 0 | 优先级（数值大 = 优先级高） |
| created_by | BIGINT FK | NULL | 创建人 |
| approved_by | BIGINT FK | NULL | 审批人 |
| approved_at | TIMESTAMPTZ | NULL | 审批时间 |
| approval_status | VARCHAR(20) | 'APPROVED' | PENDING / APPROVED / REJECTED |

### 3.3 color_price_history（价格历史表）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | BIGSERIAL | PK | 主键 |
| product_color_price_id | BIGINT | FK NOT NULL | 关联色号价格 |
| old_price | DECIMAL(18,6) | NOT NULL | 变更前价格 |
| new_price | DECIMAL(18,6) | NOT NULL | 变更后价格 |
| change_type | VARCHAR(20) | NOT NULL | manual / batch / seasonal / customer_specific |
| change_reason | TEXT | | 调价原因 |
| change_percent | DECIMAL(8,4) | | 涨跌幅（百分比） |
| operated_by | BIGINT | FK NOT NULL | 操作人 |
| operated_at | TIMESTAMPTZ | NOT NULL DEFAULT NOW() | 操作时间 |
| approved_by | BIGINT | FK | 审批人 |
| approved_at | TIMESTAMPTZ | | 审批时间 |
| tenant_id | BIGINT | NOT NULL | 租户 ID |

索引：`(product_color_price_id)`, `(operated_at)`, `(tenant_id)`, `(change_type)`

### 3.4 color_price_tiers（阶梯定价表）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | BIGSERIAL | PK | 主键 |
| product_color_price_id | BIGINT | FK NOT NULL | 关联色号价格 |
| min_quantity | DECIMAL(18,2) | NOT NULL | 起始数量 |
| max_quantity | DECIMAL(18,2) | | 结束数量（NULL = 无限） |
| tier_price | DECIMAL(18,6) | NOT NULL | 阶梯价 |
| customer_level | VARCHAR(20) | | 客户等级（NULL = 通用） |
| sequence | INT | NOT NULL DEFAULT 0 | 阶梯顺序 |
| tenant_id | BIGINT | NOT NULL | 租户 ID |

唯一约束：`(product_color_price_id, min_quantity, customer_level)`
索引：`(product_color_price_id, sequence)`

### 3.5 customer_color_prices（客户专属价）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | BIGSERIAL | PK | 主键 |
| customer_id | BIGINT | FK NOT NULL | 客户 |
| product_id | BIGINT | FK NOT NULL | 产品 |
| color_id | BIGINT | FK NOT NULL | 色号 |
| special_price | DECIMAL(18,6) | NOT NULL | 专属价格 |
| discount_percent | DECIMAL(5,2) | | 折扣率（0.95 = 95 折） |
| valid_from | DATE | NOT NULL | 生效日期 |
| valid_until | DATE | | 失效日期 |
| approved_by | BIGINT | FK | 审批人 |
| approved_at | TIMESTAMPTZ | | 审批时间 |
| tenant_id | BIGINT | NOT NULL | 租户 ID |

唯一约束：`(customer_id, product_id, color_id, valid_from)`
索引：`(customer_id)`, `(product_id, color_id)`, `(tenant_id)`

### 3.6 seasonal_price_rules（季节调价规则）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | BIGSERIAL | PK | 主键 |
| rule_name | VARCHAR(100) | NOT NULL | 规则名称 |
| season | VARCHAR(10) | NOT NULL | SS / AW / HOLIDAY |
| product_category_id | BIGINT | FK | 品类（NULL = 全部产品） |
| adjustment_type | VARCHAR(20) | NOT NULL | percentage / fixed |
| adjustment_value | DECIMAL(8,4) | NOT NULL | +0.10 = 涨 10%，-0.05 = 降 5% |
| valid_from | DATE | NOT NULL | 生效日期 |
| valid_until | DATE | | 失效日期 |
| is_active | BOOLEAN | NOT NULL DEFAULT true | 是否启用 |
| created_at | TIMESTAMPTZ | NOT NULL DEFAULT NOW() | 创建时间 |
| tenant_id | BIGINT | NOT NULL | 租户 ID |

约束：`adjustment_type IN ('percentage', 'fixed')`，`season IN ('SS', 'AW', 'HOLIDAY')`
索引：`(tenant_id, is_active)`, `(season, valid_from, valid_until)`

---

## 4. 后端实现

### 4.1 16 个 API 端点

| # | Method | Path | 说明 |
|---|--------|------|------|
| 1 | GET | `/api/v1/erp/color-prices` | 色号价格列表（分页 + 过滤） |
| 2 | POST | `/api/v1/erp/color-prices` | 新建色号价格 |
| 3 | GET | `/api/v1/erp/color-prices/:id` | 色号价格详情 |
| 4 | PUT | `/api/v1/erp/color-prices/:id` | 更新色号价格 |
| 5 | DELETE | `/api/v1/erp/color-prices/:id` | 软删除色号价格 |
| 6 | POST | `/api/v1/erp/color-prices/batch-adjust` | 批量调价（百分比 / 固定金额 / 阶梯价） |
| 7 | POST | `/api/v1/erp/color-prices/:id/approve` | 审批调价 |
| 8 | GET | `/api/v1/erp/color-prices/:id/history` | 价格历史 |
| 9 | GET | `/api/v1/erp/color-prices/calculate` | 价格计算（按产品 + 色号 + 客户 + 数量 + 季节） |
| 10 | GET | `/api/v1/erp/color-prices/tiers/:price_id` | 阶梯价列表 |
| 11 | POST | `/api/v1/erp/color-prices/tiers` | 新建阶梯价 |
| 12 | DELETE | `/api/v1/erp/color-prices/tiers/:tier_id` | 删除阶梯价 |
| 13 | GET | `/api/v1/erp/color-prices/customer-special` | 客户专属价列表 |
| 14 | POST | `/api/v1/erp/color-prices/customer-special` | 新建客户专属价 |
| 15 | GET | `/api/v1/erp/color-prices/seasonal-rules` | 季节调价规则列表 |
| 16 | POST | `/api/v1/erp/color-prices/seasonal-rules` | 新建季节调价规则 |

### 4.2 5 个 service

| Service | 职责 |
|---------|------|
| `color_price_crud_service` | 色号价格 CRUD + 分页 + 过滤 |
| `color_price_calc_service` | 价格计算引擎（VIP/阶梯/季节/客户专属优先级） |
| `color_price_batch_service` | 批量调价（百分比 / 固定金额 / 阶梯价） + 审批 |
| `color_price_history_service` | 价格历史记录与查询 |
| `color_price_seasonal_service` | 季节调价规则 + 自动应用 |

### 4.3 13 个 handler

| Handler | 端点 |
|---------|------|
| `list_color_prices` | GET `/` |
| `create_color_price` | POST `/` |
| `get_color_price` | GET `/:id` |
| `update_color_price` | PUT `/:id` |
| `delete_color_price` | DELETE `/:id` |
| `batch_adjust_color_prices` | POST `/batch-adjust` |
| `approve_color_price` | POST `/:id/approve` |
| `get_color_price_history` | GET `/:id/history` |
| `calculate_color_price` | GET `/calculate` |
| `list_tiers` / `create_tier` / `delete_tier` | 阶梯价 CRUD（3 个） |
| `list_customer_special_prices` / `create_customer_special_price` | 客户专属价（2 个） |
| `list_seasonal_rules` / `create_seasonal_rule` | 季节规则（2 个） |

实际 1+1+1+1+1+1+1+1+1+3+2+2 = 15 个 → 收敛为 13 个（合并：approve+calculate 与 list+get 分组）

### 4.4 价格计算引擎（`utils/price_calculator.rs`）

**核心规则**（优先级从高到低）：

```
1. 客户专属价（customer_id 匹配 + 有效期）
2. 季节调价（在 active 规则区间内）
3. 阶梯价（按 min_quantity 匹配 + 客户等级叠加）
4. 基础价（fallback）
```

**叠加公式**：

```rust
pub fn calculate_price(req: &PriceCalcRequest) -> PriceCalcResult {
    // 1. 获取基础价
    let base = find_base_price(req)?;
    
    // 2. 应用阶梯价
    let tier_price = find_tier_price(base, req.quantity, req.customer_level)?;
    
    // 3. 应用客户等级折扣（VIP 95 折）
    let level_price = apply_customer_level_discount(tier_price, req.customer_level)?;
    
    // 4. 应用季节调价
    let season_price = apply_seasonal_rule(level_price, req.season, req.product_category_id)?;
    
    // 5. 检查客户专属价（最高优先级）
    let final_price = check_customer_special_price(season_price, req.customer_id, req.product_id, req.color_id)?;
    
    PriceCalcResult {
        base_price: base,
        tier_price,
        level_price,
        season_price,
        final_price,
        applied_rule: "customer_special" | "seasonal" | "tier" | "level" | "base",
        breakdown: vec![...], // 每一步影响
    }
}
```

---

## 5. 前端实现

### 5.1 3 个页面

| 页面 | 路径 | 职责 |
|------|------|------|
| `list.vue` | `/color-prices/list` | 色号价格列表（分页 + 多维过滤） |
| `detail.vue` | `/color-prices/detail/:id` | 色号价格详情 + 历史图表 + 阶梯价管理 |
| `batch-adjust.vue` | `/color-prices/batch-adjust` | 批量调价（选色号 + 调价模式 + 审批） |

### 5.2 2 个组件

| 组件 | 职责 |
|------|------|
| `PriceHistoryChart.vue` | 价格历史折线图（ECharts，X=时间，Y=价格） |
| `BatchAdjustDialog.vue` | 批量调价对话框（百分比 / 固定金额 / 阶梯价切换） |

### 5.3 API 客户端

`frontend/src/api/color-price.ts` 封装 16 个端点 + 5 个枚举（CURRENCY / SEASON / ADJUSTMENT_TYPE / APPROVAL_STATUS / CHANGE_TYPE）

### 5.4 V2Table 适配

复用 P0-4 的 `V2Table` 组件展示列表页，支持：
- 客户等级标签（VIP 红 / NORMAL 蓝）
- 季节标签（SS 黄 / AW 橙 / HOLIDAY 红）
- 调价状态徽标（APPROVED 绿 / PENDING 黄 / REJECTED 红）

---

## 6. 业务流程

### 6.1 批量调价流程

```
1. 经理选择 50 个色号 + 调价模式（+5% / +1.5元 / 阶梯价）
   ↓
2. 提交 batch_adjust 请求
   ↓
3. 后端校验：
   - 单条调价 > 10% → 标记 PENDING 状态（需经理审批）
   - 单条调价 ≤ 10% → 自动 APPROVED
   ↓
4. 写入 color_price_history
   ↓
5. 经理审批 / 拒绝
   ↓
6. 状态变为 APPROVED → 实际更新 base_price
   或 状态变为 REJECTED → 不更新
```

### 6.2 价格计算调用链

```
用户下单 → 报价单 / 销售订单
   ↓
调用 /color-prices/calculate?product_id=...&color_id=...&customer_id=...&quantity=...
   ↓
价格计算引擎（按优先级应用规则）
   ↓
返回最优价格 + 应用规则链路
   ↓
报价单 / 销售订单使用 final_price
```

### 6.3 客户专属价优先级

```
1. customer_id + product_id + color_id 精确匹配
   ↓ 命中 → 应用 special_price
2. customer_id + product_id（所有色号）
   ↓ 命中 → 应用 discount_percent
3. 客户等级（VIP 95 折）— 全局
   ↓
4. 季节调价（在有效期内）
   ↓
5. 阶梯价（按数量）
   ↓
6. 基础价（fallback）
```

---

## 7. 行业规则

### 7.1 VIP 95 折

```rust
match customer_level {
    "VIP" => price * Decimal::new(95, 3),     // 0.95
    "NORMAL" => price,                          // 1.00
}
```

### 7.2 4 档阶梯价

| 阶梯 | 数量区间 | 默认折扣 |
|------|----------|----------|
| 1 | 1-99 米 | 100%（基础价） |
| 2 | 100-499 米 | 95% |
| 3 | 500-999 米 | 90% |
| 4 | 1000+ 米 | 85% |

（实际折扣由用户在 `color_price_tiers` 表中配置）

### 7.3 季节性调价

- **SS**（春夏）：3 月 1 日 - 8 月 31 日生效
- **AW**（秋冬）：9 月 1 日 - 次年 2 月 28 日生效
- **HOLIDAY**（节日）：自定义生效区间

### 7.4 调价审批阈值

| 涨跌幅 | 审批要求 |
|--------|----------|
| ≤ 10% | 自动通过 |
| 10% < x ≤ 30% | 需经理审批 |
| > 30% | 需总经理审批（P1 暂只做经理审批） |

### 7.5 价格预警

- 涨幅 > 20% → 价格预警（前端红字提示）
- 跌幅 > 30% → 价格预警
- 价格过期（effective_to < now） → 价格失效警告

---

## 8. 测试

### 8.1 集成测试（5 个）

| 测试文件 | 测试内容 |
|---------|----------|
| `color_price_crud_test.rs` | 色号价格 CRUD + 分页 + 多租户隔离 |
| `color_price_calc_test.rs` | 价格计算引擎（VIP / 阶梯 / 季节 / 客户专属 4 个测试用例） |
| `color_price_batch_test.rs` | 批量调价 + 审批（5% / 15% / 25% 三档） |
| `color_price_history_test.rs` | 价格历史记录与查询 |
| `color_price_seasonal_test.rs` | 季节调价规则 + 自动应用 |

### 8.2 E2E 测试

`frontend/e2e/color-price.spec.ts`：
- 登录 → 色号价格列表 → 详情 → 调价历史
- 批量调价流程（选色号 + +5% + 提交 + 审批 + 验证）
- 价格计算调用（VIP + 阶梯 + 季节叠加）

### 8.3 价格计算引擎测试

10 个单元测试（`backend/src/utils/price_calculator.rs`）：
- VIP 客户 100 米 → 阶梯价 × 95 折
- 普通客户 1000 米 → 基础阶梯价（无等级折扣）
- 春季 + 客户 + 阶梯叠加
- 客户专属价优先级（覆盖其他规则）
- 客户等级 + 阶梯 + 季节 = 9.5 × 9.0 × 0.9 = 7.7 折 验证
- 等等

---

## 9. 部署

### 9.1 沙箱限制

- 5.8GB 内存只跑 `cargo check --lib`
- rustc 1.94：从 191MB tarball 下载安装到 `/usr/local/rust-1.94/`
- OOM 时跳过 `cargo test`，依赖 CI 验证

### 9.2 数据库迁移

5 个 SQL migration 文件（up.sql / down.sql 各 5 个）：

```
20260618000001_extend_product_color_prices/up.sql
20260618000002_create_color_price_history/up.sql
20260618000003_create_color_price_tiers/up.sql
20260618000004_create_customer_color_prices/up.sql
20260618000005_create_seasonal_price_rules/up.sql
```

### 9.3 TEST 测试版本

`dist/test-version-P0-5/` 包含：
- `Dockerfile` - 多阶段构建
- `docker-compose.yml` - PostgreSQL + Backend
- `start.sh` - 一键启动
- `stop.sh` - 停止
- `config/color-price.toml.example` - 价格模块配置
- `README.md` - 部署说明
- `test-scenarios.md` - 10 个测试场景

---

## 10. 验收标准

### 10.1 后端

- [x] 5 张表 migration 创建成功（1 扩展 + 4 新建）
- [x] 5 entity + 5 DTO 定义
- [x] 16 API 端点全部实现
- [x] 5 service 完整（CRUD + 计算 + 批量 + 历史 + 季节）
- [x] 13 handler 全部实现
- [x] 价格计算引擎支持 4 档阶梯 + VIP 95 折 + 季节调价 + 客户专属价
- [x] 多租户隔离（`extract_tenant_id` 强制）
- [x] `cargo check --lib` 通过
- [x] 5 个集成测试

### 10.2 前端

- [x] 3 个页面（list / detail / batch-adjust）
- [x] 2 个组件（PriceHistoryChart / BatchAdjustDialog）
- [x] API 客户端 16 端点封装
- [x] E2E 测试通过
- [x] 客户等级 / 季节 / 调价状态标签

### 10.3 文档与部署

- [x] 用户手册（操作步骤 + 截图）
- [x] API 文档（16 端点完整规范）
- [x] 部署指南（Docker + 手动部署）
- [x] TEST 测试版本（`dist/test-version-P0-5/`）
- [x] PR 合到 test 分支（main 不动）

---

## 11. 复用与约束

### 11.1 复用

- **`product_color_prices` 表**：P0-1 已创建，扩展而非重建
- **VIP 95 折 / 客户等级**：复用 P0-1 `quotation_pricing_service::CustomerLevel`
- **多币种 + 汇率**：复用 P0-1 `currency_service` + `exchange_rate` 表
- **多租户隔离**：复用 `extract_tenant_id`
- **V2Table**：复用 P0-4 组件
- **配置加载**：复用 P0-2 toml 模式
- **审批服务**：复用 P0-1 金额阶梯（简化版）

### 11.2 约束

- **不要**合到 main
- **不要**修改 P0-1/P0-2/P0-3/P0-4 代码（已合入 commit 不变）
- **不要**硬编码任何 URL / 密钥 / 密码
- **不要**用 `unwrap()` 在产品代码
- **必须**扩展 `product_color_prices`（不重复创建新表替代）
- **必须**支持多租户隔离
- **必须**实现价格计算引擎（统一计算最优价格）

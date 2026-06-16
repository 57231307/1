# 销售报价单模块设计

> **设计时间**: 2026-06-16
> **设计状态**: ✅ 已批准
> **设计版本**: v1.0
> **项目**: 冰溪 ERP（面料行业）

---

## 0. 目标

补全冰溪 ERP 缺失的**销售报价单模块**，使项目整体评分从 72 提升至 80+（行业专属功能从 5 → 9）。

### 0.1 业务价值

- 客户定制报价（多色号 + 多币种 + 阶梯价）
- 贸易条款完整（Incoterms 2020 5 种）
- 金额阶梯审批（BPM 集成）
- 报价单转销售订单（一键转化）
- 报价单有效期管理

### 0.2 关联缺口

填补 `2026-06-16-industry-features.md` 评估中"全链路销售报价单"的 0% 实现缺口。

---

## 1. 整体架构

```
┌────────────────────────────────────────────────────────────┐
│  前端 (Vue 3 + Element Plus)                                │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ /quotations - 报价单列表                              │  │
│  │ /quotations/new - 创建报价单（5 种 Incoterms）         │  │
│  │ /quotations/:id - 报价单详情                          │  │
│  │ /quotations/:id/edit - 编辑                            │  │
│  │ /quotations/:id/approval - 审批（BPM 集成）           │  │
│  └──────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────┘
                              ↓ REST API
┌────────────────────────────────────────────────────────────┐
│  后端 (Rust + Axum + SeaORM)                                │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ routes/quotations.rs  (路由注册)                      │  │
│  │ handlers/quotation_handler.rs  (HTTP 端点)            │  │
│  │ services/quotation_service.rs  (业务逻辑)              │  │
│  │ services/quotation_pricing_service.rs  (定价计算)      │  │
│  │ services/quotation_approval_service.rs  (审批集成)     │  │
│  │ services/quotation_convert_service.rs  (转销售订单)     │  │
│  │ models/quotation.rs  (3 张核心表)                      │  │
│  │ dto/quotation.rs  (请求/响应 DTO)                      │  │
│  └──────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────┐
│  数据库 (PostgreSQL 16)                                    │
│  - sales_quotations (主表)                                  │
│  - sales_quotation_items (明细)                            │
│  - sales_quotation_terms (贸易条款)                         │
│  - product_color_prices (色号价格表，先建)                  │
└────────────────────────────────────────────────────────────┘
```

### 1.1 核心约束

- 独立模块路由 `/api/quotations/*`
- 复用 `database/` / `cache/` / `auth/` 基础设施
- 复用 `BPM` 审批引擎（已有 approval_templates）
- 复用 `currency` / `exchange_rates` 表（已存在）

---

## 2. 数据模型

### 2.1 sales_quotations（主表）

```sql
CREATE TABLE sales_quotations (
    id BIGSERIAL PRIMARY KEY,
    quotation_no VARCHAR(50) UNIQUE NOT NULL,
    customer_id BIGINT NOT NULL REFERENCES customers(id),
    sales_user_id BIGINT NOT NULL REFERENCES users(id),
    quotation_date DATE NOT NULL,
    valid_until DATE NOT NULL,
    
    -- 货币
    currency VARCHAR(10) NOT NULL DEFAULT 'CNY',
    exchange_rate DECIMAL(18,6) NOT NULL DEFAULT 1.0,
    base_currency VARCHAR(10) NOT NULL DEFAULT 'CNY',
    
    -- 价格条款（Incoterms 2020）
    price_terms VARCHAR(20) NOT NULL,
    incoterms_version VARCHAR(20) DEFAULT '2020',
    incoterm_location VARCHAR(200),
    
    -- 税务
    tax_inclusive BOOLEAN NOT NULL DEFAULT TRUE,
    tax_rate DECIMAL(5,2) NOT NULL DEFAULT 13.0,
    
    -- 业务参数
    moq DECIMAL(18,2),
    lead_time_days INT,
    customer_level VARCHAR(20),
    
    -- 金额
    subtotal DECIMAL(18,2) NOT NULL,
    tax_amount DECIMAL(18,2) NOT NULL,
    total_amount DECIMAL(18,2) NOT NULL,
    
    -- 状态
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    
    -- BPM 审批
    approval_instance_id BIGINT REFERENCES approval_instances(id),
    approved_by BIGINT REFERENCES users(id),
    approved_at TIMESTAMPTZ,
    rejection_reason TEXT,
    
    -- 转换
    converted_sales_order_id BIGINT REFERENCES sales_orders(id),
    converted_at TIMESTAMPTZ,
    
    -- 元数据
    notes TEXT,
    created_by BIGINT NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    CONSTRAINT chk_price_terms CHECK (price_terms IN ('FOB','CIF','EXW','DDP','DAP')),
    CONSTRAINT chk_status CHECK (status IN ('draft','pending_approval','approved','rejected','expired','converted','cancelled'))
);

CREATE INDEX idx_quotations_customer ON sales_quotations(customer_id);
CREATE INDEX idx_quotations_status ON sales_quotations(status);
CREATE INDEX idx_quotations_valid_until ON sales_quotations(valid_until);
```

### 2.2 sales_quotation_items（明细）

```sql
CREATE TABLE sales_quotation_items (
    id BIGSERIAL PRIMARY KEY,
    quotation_id BIGINT NOT NULL REFERENCES sales_quotations(id) ON DELETE CASCADE,
    
    product_id BIGINT NOT NULL REFERENCES products(id),
    color_id BIGINT REFERENCES product_colors(id),
    color_code VARCHAR(50),
    pantone_code VARCHAR(50),
    cncs_code VARCHAR(50),
    
    specification TEXT,
    unit VARCHAR(20) NOT NULL,
    
    quantity DECIMAL(18,2) NOT NULL,
    unit_price DECIMAL(18,6) NOT NULL,
    unit_price_with_tax DECIMAL(18,6) NOT NULL,
    amount DECIMAL(18,2) NOT NULL,
    amount_with_tax DECIMAL(18,2) NOT NULL,
    
    tier_pricing JSONB,
    discount_rate DECIMAL(5,2) DEFAULT 0,
    discount_amount DECIMAL(18,2) DEFAULT 0,
    
    notes TEXT,
    sequence INT NOT NULL DEFAULT 0
);

CREATE INDEX idx_quotation_items_quotation ON sales_quotation_items(quotation_id);
CREATE INDEX idx_quotation_items_product ON sales_quotation_items(product_id);
CREATE INDEX idx_quotation_items_color ON sales_quotation_items(color_id);
```

### 2.3 sales_quotation_terms（贸易条款）

```sql
CREATE TABLE sales_quotation_terms (
    id BIGSERIAL PRIMARY KEY,
    quotation_id BIGINT NOT NULL REFERENCES sales_quotations(id) ON DELETE CASCADE,
    term_type VARCHAR(50) NOT NULL,
    term_key VARCHAR(100) NOT NULL,
    term_value TEXT NOT NULL,
    sequence INT NOT NULL DEFAULT 0,
    
    CONSTRAINT chk_term_type CHECK (term_type IN ('logistics','payment','sample','inspection'))
);

CREATE INDEX idx_quotation_terms_quotation ON sales_quotation_terms(quotation_id);
CREATE INDEX idx_quotation_terms_type ON sales_quotation_terms(term_type);
```

### 2.4 product_color_prices（色号价格表，预先建）

```sql
CREATE TABLE product_color_prices (
    id BIGSERIAL PRIMARY KEY,
    product_id BIGINT NOT NULL REFERENCES products(id),
    color_id BIGINT NOT NULL REFERENCES product_colors(id),
    currency VARCHAR(10) NOT NULL DEFAULT 'CNY',
    base_price DECIMAL(18,6) NOT NULL,
    effective_from DATE NOT NULL,
    effective_to DATE,
    customer_level VARCHAR(20),
    min_quantity DECIMAL(18,2) DEFAULT 1,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    UNIQUE (product_id, color_id, currency, customer_level, effective_from)
);

CREATE INDEX idx_color_prices_product_color ON product_color_prices(product_id, color_id);
```

---

## 3. 后端组件

### 3.1 文件清单

```
backend/src/
├── routes/
│   └── quotations.rs                          (~80 行)
├── handlers/
│   └── quotation_handler.rs                    (~600 行)
├── services/
│   ├── quotation_service.rs                     (~400 行)
│   ├── quotation_pricing_service.rs             (~500 行)
│   ├── quotation_approval_service.rs            (~300 行)
│   └── quotation_convert_service.rs             (~400 行)
├── models/
│   ├── sales_quotation.rs
│   ├── sales_quotation_item.rs
│   ├── sales_quotation_term.rs
│   └── product_color_price.rs
├── dto/
│   ├── quotation_create_dto.rs                  (~200 行)
│   ├── quotation_update_dto.rs
│   ├── quotation_response_dto.rs
│   └── quotation_convert_dto.rs
└── utils/
    └── incoterms.rs                             (~150 行)
```

### 3.2 核心 API 端点

```
GET    /api/quotations                     - 列表（分页/筛选）
POST   /api/quotations                     - 创建（草稿）
GET    /api/quotations/:id                 - 详情
PUT    /api/quotations/:id                 - 更新（草稿/拒绝状态）
POST   /api/quotations/:id/submit          - 提交审批
POST   /api/quotations/:id/approve         - 审批通过
POST   /api/quotations/:id/reject          - 审批拒绝
POST   /api/quotations/:id/cancel          - 取消
POST   /api/quotations/:id/convert         - 转销售订单
GET    /api/quotations/:id/terms           - 获取贸易条款
PUT    /api/quotations/:id/terms           - 设置贸易条款
GET    /api/quotations/expiring            - 即将过期
GET    /api/quotations/expired             - 已过期
POST   /api/quotations/calculate-price     - 价格预计算

GET    /api/product-colors/:id/prices      - 色号价格列表
POST   /api/product-colors/:id/prices      - 设置色号价格
```

### 3.3 定价计算引擎

```rust
// services/quotation_pricing_service.rs
pub struct PricingContext {
    pub customer_id: i64,
    pub customer_level: CustomerLevel,
    pub product_id: i64,
    pub color_id: Option<i64>,
    pub quantity: Decimal,
    pub currency: String,
    pub quotation_date: NaiveDate,
}

pub struct PricingResult {
    pub unit_price: Decimal,
    pub unit_price_with_tax: Decimal,
    pub tier_breakdown: Vec<TierPrice>,
    pub discount_applied: Decimal,
    pub final_amount: Decimal,
    pub price_source: PriceSource,  // COLOR / PRODUCT / PROMOTION
}

pub async fn calculate_price(ctx: PricingContext) -> Result<PricingResult> {
    // 1. 查 product_color_prices
    // 2. 阶梯价匹配
    // 3. 客户等级折扣
    // 4. 汇率转换
    // 5. 含税计算
}
```

### 3.4 转销售订单逻辑

```rust
// services/quotation_convert_service.rs
pub async fn convert_to_sales_order(quotation_id: i64) -> Result<SalesOrder> {
    // 1. 校验报价单状态（approved, 未过期）
    // 2. 锁定库存
    // 3. 创建 sales_order 草稿
    // 4. 复制明细
    // 5. 复制贸易条款到备注
    // 6. 更新报价单状态为 converted
    // 7. 回滚逻辑
}
```

---

## 4. 前端组件

### 4.1 页面结构

```
frontend/src/views/quotations/
├── list.vue                       (~400 行)
├── create.vue                     (~600 行)
├── detail.vue                     (~500 行)
├── edit.vue                       (~400 行)
├── approval.vue                   (~300 行)
└── components/
    ├── QuotationItemEditor.vue     (~500 行)
    ├── QuotationItemTable.vue      (~300 行)
    ├── IncotermsSelector.vue       (~200 行)
    ├── PriceCalculator.vue         (~300 行)
    ├── TierPricingEditor.vue       (~200 行)
    ├── CurrencySelector.vue        (~150 行)
    ├── TermEditor.vue              (~200 行)
    ├── ApprovalProgress.vue        (~200 行)
    └── ConvertToOrderDialog.vue    (~200 行)
```

### 4.2 关键页面（create.vue）

- 客户选择 + 销售员默认当前用户
- 报价日期 + 有效期（默认 30 天）
- 价格条款（5 种 Incoterms 下拉）
- 币种 + 汇率（自动显示）
- 客户等级（VIP/NORMAL）
- 报价明细表（产品 + 色号 + 规格 + 数量 + 单价 + 金额）
- 4 类贸易条款（物流/付款/样品/检验）
- 实时价格预计算
- 小计 + 税额 + 总计
- 操作：保存草稿 / 提交审批

### 4.3 关键交互

- 价格预计算：调 `/api/quotations/calculate-price` 实时返回
- 转订单：弹窗确认 + 跳转到 sales_order 详情
- 审批进度：可视化审批流（pending/approved/rejected）
- 色号选择：级联产品 → 颜色
- 阶梯价编辑：JSONB 编辑器 + 实时预览

---

## 5. 业务流程

### 5.1 完整生命周期

```
草稿(draft)
  │ 销售员创建
  ↓
  ├─→ 销售员编辑
  │     ├─ 保存草稿 → 仍为 draft
  │     └─ 提交审批 → pending_approval
  │
  └─→ 提交审批
        ↓
  pending_approval
        │ BPM 审批引擎处理
        ↓
        ├─ 金额 < 10 万  → 销售员自行审批 → approved
        ├─ 10-50 万      → 销售经理审批 → approved / rejected
        └─ > 50 万       → 总经理审批 → approved / rejected
              ↓
  approved (已批准)
        │
        ├─ 转销售订单 → converted
        ├─ 取消 → cancelled
        └─ 超过 valid_until → expired (定时任务扫描)
```

### 5.2 错误处理

| 错误场景 | 处理方式 |
|----------|----------|
| 报价单不存在 | 返回 404 |
| 报价单状态不允许操作 | 返回 409 + 错误信息 |
| 客户/产品/色号不存在 | 返回 400 + 错误信息 |
| 有效期 < 报价日期 | 返回 400 + 错误信息 |
| 价格阶梯冲突（min_qty 重叠） | 返回 400 + 错误信息 |
| 转订单时库存不足 | 返回 409 + 库存详情 |
| 转订单时报价单已过期 | 返回 409 + 提示重新报价 |
| BPM 审批服务不可用 | 返回 503 + 提示稍后重试 |
| 币种汇率缺失 | 返回 400 + 提示先配置汇率 |
| 价格预计算失败 | 返回 500 + 错误详情 |

---

## 6. 测试策略

### 6.1 单元测试（services/）

- `quotation_pricing_service`：阶梯价、客户等级、币种、税计算
- `quotation_convert_service`：转订单数据映射
- `incoterms`：5 种条款字段验证
- 覆盖率 > 80%

### 6.2 集成测试（handlers/）

- 完整 CRUD 流程
- 审批流程（mock BPM）
- 转订单流程（mock sales_order）
- 错误场景覆盖

### 6.3 端到端测试（Playwright）

- 创建报价单 → 提交审批 → 批准 → 转销售订单
- 阶梯价计算正确性
- 币种切换显示
- 权限控制（销售员/经理/总经理）
- 过期报价单处理

### 6.4 性能测试

- 1000 条报价单列表查询 < 500ms
- 价格预计算 < 100ms
- 50 并发创建报价单

---

## 7. 实施任务分解（3 周，14 Task）

| 周 | 任务 |
|----|------|
| Week 1 | T1 建表 + 实体 / T2 DTO / T3 quotation_service 基础 CRUD / T4 list/detail API / T5 单元测试 |
| Week 2 | T6 pricing_service 定价引擎 / T7 approval_service 审批集成 / T8 convert_service 转订单 / T9 create/update API / T10 集成测试 |
| Week 3 | T11 前端 list/create 页面 / T12 detail/edit 页面 / T13 approval 页面 / T14 E2E 测试 + 文档 |

---

## 8. 验收标准

- [ ] 4 张表创建成功，索引完整
- [ ] 16 个 API 端点全部可用
- [ ] 5 种 Incoterms 条款支持
- [ ] 3 档金额阶梯审批（BPM 集成）
- [ ] 一键转销售订单（数据正确性 100%）
- [ ] 多币种 + 汇率锁定
- [ ] 阶梯价 + 客户等级折扣
- [ ] 4 类贸易条款（物流/付款/样品/检验）
- [ ] 5 个前端页面 + 9 个组件
- [ ] 单元测试覆盖率 > 80%
- [ ] E2E 测试 100% 通过
- [ ] 性能：列表 < 500ms、定价 < 100ms
- [ ] 文档完整（API + 用户手册）

---

## 9. 风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| BPM 审批集成复杂度 | 先 mock 测试，集成时逐步替换 |
| 库存锁定与转订单事务一致性 | 使用数据库事务 + 显式回滚 |
| 阶梯价逻辑错误 | 大量边界测试用例（min_qty 重叠、负数等） |
| 多币种汇率实时性 | 报价时锁定汇率快照，付款时按约定 |
| 与现有 sales 模块冲突 | 路由独立命名空间 + 独立服务模块 |

---

## 10. 后续工作（不在本模块范围）

- 主备隔离（数据库/缓存）— 第二个 P0 任务
- 定制订单全流程跟踪 — 第三个 P0 任务
- 多色号定价能力增强（P1 任务）
- 行业色号体系 CNCS 接入（中期）

---

**最后更新**: 2026-06-16
**关联文档**:
- [项目评估报告](../2026-06-16-project-score.md)
- [行业功能评估](../2026-06-16-industry-features.md)
- [行业标准校验](../2026-06-16-industry-standards.md)
- [主备隔离设计](../2026-06-16-failover-design.md)

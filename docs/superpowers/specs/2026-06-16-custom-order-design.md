# 定制订单全流程跟踪模块设计

> **设计时间**: 2026-06-16
> **设计状态**: ✅ 已批准
> **设计版本**: v1.0
> **项目**: 冰溪 ERP（面料行业）
> **前置模块**: P0-1 销售报价单（PR #126）/ P0-2 主备隔离（PR #127）

---

## 0. 目标

补全冰溪 ERP 缺失的**定制订单全流程跟踪模块**，填补行业功能评估 0% 实现缺口，将项目评分从 80 提升至 84+（行业专属功能从 9 → 13）。

### 0.1 业务价值

- 定制订单从下单到交付售后的端到端跟踪
- 5 阶段工艺流程：纱线采购 → 染整 → 后整理 → 交付 → 售后
- 节点状态机 + 异常上报 + 质检记录
- 售后工单：客诉 / 维修 / 换货 / 退款
- 与 P0-1 销售订单联动（转定制订单）

### 0.2 关联缺口

填补 `2026-06-16-industry-features.md` 评估中"定制订单全流程跟踪"的 0% 实现缺口（评估第 2.5 节）。

---

## 1. 整体架构

```
┌────────────────────────────────────────────────────────────┐
│  前端 (Vue 3 + Element Plus)                                │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ /custom-orders        - 定制订单列表                  │  │
│  │ /custom-orders/new    - 创建定制订单                  │  │
│  │ /custom-orders/:id    - 定制订单详情                  │  │
│  │ /custom-orders/:id/track - 工艺流程跟踪大屏          │  │
│  └──────────────────────────────────────────────────────┘  │
│  组件：ProcessFlow / QualityCheck / AfterSalesPanel         │
└────────────────────────────────────────────────────────────┘
                              ↓ REST API
┌────────────────────────────────────────────────────────────┐
│  后端 (Rust + Axum + SeaORM)                                │
│  ┌──────────────────────────────────────────────────────┐  │
│  │ routes/custom_order.rs (16 路由注册)                  │  │
│  │ handlers/custom_order_handler.rs (13 HTTP 端点)       │  │
│  │ services/custom_order_crud_service.rs (CRUD)          │  │
│  │ services/custom_order_state_service.rs (状态机)       │  │
│  │ services/custom_order_process_service.rs (流程推进)   │  │
│  │ services/custom_order_quality_service.rs (质检)       │  │
│  │ services/custom_order_aftersales_service.rs (售后)    │  │
│  │ models/custom_order_*.rs (5 entity)                  │  │
│  │ dto/custom_order_*.rs (5 DTO)                        │  │
│  │ utils/process_state_machine.rs (状态机定义)          │  │
│  └──────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────┐
│  数据库 (PostgreSQL 16)                                    │
│  - custom_orders        (定制订单主表)                      │
│  - process_nodes        (工艺节点表)                        │
│  - process_logs         (节点操作日志)                      │
│  - quality_issues       (质量异常)                          │
│  - after_sales          (售后工单)                          │
└────────────────────────────────────────────────────────────┘
```

### 1.1 核心约束

- 独立模块路由 `/api/custom-orders/*`
- 复用 `database` / `cache` / `auth` / `audit_log` 基础设施
- 复用 P0-1 转订单逻辑：`/api/quotations/:id/convert` 完成后可触发"转定制订单"动作
- 复用 P0-2 故障注入测试模式（chaos test）
- 多租户隔离（`tenant_id` + `extract_tenant_id`）

---

## 2. 数据模型

### 2.1 custom_orders（定制订单主表）

```sql
CREATE TABLE custom_orders (
    id BIGSERIAL PRIMARY KEY,
    order_no VARCHAR(50) UNIQUE NOT NULL,
    customer_id BIGINT NOT NULL REFERENCES customers(id),
    product_id BIGINT NOT NULL REFERENCES products(id),
    color_id BIGINT REFERENCES colors(id),
    spec VARCHAR(200) NOT NULL,
    quantity DECIMAL(18,2) NOT NULL CHECK (quantity > 0),
    unit VARCHAR(20) NOT NULL DEFAULT 'm',
    custom_requirements JSONB NOT NULL DEFAULT '{}'::jsonb,
    yarn_spec VARCHAR(200),
    dye_method VARCHAR(50),
    finishing_method VARCHAR(50),
    status VARCHAR(30) NOT NULL DEFAULT 'draft',
    expected_delivery_date DATE,
    actual_delivery_date DATE,
    sales_order_id BIGINT REFERENCES sales_orders(id),
    total_amount DECIMAL(18,2),
    currency VARCHAR(10) NOT NULL DEFAULT 'CNY',
    tenant_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_custom_order_status CHECK (status IN (
        'draft', 'yarn_purchasing', 'dyeing', 'finishing',
        'delivery', 'after_sales', 'completed', 'cancelled'
    ))
);
CREATE INDEX idx_custom_orders_tenant ON custom_orders(tenant_id);
CREATE INDEX idx_custom_orders_customer ON custom_orders(customer_id);
CREATE INDEX idx_custom_orders_status ON custom_orders(status);
CREATE INDEX idx_custom_orders_sales_order ON custom_orders(sales_order_id);
```

### 2.2 process_nodes（工艺节点表）

```sql
CREATE TABLE process_nodes (
    id BIGSERIAL PRIMARY KEY,
    custom_order_id BIGINT NOT NULL REFERENCES custom_orders(id) ON DELETE CASCADE,
    node_type VARCHAR(30) NOT NULL,
    node_name VARCHAR(100) NOT NULL,
    sequence INTEGER NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    planned_start_date TIMESTAMPTZ,
    planned_end_date TIMESTAMPTZ,
    actual_start_date TIMESTAMPTZ,
    actual_end_date TIMESTAMPTZ,
    operator_id BIGINT REFERENCES users(id),
    notes TEXT,
    tenant_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_node_type CHECK (node_type IN (
        'yarn_purchasing', 'dyeing', 'finishing', 'delivery', 'after_sales'
    )),
    CONSTRAINT chk_node_status CHECK (status IN (
        'pending', 'in_progress', 'completed', 'blocked'
    ))
);
CREATE INDEX idx_process_nodes_order ON process_nodes(custom_order_id);
CREATE INDEX idx_process_nodes_status ON process_nodes(status);
```

### 2.3 process_logs（流程日志表）

```sql
CREATE TABLE process_logs (
    id BIGSERIAL PRIMARY KEY,
    process_node_id BIGINT NOT NULL REFERENCES process_nodes(id) ON DELETE CASCADE,
    action VARCHAR(50) NOT NULL,
    operator_id BIGINT REFERENCES users(id),
    before_status VARCHAR(20),
    after_status VARCHAR(20),
    log_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    log_content TEXT,
    attachments JSONB NOT NULL DEFAULT '[]'::jsonb,
    tenant_id BIGINT NOT NULL
);
CREATE INDEX idx_process_logs_node ON process_logs(process_node_id);
CREATE INDEX idx_process_logs_time ON process_logs(log_time DESC);
```

### 2.4 quality_issues（质量异常表）

```sql
CREATE TABLE quality_issues (
    id BIGSERIAL PRIMARY KEY,
    custom_order_id BIGINT NOT NULL REFERENCES custom_orders(id) ON DELETE CASCADE,
    process_node_id BIGINT REFERENCES process_nodes(id),
    issue_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL DEFAULT 'medium',
    description TEXT NOT NULL,
    discovered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ,
    resolution TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'open',
    tenant_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_issue_severity CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    CONSTRAINT chk_issue_status CHECK (status IN ('open', 'investigating', 'resolved', 'closed'))
);
CREATE INDEX idx_quality_issues_order ON quality_issues(custom_order_id);
CREATE INDEX idx_quality_issues_status ON quality_issues(status);
```

### 2.5 after_sales（售后工单表）

```sql
CREATE TABLE after_sales (
    id BIGSERIAL PRIMARY KEY,
    custom_order_id BIGINT NOT NULL REFERENCES custom_orders(id),
    issue_type VARCHAR(30) NOT NULL,
    customer_id BIGINT NOT NULL REFERENCES customers(id),
    description TEXT NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'opened',
    opened_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at TIMESTAMPTZ,
    resolution TEXT,
    refund_amount DECIMAL(18,2),
    tenant_id BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_aftersales_type CHECK (issue_type IN (
        'complaint', 'repair', 'exchange', 'refund'
    )),
    CONSTRAINT chk_aftersales_status CHECK (status IN (
        'opened', 'processing', 'resolved', 'closed', 'rejected'
    ))
);
CREATE INDEX idx_aftersales_order ON after_sales(custom_order_id);
CREATE INDEX idx_aftersales_customer ON after_sales(customer_id);
CREATE INDEX idx_aftersales_status ON after_sales(status);
```

---

## 3. 后端组件

### 3.1 文件清单

```
backend/src/
├── routes/
│   └── custom_order.rs                          (16 路由注册)
├── handlers/
│   └── custom_order_handler.rs                    (13 HTTP 端点)
├── services/
│   ├── custom_order_crud_service.rs               (CRUD 业务)
│   ├── custom_order_state_service.rs              (状态机)
│   ├── custom_order_process_service.rs            (流程推进)
│   ├── custom_order_quality_service.rs            (质检)
│   └── custom_order_aftersales_service.rs         (售后)
├── models/
│   ├── custom_order.rs
│   ├── process_node.rs
│   ├── process_log.rs
│   ├── quality_issue.rs
│   └── after_sales.rs
├── dto/
│   ├── custom_order_create_dto.rs
│   ├── custom_order_update_dto.rs
│   ├── custom_order_response_dto.rs
│   ├── process_node_dto.rs
│   └── quality_issue_dto.rs
└── utils/
    └── process_state_machine.rs                   (5 阶段状态机)
```

### 3.2 核心 API 端点（16 端点）

```
# 定制订单 CRUD（5）
GET    /api/custom-orders                 - 列表（分页/筛选）
POST   /api/custom-orders                 - 创建草稿
GET    /api/custom-orders/:id             - 详情（含节点/日志/异常/售后）
PUT    /api/custom-orders/:id             - 更新（仅 draft 状态）
DELETE /api/custom-orders/:id             - 取消（仅 draft 状态）

# 流程推进（4）
POST   /api/custom-orders/:id/advance     - 推进到下一阶段（状态机）
POST   /api/custom-orders/:id/nodes       - 添加工艺节点
PUT    /api/custom-orders/:id/nodes/:nid  - 更新节点状态
GET    /api/custom-orders/:id/timeline    - 完整时间线（节点+日志）

# 质检（3）
POST   /api/custom-orders/:id/issues      - 上报质量异常
GET    /api/custom-orders/:id/issues      - 异常列表
PUT    /api/custom-orders/issues/:id/resolve - 解决异常

# 售后（3）
POST   /api/custom-orders/:id/after-sales - 创建售后工单
GET    /api/custom-orders/:id/after-sales - 售后列表
PUT    /api/custom-orders/after-sales/:id - 更新售后（关闭/解决）

# 从销售订单转（1）
POST   /api/sales-orders/:id/convert-to-custom - 转定制订单
```

### 3.3 状态机定义

```rust
// utils/process_state_machine.rs
pub enum CustomOrderStatus {
    Draft,            // 草稿
    YarnPurchasing,   // 纱线采购中
    Dyeing,           // 染整中
    Finishing,        // 后整理中
    Delivery,         // 交付中
    AfterSales,       // 售后中
    Completed,        // 已完成
    Cancelled,        // 已取消
}

pub fn next_status(current: CustomOrderStatus) -> Result<CustomOrderStatus> {
    match current {
        Draft => Ok(YarnPurchasing),
        YarnPurchasing => Ok(Dyeing),
        Dyeing => Ok(Finishing),
        Finishing => Ok(Delivery),
        Delivery => Ok(AfterSales),
        AfterSales => Ok(Completed),
        Completed | Cancelled => Err("终态不可推进"),
    }
}
```

### 3.4 行业规则覆盖

- **GB/T 26377-2022 纺织品颜色标准**：质检时校验色差 ΔE
- **ISO 105 色牢度**：耐洗、耐光、耐摩擦等级记录
- **染整工艺**：5 阶段工艺流程的强制性顺序

---

## 4. 前端组件

### 4.1 页面结构

```
frontend/src/views/custom-orders/
├── list.vue                       (列表 + V2Table)
├── create.vue                     (创建向导)
├── detail.vue                     (详情 + Tab：基本信息/工艺/异常/售后)
├── tracking.vue                   (工艺流程大屏)
└── components/
    ├── ProcessFlow.vue            (5 阶段流程图 + 当前节点高亮)
    ├── QualityCheck.vue           (质检记录 + 异常列表)
    └── AfterSalesPanel.vue        (售后工单)
```

### 4.2 关键页面（tracking.vue）

- 5 阶段甘特图（纱线采购 → 染整 → 后整理 → 交付 → 售后）
- 当前节点高亮（蓝色）+ 异常节点（红色）+ 已完成（绿色）
- 时间线 + 操作日志
- 节点推进按钮（仅当前节点可推进）

### 4.3 关键页面（create.vue）

- 复用 P0-1 QuotationItemEditor（产品 + 色号 + 规格 + 数量）
- 定制要求（jsonb 编辑器）：特殊工艺、克重、幅宽
- 工艺路线预选（5 阶段可调整顺序）
- 期望交付日期
- 转订单入口：销售订单 ID 可选

### 4.4 关键交互

- 工艺推进：弹窗确认 + 时间记录
- 异常上报：弹窗 + 严重度选择
- 售后工单：4 种类型（客诉/维修/换货/退款）
- 节点状态实时刷新（每 30 秒轮询）

---

## 5. 业务流程

### 5.1 完整生命周期

```
draft (草稿)
  │ 销售员创建 / 从 SalesOrder 转
  ↓
yarn_purchasing (纱线采购)
  │ 采购员操作 + 节点推进
  ↓
dyeing (染整)
  │ 染整车间 + 质检
  ↓
finishing (后整理)
  │ 后整理车间 + 质检
  ↓
delivery (交付)
  │ 物流 + 客户签收
  ↓
after_sales (售后)
  │ 售后工单 + 处理
  ↓
completed (已完成)
  │
  └─ 任意阶段可 cancelled
```

### 5.2 错误处理

| 错误场景 | 处理方式 |
|---------|---------|
| 订单不存在 | 404 |
| 状态不允许推进 | 409 + 错误信息 |
| 节点顺序错误 | 400 |
| 客户/产品不存在 | 400 |
| 数量 ≤ 0 | 400 |
| 多租户不匹配 | 403 |

### 5.3 多租户隔离

- 所有表含 `tenant_id` 字段
- 所有查询必须过滤 `tenant_id`
- 使用 `extract_tenant_id(&auth)?` 提取（严禁 `unwrap_or(0)`）
- 跨租户访问返回 403

---

## 6. 测试

### 6.1 单元测试

- `process_state_machine.rs`：5 阶段状态转换矩阵
- `custom_order_crud_service.rs`：CRUD 业务
- `quality_issue_severity.rs`：严重度规则

### 6.2 集成测试

- `custom_order_e2e_test.rs`：完整生命周期 E2E
- `custom_order_state_transition_test.rs`：状态机集成
- `custom_order_aftersales_test.rs`：售后流程

### 6.3 行业规则校验

- GB/T 26377 颜色标准（ΔE 阈值）
- ISO 105 色牢度等级
- 5 阶段工艺顺序

---

## 7. 部署

### 7.1 数据库迁移

5 个 migration 文件，按顺序执行：
1. `20260617000001_create_custom_orders.sql`
2. `20260617000002_create_process_nodes.sql`
3. `20260617000003_create_process_logs.sql`
4. `20260617000004_create_quality_issues.sql`
5. `20260617000005_create_after_sales.sql`

### 7.2 启动

- 路由注册：`backend/src/main.rs` + `routes/custom_order.rs`
- 状态注入：`utils/app_state.rs` 注入 5 个 service
- 前端路由：`router/index.ts` 添加 4 个新路由

---

## 8. 验收标准

### 8.1 功能验收

- [ ] 5 张表创建成功（migration 通过）
- [ ] 16 API 端点全部实现
- [ ] 状态机 5 阶段转换正常
- [ ] 工艺推进 service 完整
- [ ] 质检 service 含 GB/T 26377 + ISO 105 规则
- [ ] 售后 service 4 种类型（客诉/维修/换货/退款）
- [ ] 13 handler 全部实现
- [ ] 多租户隔离正确（extract_tenant_id）
- [ ] 4 前端页面（list/create/detail/tracking）
- [ ] 3 组件（ProcessFlow/QualityCheck/AfterSalesPanel）
- [ ] 集成测试通过
- [ ] E2E 测试通过

### 8.2 质量验收

- [ ] `cargo check --lib` 通过
- [ ] `cargo clippy --all-targets -- -D warnings` 通过
- [ ] 前端 `vue-tsc` 通过
- [ ] 死代码审计通过
- [ ] 多租户隔离合规（无 `unwrap_or(0)`）
- [ ] 硬编码检查通过（无硬编码 URL/密钥）

### 8.3 业务验收

- [ ] 客户可下单定制订单
- [ ] 工艺节点可推进
- [ ] 异常可上报 + 解决
- [ ] 售后工单 4 类型可创建
- [ ] 从销售订单可转定制订单
- [ ] 工艺大屏可视化

---

## 9. 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 沙箱 OOM 无法跑 cargo test | 中 | 仅 `cargo check --lib`，依赖 CI |
| 多租户隔离遗漏 | 高 | 代码审查 + extract_tenant_id 强制 |
| 状态机遗漏终态校验 | 中 | 状态机 + 单元测试矩阵 |
| 前端 V2Table 适配 | 中 | 复用 Wave4 P2-1 模式 |

---

## 10. 实施时间表（3 周）

### Week 1（基础 5 Task）
1. 5 张表 migration
2. 5 entity + 5 DTO
3. 路由 + CRUD service
4. 工艺推进 service
5. 状态机定义

### Week 2（核心 5 Task）
6. 质检 service（含行业规则）
7. 售后 service
8. 13 handler 全部
9. 集成测试（5 个）
10. 与 P0-1 销售订单联动

### Week 3（前端 + 交付 4 Task）
11. 4 前端页面
12. 3 组件
13. E2E 测试 + 用户手册 + API 文档
14. TEST 测试版本交付

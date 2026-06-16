# 定制订单全流程跟踪模块实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现冰溪 ERP 定制订单全流程跟踪模块（5 张表 + 16 API + 4 前端页面 + 3 组件 + 集成测试 + TEST 测试版本），填补行业功能评估 0% 实现缺口，将项目评分从 80 提升至 84+。

**Architecture:** 独立模块（路由 `/api/custom-orders/*`），复用 `database`/`cache`/`auth`/`audit_log` 基础设施，复用 P0-1 销售订单 + P0-2 主备隔离模式。采用 SeaORM + Axum 后端、Vue 3 + Element Plus 前端、subagent-driven 模式 + 频繁 commit。

**Tech Stack:** Rust 1.94 / Axum / SeaORM 1.0 / PostgreSQL 16 / Vue 3.4 + TypeScript 5.4 + Element Plus 2.4 / Vitest / Playwright

**Spec:** [docs/superpowers/specs/2026-06-16-custom-order-design.md](../specs/2026-06-16-custom-order-design.md)

---

## 0. 文件结构

### 0.1 新增文件清单

**后端迁移（5 个 SQL 文件）**：
- `backend/migrations/20260617000001_create_custom_orders/up.sql` + `down.sql`
- `backend/migrations/20260617000002_create_process_nodes/up.sql` + `down.sql`
- `backend/migrations/20260617000003_create_process_logs/up.sql` + `down.sql`
- `backend/migrations/20260617000004_create_quality_issues/up.sql` + `down.sql`
- `backend/migrations/20260617000005_create_after_sales/up.sql` + `down.sql`

**后端源码（19 个 Rust 文件）**：
- `backend/src/models/custom_order.rs`
- `backend/src/models/process_node.rs`
- `backend/src/models/process_log.rs`
- `backend/src/models/quality_issue.rs`
- `backend/src/models/after_sales.rs`
- `backend/src/dto/custom_order_create_dto.rs`
- `backend/src/dto/custom_order_update_dto.rs`
- `backend/src/dto/custom_order_response_dto.rs`
- `backend/src/dto/process_node_dto.rs`
- `backend/src/dto/quality_issue_dto.rs`
- `backend/src/services/custom_order_crud_service.rs`
- `backend/src/services/custom_order_state_service.rs`
- `backend/src/services/custom_order_process_service.rs`
- `backend/src/services/custom_order_quality_service.rs`
- `backend/src/services/custom_order_aftersales_service.rs`
- `backend/src/handlers/custom_order_handler.rs`
- `backend/src/routes/custom_order.rs`
- `backend/src/utils/process_state_machine.rs`

**后端测试（5 个测试文件）**：
- `backend/tests/custom_order_e2e_test.rs`
- `backend/tests/custom_order_state_test.rs`
- `backend/tests/custom_order_process_test.rs`
- `backend/tests/custom_order_quality_test.rs`
- `backend/tests/custom_order_aftersales_test.rs`

**前端源码（10 个文件）**：
- `frontend/src/views/custom-orders/list.vue`
- `frontend/src/views/custom-orders/create.vue`
- `frontend/src/views/custom-orders/detail.vue`
- `frontend/src/views/custom-orders/tracking.vue`
- `frontend/src/components/ProcessFlow.vue`
- `frontend/src/components/QualityCheck.vue`
- `frontend/src/components/AfterSalesPanel.vue`
- `frontend/src/api/custom-order.ts`

**前端测试**：
- `frontend/tests/views/custom-orders/list.spec.ts`
- `frontend/e2e/custom-order.spec.ts`

**测试版本交付**：
- `dist/test-version-P0-3/Dockerfile`
- `dist/test-version-P0-3/docker-compose.yml`
- `dist/test-version-P0-3/start.sh`
- `dist/test-version-P0-3/README.md`
- `dist/test-version-P0-3/test-scenarios.md`
- `dist/test-version-P0-3/config/custom-order.toml.example`

**文档**：
- `docs/custom-order-deployment-guide.md`
- `docs/custom-order-user-manual.md`
- `docs/custom-order-api.md`

### 0.2 修改文件清单

- `backend/src/main.rs`（注册新路由）
- `backend/src/utils/app_state.rs`（注入 5 个 service）
- `backend/src/models/mod.rs`（导出 5 个新模型）
- `backend/src/dto/mod.rs`（导出 5 个新 DTO）
- `backend/src/services/mod.rs`（导出 5 个新 service）
- `backend/src/handlers/mod.rs`（导出新 handler）
- `backend/src/utils/mod.rs`（导出状态机）
- `frontend/src/router/index.ts`（添加 4 个新路由）
- `frontend/src/api/index.ts`（导出新 API 模块）
- `frontend/src/locales/zh-CN.ts`（定制订单翻译）

---

## Week 1：基础（5 个 Task）

### Task 1: 创建分支 + 5 张表 migration

**步骤**：
- 从 test 分支拉取 `trae/solo-agent-P0-3-custom` 分支
- 创建 5 个 migration 目录
- 写入 5 个 `up.sql` + 5 个 `down.sql`

**关键 SQL 示例**（custom_orders）：

```sql
-- up.sql
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
```

**commit 规范**：`feat(db): 定制订单 5 张表 migration`

### Task 2: 5 entity + 5 DTO

**步骤**：
- 在 `backend/src/models/` 创建 5 个 entity 文件
- 在 `backend/src/dto/` 创建 5 个 DTO 文件
- 更新 `mod.rs` 导出

**commit 规范**：`feat(model): 定制订单 5 entity + 5 DTO`

### Task 3: 路由 + CRUD service

**步骤**：
- 创建 `backend/src/services/custom_order_crud_service.rs`（CRUD 业务）
- 创建 `backend/src/routes/custom_order.rs`（16 路由注册）
- 在 `main.rs` 注册路由
- 验证 `cargo check --lib`

**commit 规范**：`feat(service): 定制订单 CRUD service + 16 路由`

### Task 4: 状态机定义

**步骤**：
- 创建 `backend/src/utils/process_state_machine.rs`
- 定义 5 阶段状态枚举 + 转换函数
- 单元测试状态转换矩阵

**commit 规范**：`feat(util): 工艺流程状态机 + 单元测试`

### Task 5: 工艺推进 service

**步骤**：
- 创建 `backend/src/services/custom_order_process_service.rs`
- 实现节点推进 + 时间戳记录 + 日志
- 与状态机集成

**commit 规范**：`feat(service): 工艺流程推进 service`

---

## Week 2：核心（5 个 Task）

### Task 6: 质检 service

**步骤**：
- 创建 `backend/src/services/custom_order_quality_service.rs`
- 实现异常上报 + 严重度校验
- GB/T 26377 颜色标准规则
- ISO 105 色牢度规则

**commit 规范**：`feat(service): 质检 service（含 GB/T 26377 + ISO 105 规则）`

### Task 7: 售后 service

**步骤**：
- 创建 `backend/src/services/custom_order_aftersales_service.rs`
- 4 种售后类型（客诉/维修/换货/退款）
- 状态机：opened → processing → resolved/closed/rejected

**commit 规范**：`feat(service): 售后 service（4 类型 + 状态机）`

### Task 8: 13 handler 全部实现

**步骤**：
- 创建 `backend/src/handlers/custom_order_handler.rs`
- 实现 13 个 HTTP handler（CRUD + 推进 + 质检 + 售后）
- 在 `handlers/mod.rs` 导出
- 验证 `cargo check --lib`

**commit 规范**：`feat(handler): 定制订单 13 handler 全部实现`

### Task 9: 集成测试（5 个）

**步骤**：
- 创建 5 个测试文件（e2e / state / process / quality / aftersales）
- 覆盖完整生命周期
- 验证（沙箱 5.8GB 限制无法跑 cargo test，CI 验证）

**commit 规范**：`test: 定制订单集成测试 5 个`

### Task 10: 与 P0-1 销售订单联动

**步骤**：
- 添加 `/api/sales-orders/:id/convert-to-custom` 端点
- 从 sales_order 复制 customer/product/quantity 到 custom_order
- 自动生成 5 阶段工艺节点
- 创建 custom_order 草稿

**commit 规范**：`feat(integration): 销售订单转定制订单联动`

---

## Week 3：前端 + 交付（4 个 Task）

### Task 11: 4 前端页面

**步骤**：
- 创建 `frontend/src/views/custom-orders/` 4 个页面
- 复用 V2Table 组件（Wave4 P2-1）
- 添加路由
- 验证前端 `vue-tsc` 通过

**commit 规范**：`feat(frontend): 定制订单 4 页面（list/create/detail/tracking）`

### Task 12: 3 组件

**步骤**：
- 创建 `ProcessFlow.vue`（5 阶段甘特图）
- 创建 `QualityCheck.vue`（异常列表 + 解决）
- 创建 `AfterSalesPanel.vue`（售后工单）

**commit 规范**：`feat(component): 工艺流程 3 组件（ProcessFlow/QualityCheck/AfterSalesPanel）`

### Task 13: E2E 测试 + 用户手册 + API 文档

**步骤**：
- 创建 `frontend/e2e/custom-order.spec.ts`
- 创建 `docs/custom-order-deployment-guide.md`
- 创建 `docs/custom-order-user-manual.md`
- 创建 `docs/custom-order-api.md`

**commit 规范**：`docs: 定制订单 E2E + 用户手册 + API 文档`

### Task 14: TEST 测试版本交付

**步骤**：
- 创建 `dist/test-version-P0-3/` 目录
- 编写 Dockerfile / docker-compose.yml / start.sh
- 编写 README.md + test-scenarios.md
- 更新 MEMORY.md + CHANGELOG.md

**commit 规范**：`docs(dist): P0-3 TEST 测试版本交付`

---

## PR 创建指令

PR 标题：`feat(custom-order): 定制订单全流程跟踪（5 表 + 16 端点 + 4 页面 + E2E + 测试版本）`

PR 内容：
```
## 概要
实现冰溪 ERP 定制订单全流程跟踪模块，填补行业功能评估 0% 实现缺口。

## 变更
- 5 张表：custom_orders / process_nodes / process_logs / quality_issues / after_sales
- 16 API 端点：CRUD + 流程推进 + 质检 + 售后
- 4 前端页面：list / create / detail / tracking
- 3 组件：ProcessFlow / QualityCheck / AfterSalesPanel
- 5 集成测试
- E2E 测试
- 用户手册 + API 文档 + 部署文档
- TEST 测试版本（dist/test-version-P0-3/）

## 业务价值
- 5 阶段工艺流程：纱线采购 → 染整 → 后整理 → 交付 → 售后
- 状态机管理完整生命周期
- 质检规则覆盖 GB/T 26377 + ISO 105
- 售后工单 4 类型
- 与 P0-1 销售订单联动（转定制订单）

## 复用
- P0-1 销售报价单：转订单逻辑
- P0-2 主备隔离：故障注入测试模式
- Wave4 P2-1：V2Table 组件
```

**目标分支**：`test`（**不要**合到 main）

---

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 沙箱 OOM 无法跑 cargo test | 中 | 仅 `cargo check --lib`，CI 验证 |
| 多租户隔离遗漏 | 高 | extract_tenant_id 强制 + 代码审查 |
| 死代码警告 | 中 | 项级 `#[allow(dead_code)]` + TODO |
| 前端类型错误 | 中 | 复用 P0-1 模式 + vue-tsc CI |

## 验收清单

- [ ] 5 张表 migration 通过
- [ ] 16 API 端点全部实现
- [ ] 状态机 5 阶段正常
- [ ] 13 handler 实现
- [ ] 集成测试 5 个
- [ ] 4 前端页面
- [ ] 3 组件
- [ ] E2E 测试
- [ ] 用户手册 + API 文档
- [ ] TEST 测试版本
- [ ] PR 合到 test

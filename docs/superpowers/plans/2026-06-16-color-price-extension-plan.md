# 面料多色号定价扩展模块实施 Plan

> 冰溪 ERP P0-5 行业功能：面料多色号定价扩展
> 计划日期: 2026-06-17
> 关联 spec: `docs/superpowers/specs/2026-06-16-color-price-extension-design.md`
> 目标: 14 Task / 3 周 / 全部合到 test 分支

---

## 1. 任务总览

| 阶段 | Task 范围 | 提交频率 | 关键产出 |
|------|----------|----------|----------|
| **Week 1** | 5 张表 + 5 entity + 5 DTO + 路由 + CRUD service | 5 commit | 后端基础 |
| **Week 2** | 价格计算引擎 + 5 service 完整版 + 13 handler + 集成测试 | 5 commit | 后端完整 |
| **Week 3** | 3 页面 + 2 组件 + API 客户端 + E2E + 文档 + TEST 版本 | 4 commit | 端到端交付 |

**总计 14 Task = 14 commit**

---

## 2. Week 1 任务清单（后端基础）

### Task 1：数据库迁移（5 张表）

**目标**：创建 5 张表的 migration 文件（1 扩展 + 4 新建）

**文件**：
- `backend/migrations/20260618000001_extend_product_color_prices/up.sql` + `down.sql`
- `backend/migrations/20260618000002_create_color_price_history/up.sql` + `down.sql`
- `backend/migrations/20260618000003_create_color_price_tiers/up.sql` + `down.sql`
- `backend/migrations/20260618000004_create_customer_color_prices/up.sql` + `down.sql`
- `backend/migrations/20260618000005_create_seasonal_price_rules/up.sql` + `down.sql`

**内容**：
- 扩展 `product_color_prices`（ALTER TABLE ADD COLUMN）
- 新建 4 张表（含索引 + 唯一约束 + CHECK 约束）
- 5 个 down.sql 用于回滚

**验收**：
- 5 个 up.sql 语法正确
- 5 个 down.sql 可回滚

**Commit**：`feat(db): 面料多色号定价扩展 5 张表 migration（1 扩展 + 4 新建）`

---

### Task 2：5 个 entity

**目标**：定义 5 个 SeaORM entity（覆盖 5 张表）

**文件**：
- `backend/src/models/product_color_price.rs`（扩展字段）
- `backend/src/models/color_price_history.rs`（新建）
- `backend/src/models/color_price_tier.rs`（新建）
- `backend/src/models/customer_color_price.rs`（新建）
- `backend/src/models/seasonal_price_rule.rs`（新建）

**内容**：
- 每个 entity 都有 `#[sea_orm(table_name = "...")]`
- `DeriveEntityModel` 派生
- `Relation` 定义 belongs_to / has_many
- `ActiveModelBehavior` 实现
- 顶层 `#![allow(dead_code)]` + TODO 注释（按项目规范）

**注册**：在 `backend/src/models/mod.rs` 添加 4 个新 mod（product_color_price 已存在）

**验收**：
- 5 entity 文件创建
- `models/mod.rs` 注册

**Commit**：`feat(model): 面料多色号定价扩展 5 entity`

---

### Task 3：5 个 DTO

**目标**：定义 5 个 DTO 文件（含 create / update / query / response 4 类）

**文件**：
- `backend/src/models/color_price_dto.rs`（色号价格 CRUD DTO）
- `backend/src/models/color_price_history_dto.rs`（历史 DTO）
- `backend/src/models/color_price_tier_dto.rs`（阶梯价 DTO）
- `backend/src/models/customer_color_price_dto.rs`（客户专属价 DTO）
- `backend/src/models/seasonal_price_rule_dto.rs`（季节规则 DTO）

**每个 DTO 文件**：
- `CreateXxxDto` — 新建请求
- `UpdateXxxDto` — 更新请求
- `XxxQuery` — 列表过滤（分页 + 多字段）
- `XxxInfo` / `XxxDetail` — 响应

**注册**：在 `backend/src/models/mod.rs` 添加 5 个 mod

**验收**：
- 5 DTO 文件创建
- `models/mod.rs` 注册

**Commit**：`feat(dto): 面料多色号定价扩展 5 DTO`

---

### Task 4：路由文件（16 端点）

**目标**：创建 16 路由文件

**文件**：
- `backend/src/routes/color_price.rs`（16 端点完整定义）

**内容**：
- Router::new() 链式 .route()
- 5 个 CRUD 端点（/、/:id 列表/创建/详情/更新/删除）
- 批量调价 / 审批 / 历史 / 计算 / 阶梯价 / 客户专属价 / 季节规则端点
- 路径前缀：`/api/v1/erp/color-prices`

**注册**：在 `backend/src/routes/mod.rs` 添加 `pub mod color_price;` + `nest("/api/v1/erp/color-prices", color_price::routes())`

**验收**：
- 16 路由文件创建
- `routes/mod.rs` 注册

**Commit**：`feat(route): 面料多色号定价扩展 16 路由`

---

### Task 5：CRUD service（color_price_crud_service）

**目标**：实现色号价格 CRUD service

**文件**：
- `backend/src/services/color_price_crud_service.rs`

**方法**：
- `list(tenant_id, page, page_size, filters)` → 分页查询
- `get_by_id(id, tenant_id)` → 详情
- `create(dto, tenant_id, operated_by)` → 创建
- `update(id, dto, tenant_id)` → 更新
- `delete(id, tenant_id)` → 软删除（is_active = false）

**业务规则**：
- 多租户隔离（WHERE tenant_id = ?）
- 校验 currency / season / customer_level 枚举值
- 校验 min_quantity < max_quantity
- 校验 effective_from ≤ effective_to

**注册**：在 `backend/src/services/mod.rs` 添加 `pub mod color_price_crud_service;`

**验收**：
- service 文件创建
- 5 个方法实现

**Commit**：`feat(service): 面料多色号定价扩展 CRUD service`

---

## 3. Week 2 任务清单（后端完整）

### Task 6：价格计算引擎（核心）

**目标**：实现统一价格计算引擎

**文件**：
- `backend/src/utils/price_calculator.rs`

**结构**：
```rust
pub struct PriceCalcRequest {
    pub product_id: i64,
    pub color_id: i64,
    pub customer_id: Option<i64>,
    pub customer_level: Option<String>,
    pub quantity: Decimal,
    pub season: Option<String>,    // SS / AW / HOLIDAY
    pub product_category_id: Option<i64>,
    pub currency: String,
    pub calc_date: NaiveDate,       // 用于判断有效期
}

pub struct PriceCalcResult {
    pub final_price: Decimal,
    pub base_price: Decimal,
    pub tier_price: Option<Decimal>,
    pub level_price: Option<Decimal>,
    pub season_price: Option<Decimal>,
    pub special_price: Option<Decimal>,
    pub applied_rule: String,       // 最高优先级命中的规则
    pub breakdown: Vec<PriceCalcStep>,
}

pub struct PriceCalcStep {
    pub step: String,
    pub before: Decimal,
    pub after: Decimal,
    pub rule: String,
}

pub fn calculate(req: &PriceCalcRequest, db: &DatabaseConnection) 
    -> Result<PriceCalcResult, CalcError>;
```

**计算流程**（优先级从高到低）：
1. 客户专属价（customer_id + product_id + color_id + 有效期）
2. 季节调价（在 active 规则区间内）
3. 阶梯价（min_quantity ≤ quantity < max_quantity + 客户等级）
4. 客户等级折扣（VIP 95 折 / NORMAL 100%）
5. 基础价（fallback）

**单元测试**：10 个测试用例（`#[cfg(test)]` 模块内）

**验收**：
- `price_calculator.rs` 完整实现
- 10 个单元测试
- `utils/mod.rs` 注册

**Commit**：`feat(util): 面料多色号定价 价格计算引擎（4 档阶梯 + VIP 95 折 + 季节 + 客户专属优先级）`

---

### Task 7：批量调价 service（color_price_batch_service）

**目标**：实现批量调价 + 审批 service

**文件**：
- `backend/src/services/color_price_batch_service.rs`

**方法**：
- `batch_adjust(dto, tenant_id, operated_by)` → 批量调价
- `approve(id, dto, tenant_id, approved_by)` → 审批
- `get_pending_approvals(tenant_id, page, page_size)` → 待审批列表

**业务流程**：
1. 接收批量调价请求（items: [{price_id, adjustment_type, adjustment_value}]）
2. 遍历每条调价
3. 计算 change_percent
4. change_percent > 10% → 标记 PENDING（不更新 base_price）
5. change_percent ≤ 10% → 标记 APPROVED（直接更新 base_price）
6. 写入 color_price_history
7. 返回成功 + 待审批 ID 列表

**审批**：
- 经理调用 approve → 状态改为 APPROVED → 更新 base_price
- 拒绝 → 状态改为 REJECTED → 不更新

**注册**：在 `backend/src/services/mod.rs` 添加 mod

**验收**：
- service 文件创建
- 3 个方法实现

**Commit**：`feat(service): 面料多色号定价 批量调价 service（百分比 / 固定金额 / 阶梯价 + 10% 审批）`

---

### Task 8：价格历史 + 季节规则 + 阶梯价 service（3 个）

**目标**：实现 3 个辅助 service

**文件**：
- `backend/src/services/color_price_history_service.rs`
- `backend/src/services/color_price_seasonal_service.rs`
- `backend/src/services/color_price_tier_service.rs`

**历史 service 方法**：
- `list_by_price(price_id, tenant_id, page, page_size)` → 查询某色号价格的历史
- `list_by_product(product_id, tenant_id, page, page_size, from_date, to_date)` → 按产品 + 时间段
- `record_change(...)` → 内部调用，service 主动写入

**季节规则 service 方法**：
- `list(tenant_id, page, page_size, season, is_active)` → 列表
- `create(dto, tenant_id)` → 创建
- `update(id, dto, tenant_id)` → 更新
- `delete(id, tenant_id)` → 删除
- `get_active_rules_at(date, tenant_id, season, product_category_id)` → 查询某日期生效的规则（计算引擎用）

**阶梯价 service 方法**：
- `list_by_price(price_id, tenant_id)` → 阶梯列表
- `create(dto, tenant_id)` → 新建
- `delete(id, tenant_id)` → 删除
- `get_tier_for_quantity(price_id, quantity, customer_level, tenant_id)` → 查询某数量命中的阶梯价

**注册**：在 `backend/src/services/mod.rs` 添加 3 个 mod

**验收**：
- 3 个 service 文件创建
- 所有方法实现

**Commit**：`feat(service): 面料多色号定价 历史 + 季节 + 阶梯价 service（3 个）`

---

### Task 9：13 个 handler

**目标**：实现 13 个 HTTP handler

**文件**：
- `backend/src/handlers/color_price_handler.rs`（13 个 handler 函数）

**13 个 handler 列表**：

| # | 函数名 | Method | 路径 | 调用的 service |
|---|--------|--------|------|---------------|
| 1 | `list_color_prices` | GET | `/` | crud.list |
| 2 | `create_color_price` | POST | `/` | crud.create |
| 3 | `get_color_price` | GET | `/:id` | crud.get_by_id |
| 4 | `update_color_price` | PUT | `/:id` | crud.update |
| 5 | `delete_color_price` | DELETE | `/:id` | crud.delete |
| 6 | `batch_adjust_color_prices` | POST | `/batch-adjust` | batch.batch_adjust |
| 7 | `approve_color_price` | POST | `/:id/approve` | batch.approve |
| 8 | `get_color_price_history` | GET | `/:id/history` | history.list_by_price |
| 9 | `calculate_color_price` | GET | `/calculate` | calculator.calculate |
| 10 | `list_tiers` / `create_tier` / `delete_tier` | 阶梯价 3 个 | tier.* | |
| 11 | `list_customer_special_prices` / `create_customer_special_price` | 客户专属价 2 个 | special.* | |
| 12 | `list_seasonal_rules` / `create_seasonal_rule` | 季节规则 2 个 | seasonal.* | |

实际 5+1+1+1+1+3+2+2 = 16 个 handler 函数 → 收敛为 13 个有效端点

**注册**：在 `backend/src/handlers/mod.rs` 添加 `pub mod color_price_handler;`

**验收**：
- handler 文件创建（13 个函数）
- 错误类型转换
- 认证 + 租户隔离
- `handlers/mod.rs` 注册

**Commit**：`feat(handler+route): 面料多色号定价 13 handler + 16 路由`

---

### Task 10：5 个集成测试

**目标**：编写 5 个集成测试

**文件**：
- `backend/tests/color_price_crud_test.rs`
- `backend/tests/color_price_calc_test.rs`
- `backend/tests/color_price_batch_test.rs`
- `backend/tests/color_price_history_test.rs`
- `backend/tests/color_price_seasonal_test.rs`

**测试内容**：

**crud_test**（5 测）：
- 创建色号价格（成功）
- 列表分页
- 详情查询
- 更新
- 软删除 + 多租户隔离

**calc_test**（5 测）：
- VIP 95 折应用
- 4 档阶梯价匹配
- 季节调价叠加
- 客户专属价优先级
- 多规则叠加

**batch_test**（4 测）：
- +5% 自动通过
- +15% 需审批
- +25% 需审批
- 审批通过后历史记录

**history_test**（3 测）：
- 写入历史
- 按 price_id 查询
- 按时间区间查询

**seasonal_test**（3 测）：
- 规则创建
- 按日期查询生效规则
- 季节匹配（SS/AW/HOLIDAY）

**验收**：
- 5 个测试文件
- 20 个测试函数

**Commit**：`test: 面料多色号定价 5 集成测试（20 个测试用例）`

---

## 4. Week 3 任务清单（前端 + 文档 + 部署）

### Task 11：前端 API 客户端 + 类型定义

**目标**：封装 16 个 API 端点

**文件**：
- `frontend/src/api/color-price.ts`（~600 行）

**内容**：
- 5 个枚举（CURRENCY / SEASON / ADJUSTMENT_TYPE / APPROVAL_STATUS / CHANGE_TYPE）
- 5 个接口（ColorPriceListItem / ColorPriceDetail / ColorPriceTier / CustomerColorPrice / SeasonalPriceRule）
- 16 个 API 函数（get / post / put / delete）
- 4 个辅助函数（formatPrice / getLevelLabel / getSeasonLabel / getApprovalColor）

**验收**：
- API 客户端文件创建
- 16 个端点封装

**Commit**：`feat(api): 面料多色号定价 API 客户端 16 端点封装`

---

### Task 12：2 个前端组件

**目标**：实现 2 个可复用组件

**文件**：
- `frontend/src/components/PriceHistoryChart.vue`（价格历史折线图）
- `frontend/src/components/BatchAdjustDialog.vue`（批量调价对话框）

**PriceHistoryChart.vue**：
- Props: historyData: ColorPriceHistory[]
- 使用 ECharts 折线图
- X 轴：时间
- Y 轴：价格
- 多币种显示（按 currency 分组）
- 价格变化点标记（涨/跌/平）

**BatchAdjustDialog.vue**：
- Props: visible: boolean, selectedPrices: ColorPriceListItem[]
- 3 种调价模式切换（百分比 / 固定金额 / 阶梯价）
- 调价预览（每条色号变化前后）
- 提交 + 取消

**验收**：
- 2 个组件文件创建

**Commit**：`feat(component): 面料多色号定价 2 组件（PriceHistoryChart / BatchAdjustDialog）`

---

### Task 13：3 个前端页面

**目标**：实现 3 个页面

**文件**：
- `frontend/src/views/color-prices/list.vue`（列表页）
- `frontend/src/views/color-prices/detail.vue`（详情页）
- `frontend/src/views/color-prices/batch-adjust.vue`（批量调价页）

**list.vue**：
- 顶部筛选（产品 / 色号 / 客户等级 / 季节 / 币种 / 状态 / 关键字）
- V2Table 表格
- 分页
- 行操作：详情 / 编辑 / 软删除 / 查看历史

**detail.vue**：
- 基本信息卡片
- 阶梯价列表（增删改）
- 客户专属价列表（只读）
- 价格历史图表
- 调价按钮（单条）

**batch-adjust.vue**：
- 选择色号（多选表格 + 筛选）
- 选择调价模式（百分比 / 固定金额 / 阶梯价）
- 调价预览
- 提交 + 审批状态

**路由**：在 `frontend/src/router/index.ts` 添加 3 个路由

**验收**：
- 3 个页面文件
- 3 个路由配置

**Commit**：`feat(page): 面料多色号定价 3 页面（list / detail / batch-adjust）`

---

### Task 14：E2E 测试 + 文档 + TEST 测试版本

**目标**：端到端测试 + 文档 + 部署

**E2E 文件**：
- `frontend/e2e/color-price.spec.ts`（5 个 E2E 用例）
  1. 登录 → 色号价格列表 → 验证分页
  2. 详情页 → 查看历史图表
  3. 批量调价 +5% → 自动通过
  4. 批量调价 +15% → 待审批
  5. 价格计算 API → VIP 95 折 + 阶梯叠加

**文档文件**：
- `docs/color-price-user-manual.md`（用户手册）
- `docs/color-price-api.md`（API 文档）
- `docs/color-price-deployment-guide.md`（部署指南）

**TEST 测试版本**：
- `dist/test-version-P0-5/Dockerfile`
- `dist/test-version-P0-5/docker-compose.yml`
- `dist/test-version-P0-5/start.sh`
- `dist/test-version-P0-5/stop.sh`
- `dist/test-version-P0-5/.env.example`
- `dist/test-version-P0-5/README.md`
- `dist/test-version-P0-5/test-scenarios.md`（10 个测试场景）
- `dist/test-version-P0-5/config/color-price.toml.example`（价格模块配置）

**验收**：
- 1 个 E2E 文件
- 3 个文档
- 完整 TEST 测试版本

**Commit**：`docs(dist): P0-5 面料多色号定价扩展 TEST 测试版本交付 + 用户手册 + API 文档 + 部署指南 + E2E`

---

## 5. 任务依赖关系

```
Task 1 (5 tables)
  ↓
Task 2 (5 entities) ─→ Task 3 (5 DTOs) ─→ Task 4 (16 routes)
  ↓                       ↓                  ↓
Task 5 (CRUD svc) ─────────────────→ Task 9 (13 handlers)
  ↓
Task 6 (calc engine) ─→ Task 9
  ↓
Task 7 (batch svc) ─→ Task 9
  ↓
Task 8 (history + seasonal + tier svc) ─→ Task 9
  ↓
Task 9 (13 handlers)
  ↓
Task 10 (5 integration tests)
  ↓
[并行] Task 11 (API) + Task 12 (2 components) + Task 13 (3 pages)
  ↓
Task 14 (E2E + docs + TEST version)
```

**关键路径**：Task 1 → Task 2/3/4 → Task 5/6/7/8 → Task 9 → Task 10 → Task 11/12/13 → Task 14

---

## 6. 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| `cargo check` OOM | 编译失败 | 用 rustc 1.94 单独编译 + 限制并行 |
| `cargo test` OOM | 测试失败 | 跳过 `cargo test`，依赖 CI 验证 |
| 已有 `product_color_price` 表结构冲突 | migration 失败 | ALTER TABLE 兼容（ADD COLUMN nullable） |
| 复杂 SQL 查询性能 | 计算接口慢 | 加索引 + 缓存（moka） |
| 调价并发冲突 | 数据不一致 | 事务 + 行锁 |
| 前端 ECharts 体积 | 加载慢 | 按需引入（`echarts/core`） |

---

## 7. 沙箱限制处理

- **5.8GB 内存** → 仅跑 `cargo check --lib`（不跑 `cargo test`）
- **rustc 1.94** → 从 191MB tarball 下载到 `/usr/local/rust-1.94/`
- **跳过 OOM 测试** → 集成测试依赖 CI 验证（`.github/workflows/ci-cd.yml`）
- **CI 失败修复** → 如 CI 反馈，单独修复不阻塞主流程

---

## 8. 验收 checklist

- [ ] 14 Task 全部完成
- [ ] 5 张表创建成功（含 1 扩展）
- [ ] 16 API 端点实现
- [ ] 价格计算引擎 4 档阶梯 + VIP 95 折 + 季节 + 客户专属优先级
- [ ] 批量调价 + 审批（10% 阈值）
- [ ] 13 handler 全部实现
- [ ] 5 集成测试 20 用例
- [ ] 3 前端页面
- [ ] 2 组件
- [ ] E2E 测试 5 用例
- [ ] 用户手册 + API 文档 + 部署指南
- [ ] TEST 测试版本（dist/test-version-P0-5/）
- [ ] PR 合到 test 分支（main 不动）

---

## 9. 提交示例

```bash
git checkout -b trae/solo-agent-P0-5-color-price

git add backend/migrations/2026061800000*
git commit -m "feat(db): 面料多色号定价扩展 5 张表 migration（1 扩展 + 4 新建）"

git add backend/src/models/product_color_price.rs backend/src/models/color_price_*.rs backend/src/models/customer_color_price.rs backend/src/models/seasonal_price_rule.rs backend/src/models/mod.rs
git commit -m "feat(model): 面料多色号定价扩展 5 entity"

git add backend/src/models/color_price_*dto.rs backend/src/models/customer_color_price_dto.rs backend/src/models/seasonal_price_rule_dto.rs
git commit -m "feat(dto): 面料多色号定价扩展 5 DTO"

# ... 共 14 commit

git push origin trae/solo-agent-P0-5-color-price

gh pr create --title "feat(color-price): 面料多色号定价扩展（5 表 + 16 端点 + 3 页面 + E2E + 测试版本）" --base test

gh pr merge <PR_NUMBER> --merge
```

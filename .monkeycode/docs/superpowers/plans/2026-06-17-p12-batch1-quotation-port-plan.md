# 2026-06-17 P12 批 1 销售报价单 Port 计划

> **创建日期**：2026-06-17
> **基线版本**：main @ 495a918
> **源分支**：test @ 9c7c3b7（P10-2 报告 #172）
> **关联路线图**：[2026-06-17-roadmap.md](2026-06-17-roadmap.md) v0.3
> **目标范围**：销售报价单系统（test 独有 P0 高价值资产）

---

## 一、背景

`test` 分支与 `main` 完全分叉（无共同祖先），含 1154 个 test 独有 commit、29+ 个独有数据库迁移文件、7 个独有后端 handler。

经业务价值评估，**销售报价单系统**是 P0 高价值资产（test 独有，main 完全缺失）：

- **业务价值**：⭐⭐⭐⭐⭐（销售模块核心，订单前序）
- **业务复杂度**：报价单主表 + 明细 + 贸易条款（Incoterms 2020）+ 货币 + 税率 + 状态机 + BPM 审批 + 报价转订单
- **端口影响**：新增 `/api/v1/erp/quotations` 路由簇

**关键约束**：
- test 与 main 无共同祖先，**所有 test 代码必须重新实现**（不能直接 copy）
- main 已有 `sales_order_handler`（不同业务），需注意命名区分
- `quotation_pricing_service` 依赖 `product_color_price`（test 独有模型），port 时**stub pricing**（不引入色价依赖）

---

## 二、port 范围（3-4 PR 串行）

### 2.1 PR-1：销售报价单数据层（迁移 + SeaORM 模型）

**目标**：建立 sales_quotation / sales_quotation_item / sales_quotation_term 三张表 + SeaORM 模型

| 任务 | 来源（test 分支） | main 集成点 |
|------|------------------|------------|
| 迁移 `m0020_create_sales_quotations` | [m0020](test:backend/migration/src/m0020_create_sales_quotations.rs) | `backend/migration/src/lib.rs` 注册 |
| 迁移 `m0021_create_sales_quotation_items` | [m0021](test:backend/migration/src/m0021_create_sales_quotation_items.rs) | 同上 |
| 迁移 `m0022_create_sales_quotation_terms` | [m0022](test:backend/migration/src/m0022_create_sales_quotation_terms.rs) | 同上 |
| SQL `20260616000001_create_sales_quotations/up.sql` + `down.sql` | [SQL](test:backend/migrations/20260616000001_create_sales_quotations/) | `backend/migrations/` |
| SQL `20260616000002_create_sales_quotation_items/up.sql` + `down.sql` | [SQL](test:backend/migrations/20260616000002_create_sales_quotation_items/) | 同上 |
| SQL `20260616000003_create_sales_quotation_terms/up.sql` + `down.sql` | [SQL](test:backend/migrations/20260616000003_create_sales_quotation_terms/) | 同上 |
| SeaORM 模型 `sales_quotation.rs` | auto-generate | `backend/src/models/` |
| SeaORM 模型 `sales_quotation_item.rs` | auto-generate | 同上 |
| SeaORM 模型 `sales_quotation_term.rs` | auto-generate | 同上 |

**风险**：
- main 的 `m0019_fix_schema_model_sync` 之后插入 m0020-0022，注意主键 / 索引 / 外键兼容
- `sales_quotation` 与 main 已有 `sales_order` / `sales_fabric_order` 命名区分
- `customer_id` / `sales_user_id` 外键引用 main 的 `customers` / `users` 表（需确认存在）

**CI 风险**：低（纯 schema + 模型）
**预计文件数**：9 个（3 迁移 + 6 SQL + 3 模型）
**预计行数**：~500 行

### 2.2 PR-2：销售报价单 DTO + 基础 Service

**目标**：实现报价单基础 CRUD（不含 product_color_price 依赖）

| 任务 | 来源（test 分支） | main 集成点 |
|------|------------------|------------|
| DTO `quotation_create_dto.rs` | [DTO](test:backend/src/models/quotation_create_dto.rs) | `backend/src/models/` |
| DTO `quotation_response_dto.rs` | [DTO](test:backend/src/models/quotation_response_dto.rs) | 同上 |
| DTO `quotation_update_dto.rs` | [DTO](test:backend/src/models/quotation_update_dto.rs) | 同上 |
| Service `quotation_service.rs`（基础 CRUD）| [Service](test:backend/src/services/quotation_service.rs) | `backend/src/services/` |
| Stub `quotation_pricing_service.rs` | [Service](test:backend/src/services/quotation_pricing_service.rs) | 同上（标 `#[allow(dead_code)]`）|

**Stub 设计**：
```rust
/// 报价单定价服务占位（产品色价 port 之前）
#[allow(dead_code)] // TODO(tech-debt): 待 P13+ port product_color_price 后移除
pub struct QuotationPricingService;

impl QuotationPricingService {
    pub async fn calculate(_ctx: PricingContext) -> Result<Decimal, AppError> {
        // TODO: 集成 product_color_price 后实现阶梯价/折扣
        Ok(Decimal::ZERO)
    }
}
```

**风险**：
- `quotation_create_dto` 中引用的 `product_color_price` 模型不存在（test 独有），需改用 main 已有 `product` 模型或 stub
- 引用 `sea_orm::DatabaseConnection` 改为 main 风格的 `Arc<DatabaseConnection>`

**CI 风险**：低（基础 CRUD）
**预计文件数**：4 个
**预计行数**：~600 行

### 2.3 PR-3：销售报价单 Handler + 路由（基础 8 端点）

**目标**：暴露 `/api/v1/erp/quotations` 路由簇（8 基础端点）

| 端点 | 方法 | 功能 |
|------|------|------|
| `/api/v1/erp/quotations` | GET | 列表查询（分页 + 过滤）|
| `/api/v1/erp/quotations/{id}` | GET | 详情查询 |
| `/api/v1/erp/quotations` | POST | 创建草稿 |
| `/api/v1/erp/quotations/{id}` | PUT | 更新 |
| `/api/v1/erp/quotations/{id}/cancel` | POST | 取消 |
| `/api/v1/erp/quotations/{id}/submit` | POST | 提交审批 |
| `/api/v1/erp/quotations/{id}/approve` | POST | 审批通过 |
| `/api/v1/erp/quotations/{id}/reject` | POST | 审批拒绝 |

| 任务 | 来源（test 分支） | main 集成点 |
|------|------------------|------------|
| Handler `quotation_handler.rs` | [Handler](test:backend/src/handlers/quotation_handler.rs) | `backend/src/handlers/` |
| 注册 `pub mod quotation_handler` | [mod](test:backend/src/handlers/mod.rs) | `backend/src/handlers/mod.rs` |
| 路由 `/api/v1/erp/quotations` | [routes](test:backend/src/routes/) | `backend/src/routes/` |
| `AuthContext` 适配 | main 已有 | 直接使用 |
| `extract_tenant_id` 适配 | main 已有 | 直接使用 |

**风险**：
- test 引用 `crate::middleware::auth_context::AuthContext`，main 路径可能不同
- test handler 引用 `crate::models::product_color_price` 需 stub
- 路由路径需与 main 现有路由风格一致（main 用 `/api/v1/erp/{module}`）

**CI 风险**：中（handler 签名 + 路由集成）
**预计文件数**：3 个
**预计行数**：~700 行

### 2.4 PR-4（可选）：审批流 + 报价转订单 + 集成测试

**目标**：完整业务流（create → submit → approve → convert_to_order）

| 任务 | 来源（test 分支） | main 集成点 |
|------|------------------|------------|
| Service `quotation_approval_service.rs` | [Service](test:backend/src/services/quotation_approval_service.rs) | `backend/src/services/` |
| Service `quotation_convert_service.rs` | [Service](test:backend/src/services/quotation_convert_service.rs) | 同上 |
| 端点 `/api/v1/erp/quotations/{id}/convert` | test handler 末段 | `quotation_handler.rs` |
| 端点 `/api/v1/erp/quotations/expiring` | test handler 末段 | 同上 |
| 集成测试 `tests/quotation_e2e.rs` | 新建 | `backend/tests/` |

**风险**：
- `quotation_convert_service` 依赖 main 已有的 `sales_order` 模型（需验证字段映射）
- 集成测试需要 PostgreSQL 环境（CI 已有）

**CI 风险**：中（依赖 main 已有 sales_order）
**预计文件数**：3 个
**预计行数**：~800 行

---

## 三、影响文件汇总

| 类型 | 文件数 | 行数估计 |
|------|-------|---------|
| 迁移文件（.rs + .sql）| 9 | ~500 |
| SeaORM 模型 | 3 | ~400 |
| DTO | 3 | ~300 |
| Service | 2-4 | ~1200 |
| Handler | 1 | ~700 |
| 路由 | 2 | ~50 |
| 测试 | 1 | ~300 |
| **总计** | **21-23** | **~3450** |

**新增 API 端点**：8-12 个（PR-3 + PR-4）

---

## 四、执行策略

### 4.1 派发

- **1 个独立子代理**负责整个 P0 port（4 PR 串行）
- 子代理任务：按照本 plan 顺序执行 PR-1 → PR-2 → PR-3 → PR-4
- 每个 PR 完成后子代理输出：feature 分支 + commit + PR

### 4.2 与 P12 批 1 整合

| 顺序 | 子代理 | 工作量 | 串行/并行 |
|------|--------|-------|----------|
| 1 | 子代理 A：P0 port 销售报价单 | 4 PR 串行 | 串行（强依赖）|
| 2 | 子代理 B：P2-1 el-table-v2 | 4 PR 串行 | 可与 A 并行 |
| 3 | 子代理 C：B-type-check | 1 PR | 可与 A/B 并行 |
| 4 | 子代理 D：P2-2 性能优化 | 1 PR | 可与 A/B/C 并行 |

**总 PR 数**：4 + 4 + 1 + 1 = 10 PR
**预计总工作量**：4-6 周（视子代理并行度）

### 4.3 风险缓解

- **schema 兼容性**：PR-1 完成后立即跑 `sea-orm-cli generate entity` 验证模型
- **stub 依赖**：PR-2 的 stub 标 `#[allow(dead_code)]` + TODO(tech-debt)，与项目死代码处理规范一致
- **CI 早期反馈**：PR-1 完成后立即验证 CI，发现 schema 问题立即修复
- **集成测试**：PR-4 端到端测试覆盖完整业务流，CI fail-fast

---

## 五、验收

- [ ] PR-1：`cargo build` 通过 + 3 张表创建成功
- [ ] PR-2：基础 CRUD 单元测试通过
- [ ] PR-3：8 端点 Postman/curl 验证
- [ ] PR-4：create → submit → approve → convert 完整流程 e2e 测试通过
- [ ] MEMORY.md / CHANGELOG.md 同步更新
- [ ] 端口冲突：确认 `/api/v1/erp/quotations` 不与现有路由冲突

---

## 六、关联文档

- [2026-06-17-roadmap.md v0.3](2026-06-17-roadmap.md) — 综合路线图
- [2026-06-16-wave4-p2-1-plan.md](2026-06-16-wave4-p2-1-plan.md) — P2-1 计划
- test 分支源文件 — 见各 PR "来源（test 分支）" 链接
- MEMORY.md / CHANGELOG.md — 同步记录

---

**当前版本**：v0.1（2026-06-17 创建）
**下一步**：用户确认后 P12 批 1 启动时执行

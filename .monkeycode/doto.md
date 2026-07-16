# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单）。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近整理：2026-07-16（V15 修复阶段 Batch 433-441 完成，12/104 P0 已修复；剩余 92 P0 + 257 P1 + 248 P2 + 123 P3；重新规划为小粒度批次，每批 3-5 文件）。

---

## 一、当前状态：V15 修复阶段进行中（自动化修复流程·重新规划）

### 1.0 重新规划执行计划（2026-07-16）

> **卡死原因**：P0-S01 service 注入涉及 60+ 文件，单批太大导致上下文/CI 卡死。
> **新策略**：每批严格 3-5 文件、1-2 个 P0 任务，按"快速胜利优先"排序。
> **再次拆分**：Batch 440 进一步拆分为 440a/440b/440c 三个微批次，避免单批过大卡死。

| 批次 | P0 任务 | 文件数 | 执行顺序 | 状态 |
|------|---------|--------|----------|------|
| 437 | P0-S18 dye_recipe_master 角色创建 | 2 | 1 | ✅ 已完成 |
| 438 | P0-S07 权限缓存不失效 | 3 | 2 | ✅ 已完成 |
| 439 | P0-S05 SoD 职责分离互斥 | 3 | 3 | ✅ 已完成 |
| 440a | P0-S06 权限变更审计-基础设施 | 4 | 4 | ✅ 已完成 |
| 440b | P0-S06 权限变更审计-role_permission_service | 1 | 5 | ✅ 已完成 |
| 440c | P0-S06 权限变更审计-user_service | 2 | 6 | ✅ 已完成 |
| 441 | P0-S10 method_to_action 升级 | 2 | 7 | ✅ 已完成 |
| 442-444 | P0-S09 打印导出 AuthContext 补齐 | 13 | 8-10 | 🔄 下一个 |
| 445-446 | P0-S11 导出审计日志补齐 | 10 | 11-12 | ⏳ |
| 447 | P0-S01 service 注入-销售域 | 5 | 13 | ⏳ |
| 448 | P0-S01 service 注入-采购域 | 4 | 14 | ⏳ |
| 449 | P0-S01 service 注入-生产域 | 5 | 15 | ⏳ |
| 450 | P0-S01 service 注入-CRM 域 | 4 | 16 | ⏳ |
| 451 | P0-S01 service 注入-财务域 | 5 | 17 | ⏳ |
| 452 | P0-S01 service 注入-库存域 | 4 | 18 | ⏳ |
| 453-456 | P0-S02 IDOR 防护-handler 层 | 20 | 19-22 | ⏳ |
| 457+ | P0-S08/S23/S24/S25/S26/S27/S28 | - | 23+ | ⏳ |
| 470+ | P0-F01~F21 面料行业核心特性 | - | 30+ | ⏳ |
| 490+ | P0-T01~T08 测试体系 | - | 40+ | ⏳ |
| 500+ | P0-D01~D17 部署运维 | - | 50+ | ⏳ |
| 520+ | P0-B01~B30 财务业务流程 | - | 60+ | ⏳ |


### 1.1 V15 审计完成进度（2026-07-16 全部完成）

| 批次 | 类别 | 维度数 | P0 | P1 | P2 | P3 | 小计 | 状态 |
|------|------|--------|----|----|----|----|------|------|
| 01-04 | 类一~类四 | 38 | 8 | 21 | 14 | 9 | 52 | ✅ 完成 |
| 05-08 | 类五~类八 | 27 | 16 | 49 | 37 | 11 | 113 | ✅ 完成 |
| 09-10 | 类九~类十二 | 21 | 22 | 16 | 20 | 9 | 67 | ✅ 完成 |
| 11-12 | 类十三~类十四 | 22 | 35 | 28 | 25 | 5 | 93 | ✅ 完成 |
| 13-14 | 类十五~类十六 | 25 | 0 | 25 | 33 | 25 | 83 | ✅ 完成 |
| 15-16 | 类十七~类十九 | 21 | 13 | 52 | 39 | 11 | 115 | ✅ 完成 |
| 17-18 | 类二十~类二十二 | 19 | 5 | 28 | 25 | 12 | 70 | ✅ 完成 |
| 19-21 | 类二十三~类二十五 | 30 | 5 | 38 | 55 | 41 | 139 | ✅ 完成 |
| **合计** | **25 大类** | **195** | **104** | **257** | **248** | **123** | **732** | ✅ **审计全部完成** |

### 1.2 核心交付物

- **审计汇总报告**：[v15-summary-2026-07-16.md](file:///workspace/.monkeycode/docs/audits/v15/v15-summary-2026-07-16.md)
- **21 批审计报告**：[batch-01 ~ batch-21](file:///workspace/.monkeycode/docs/audits/v15/)
- **审计计划**：[v15-review-plan-2026-07-15.md](file:///workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md)

---

## 二、V15 修复任务规划（732 项）

> **执行策略**：规则 13+14+15 联动，CI 全绿后自动进入下一批，无需用户确认；所有警告视为错误必须真实修复；修复前必须调研现有实现禁止重复造轮子（§10.0.1 复用现有功能原则）。
> **批次节奏**：每批 5-6 文件，遵循规则 13 连续执行流程；每 30 批触发 E2E（规则 5）；每 15 批整理记忆（规则 10）。
> **修复路线图**：阶段一 P0（104）→ 阶段二 P1（257）→ 阶段三 P2（248）→ 阶段四 P3（123）。

### 2.1 阶段一：P0 阻塞级修复（104 项，分 5 个优先级）

#### 优先级 1：安全与权限（约 28 P0，最高优先级）

##### P0-S01 行级数据权限完全未实现（类十二+类十四+类十八）🔄 进行中（Batch 436 基础设施完成）

- **来源**：batch-10 P0-10-6/7 + batch-12 P0-12-13/14 + batch-15 P0-15-10
- **证据**：
  - [permission.rs](file:///workspace/backend/src/middleware/permission.rs) 缺 `apply_data_scope` 函数
  - [customer_service.rs](file:///workspace/backend/src/services/customer_service.rs) 查询无数据范围过滤
  - CRM 所有查询无 owner_id 过滤
- **业务影响**：销售员可查询所有客户订单、CRM 数据无归属过滤、所有 `/:id` 路由未校验资源归属
- **修复方案**：
  1. ✅ 实现 `apply_data_scope(query, user_id, scope)` 工具函数（all/department/self 三级）——Batch 436
  2. ⏳ 在 customer/supplier/sales_order/purchase_order/crm_* 等 60+ service 查询入口注入——Batch 437
  3. ✅ 新增 `data_scope` 字段到 role 表（all/dept/self）——Batch 436
  4. ⏳ 在所有 `/:id` handler 增加 `check_resource_owner` 校验（IDOR 防护）——Batch 438
  5. ⏳ PostgreSQL 行级安全 RLS 策略（可选，作为代码层兜底后的二次防护）——后续
- **关联文件**（15+）：permission.rs / customer_service.rs / supplier_service.rs / sales_order_service.rs / purchase_order_service.rs / crm_lead_service.rs / crm_opportunity_service.rs / role_service.rs / 各 handler / 前端 customer/supplier/crm 视图
- **预估批次**：5-6 批（约 30 文件）
- **完成情况（Batch 436）**：
  - migration m0051：role 表新增 data_scope 字段（all/dept/self），33 个角色配置默认值
  - data_scope.rs 工具模块：DataScope 枚举 + DataScopeContext + build_data_scope_condition + apply_data_scope + check_resource_owner + 15 单元测试
  - AuthContext 新增 department_id 和 data_scope 字段 + to_data_scope_context 方法
  - auth 中间件从 DB 加载 role.data_scope 和 user.department_id 注入 AuthContext
  - init_service.rs / role_service.rs / role_permission_service.rs 所有角色创建支持 data_scope

##### P0-S02 IDOR 越权访问防护未实现（类十二）

- **来源**：batch-10 P0-10-8
- **证据**：所有 `/:id` 路由仅校验登录状态，未校验资源归属
- **修复方案**：在 get/update/delete handler 增加 `require_resource_owner(resource_id, user_id)` 中间件
- **关联文件**：所有 handler 文件（140+）

##### P0-S03 `*:*` 超级权限注入修复（类十四）✅ 已完成（Batch 433 / PR #611）

- **来源**：batch-12 P0-12-1/3/10/11/12/20
- **修复**：auth_handler.rs 将 `is_system` 判断改为 `code == ADMIN_ROLE_CODE`，仅 admin 注入超级通配权限；init_service.rs 新增 `create_default_role_permissions` 为 manager/operator 插入基本 role_permission 记录
- **状态**：✅ 已合并到 main（c3f3cc7c）

##### P0-S04 14 类业务角色补齐（类十四）✅ 已完成（Batch 434 / PR #612）

- **来源**：batch-12 P0-12-2/4/5
- **修复**：补齐 31 类业务角色覆盖面料行业全业务场景（管理/销售/采购/库存/生产/质量/财务/CRM/物流/人力/安全/IT），为全部角色配置基本 role_permission 权限记录
- **状态**：✅ 已合并到 main（15652b2a）

##### P0-S05 SoD 职责分离互斥未实现（类十四）

- **来源**：batch-12 P0-12-6/7/8
- **修复方案**：
  1. 新增 `role_conflict` 表记录互斥角色对
  2. 新增 `check_role_conflict(user_id, new_role_id)` 函数
  3. 财务三权分立：制单/审核/支付互斥
  4. 用户分配角色时校验互斥
- **关联文件**：role_service.rs / user_service.rs / schema migrations

##### P0-S06 权限变更审计未实现（类十四）

- **来源**：batch-12 P0-12-18/19
- **修复方案**：新增 `permission_change_audit` 表，记录角色权限变更、用户角色变更；每周 cron 合规审查
- **关联文件**：role_service.rs / user_service.rs / audit_service.rs / schema migrations

##### P0-S07 权限缓存不失效（类十四）

- **来源**：batch-12 P0-12-15/16
- **证据**：权限变更/用户禁用后缓存仍命中（5min TTL）
- **修复方案**：
  1. 实现 `invalidate_user_permission_cache(user_id)` 函数
  2. Redis pub/sub 热更新通知
  3. 用户禁用时主动失效缓存
- **关联文件**：permission.rs / user_service.rs / role_service.rs

##### P0-S08 CRM 数据权限完全缺失（类十八）

- **来源**：batch-15 P0-15-10
- **证据**：CRM 所有表（lead/opportunity/customer_pool）无 owner_id 过滤
- **修复方案**：
  1. lead/opportunity 表新增 owner_id 字段
  2. 公海/私海规则：private_pool 仅 owner 可见，public_pool 所有可见
  3. 客户转移需审批 + 审计
- **关联文件**：crm_lead_service.rs / crm_opportunity_service.rs / customer_pool_service.rs / 各 handler

##### P0-S09 打印导出端点 AuthContext 补齐（类十三）

- **来源**：batch-11 P0-11-1/2/3
- **证据**：
  - [dye_recipe_handler.rs](file:///workspace/backend/src/handlers/dye_recipe_handler.rs) export 接口缺 AuthContext
  - [dye_batch_handler.rs](file:///workspace/backend/src/handlers/dye_batch_handler.rs) export 接口缺 AuthContext
- **修复方案**：所有 print/export handler 强制注入 AuthContext，校验 `*:print` / `*:export` 权限
- **关联文件**：13 个 print/export handler（dye_recipe/dye_batch/quotation/sales_order/purchase_order/customer/supplier/product/inventory/report/finance/crm/quality）

##### P0-S10 method_to_action 不识别 print/export（类十三）

- **来源**：batch-11 P0-11-4/5/6
- **证据**：[audit_middleware.rs](file:///workspace/backend/src/middleware/audit_middleware.rs) `classify_operation` 仅识别 GET/POST/PUT/DELETE
- **修复方案**：
  1. OperationType 新增 Print/Download 变体
  2. `method_to_action` 识别 `?action=print` / `?action=export` 查询参数
  3. 权限码表新增 `*:print` / `*:export` 共 38 个权限码
- **关联文件**：audit_middleware.rs / models/audit_log.rs / init_service.rs（权限码注册）

##### P0-S11 10 个导出 handler 缺审计日志（类十三）

- **来源**：batch-11 P0-11-7
- **修复方案**：为 10 个导出 handler 增加审计日志写入（用户/时间/资源/条件/导出条数/文件大小）
- **关联文件**：10 个导出 handler + audit_service.rs

##### P0-S12 前端本地导出完全无审计（类十三）

- **来源**：batch-11 P0-11-10/11
- **证据**：25+ 前端页面使用 `exportToExcel` 本地生成，绕过后端 API
- **修复方案**：
  1. 前端 `exportToExcel` 工具改为调用后端 `/api/{resource}/export` 接口
  2. 后端返回 xlsx 文件流（含水印）
  3. 25+ 页面改造（customer/supplier/product/inventory/sales_order/purchase_order/finance/crm/report 等）
- **关联文件**：[frontend/src/utils/export.ts](file:///workspace/frontend/src/utils/export.ts) + 25+ 视图文件

##### P0-S13 审计日志导出"假按钮"陷阱（类十三）

- **来源**：batch-11 P0-11-12
- **证据**：[audit_log_view.vue](file:///workspace/frontend/src/views/system/audit_log_view.vue) 导出按钮走本地 exportToExcel
- **修复方案**：审计日志导出必须走后端 API，且导出动作本身需二次审计（写入 audit_log_export_log 表）
- **关联文件**：audit_log_view.vue / audit_log_handler.rs

##### P0-S14 二级审批机制完全缺失（类十三）

- **来源**：batch-11 P0-11-8/9
- **修复方案**：
  1. 新增 `export_approval_request` 表（申请人/审批人/资源/条件/状态/有效期）
  2. 敏感数据导出（财务报表/客户清单/染色配方/价格清单）需二级审批
  3. 审批通过后生成临时 token（5min 有效）才能下载
- **关联文件**：export_approval_service.rs / export_approval_handler.rs / schema migrations

##### P0-S15 导出文件无水印（类十三）

- **来源**：batch-11 P0-11-13
- **修复方案**：xlsx 文件加水印（操作员+IP+时间戳），PDF 加水印（中文字体支持）
- **关联文件**：[xlsx_export.rs](file:///workspace/backend/src/utils/xlsx_export.rs) / pdf_export.rs

##### P0-S16 导出无条数上限（类十三）

- **来源**：batch-11 P0-11-14
- **修复方案**：9 类资源（customer/supplier/product/inventory/order/finance/crm/report/audit_log）导出上限 10000 条，超过需分页或拒绝
- **关联文件**：9 个导出 handler + export_approval_service.rs

##### P0-S17 打印 HTML 是占位假数据（类十三）

- **来源**：batch-11 P0-11-15
- **证据**：[print_handler.rs](file:///workspace/backend/src/handlers/print_handler.rs) 返回硬编码 stub HTML
- **修复方案**：print_handler 根据资源类型查询真实数据，使用 handlebars 模板渲染
- **关联文件**：print_handler.rs / print_templates/ 目录

##### P0-S18 dye_recipe_master 角色未创建（类十三）

- **来源**：batch-11 P0-11-10
- **修复方案**：新增 dye_recipe_master 角色，配置染色配方 read/write/print/export 权限
- **关联文件**：init_service.rs / role_service.rs

##### P0-S19 14 端点审计不达标（类十三）

- **来源**：batch-11 P0-11-12
- **证据**：15 端点 × 8 字段审计矩阵，仅 23% 达标
- **修复方案**：补齐 14 端点的 8 个审计字段（user_id/ip/user_agent/resource_id/action/condition/result/duration）
- **关联文件**：14 个 handler + audit_middleware.rs

##### P0-S20 权限资源缺口（类十四）✅ 已完成（Batch 435 / PR #613）

- **来源**：batch-12 P0-12-9
- **证据**：当前 11 类权限资源，实际 60+ 类业务模块
- **修复方案**：补齐 49+ 类权限资源（fabric/dye_batch/dye_recipe/chemical/energy/wage/outsourcing/energy/quality_issue/8d/custom_order/after_sales/logistics/incoterms/oa/bi/dashboard/notification/email/business_trace 等），每个资源配 11 个操作权限码
- **关联文件**：init_service.rs / permission.rs / path_utils.rs
- **完成情况**：新增 PERMISSION_RESOURCES 常量（60+ 类资源）+ PERMISSION_ACTIONS 常量（11 个操作权限码）+ extract_action_from_path 函数（从路径提取 print/export/approve 等 11 个动作）

##### P0-S21 模块前缀白名单不足（类十四）✅ 已完成（Batch 435 / PR #613）

- **来源**：batch-12 P0-12-10
- **修复方案**：扩展模块前缀白名单至 60+ 类，未在白名单的路由直接拒绝
- **关联文件**：path_utils.rs / permission.rs
- **完成情况**：清理 15+ 脏数据（purchases→purchase 等）+ 新增 28 个模块前缀（production/auth/quotations 等）+ 新增 is_known_resource_segment 函数 + permission_middleware 白名单校验

##### P0-S22 权限矩阵未实现（类十四）✅ 已完成（Batch 435 / PR #613）

- **来源**：batch-12 P0-12-11/12/13
- **修复方案**：实现 14 角色 × 60+ 资源 × 11 操作的权限矩阵，写入 init_service.rs 初始化
- **关联文件**：init_service.rs / role_service.rs
- **完成情况**：create_default_role_permissions 扩展为 33 个角色 × 60+ 资源的完整权限矩阵（管理层全资源 read / 经理本域 * / 执行角色本域 read+create+update）

##### P0-S23 用户角色无互斥校验（类十四）

- **来源**：batch-12 P0-12-17
- **修复方案**：用户分配多角色时校验互斥规则（如 sales_manager 与 purchase_manager 不可同持）
- **关联文件**：user_service.rs / role_service.rs

##### P0-S24 前后端权限边界一致性（类十四）

- **来源**：batch-12 P0-12-14
- **修复方案**：前端 v-permission 指令与后端权限码完全对齐，4 项不一致场景修复
- **关联文件**：[v-permission.ts](file:///workspace/frontend/src/directives/v-permission.ts) + 所有视图组件

##### P0-S25 行级数据权限 RLS（类十二）

- **来源**：batch-10 P0-10-7
- **修复方案**：PostgreSQL RLS 策略，按 user_id / department_id 过滤敏感表（customer/supplier/sales_order/crm_*）
- **关联文件**：schema migrations / database/rls.sql

##### P0-S26 AI 端点权限码未注册（类十六）

- **来源**：batch-14 P1（升级为 P0）
- **修复方案**：14 个 AI 端点权限码注册（ai:recipe_opt:read/write、ai:quality_pred:read、ai:reorder:read 等）
- **关联文件**：init_service.rs / ai_route.rs

##### P0-S27 AI 推理数据范围未过滤（类十六）

- **来源**：batch-14 P1（升级为 P0）
- **修复方案**：AI 推理查询注入 apply_data_scope，销售员调 AI 推荐时仅看自己的客户
- **关联文件**：ai_*.rs services

##### P0-S28 前端 v-permission 覆盖率仅 4%（类二十四）

- **来源**：batch-20 P1（升级为 P0）
- **修复方案**：85+ 视图组件全部接入 v-permission 指令，按钮级控制
- **关联文件**：85+ .vue 文件

---

#### 优先级 2：面料行业核心特性（约 21 P0）

##### P0-F01 dye_batch 表缺少 dye_lot_no 字段（类四）

- **来源**：batch-04 P0-04-1/2
- **证据**：[dye_batch.rs](file:///workspace/backend/src/models/dye_batch.rs) 第 12-26 行无 dye_lot_no 字段
- **业务影响**：四层级联断裂、成本归集不完整、缸号追溯失效
- **修复方案**：
  1. migration 新增 dye_batch.dye_lot_no 字段（VARCHAR(50) NOT NULL）
  2. 更新 dye_batch model + DTO
  3. [dye_batch_cost_bridge_service.rs:152-153](file:///workspace/backend/src/services/dye_batch_cost_bridge_service.rs) 补全 dye_lot_no 关联
  4. 历史数据回填（默认 'DEFAULT'）
- **关联文件**（9+）：dye_batch.rs / dye_batch_service.rs / dye_batch_cost_bridge_service.rs / dye_batch_handler.rs / migrations / 前端 dye_batch_view.vue

##### P0-F02 v14 §2.2.2 关键业务约束 UNIQUE 未实现（类一）

- **来源**：batch-01 P0-01-01
- **证据**：[up.sql:4](file:///workspace/backend/migrations/20260518000002_add_dye_tables/up.sql) 仅 batch_no 单字段全局 UNIQUE
- **修复方案**：新增 4 项联合唯一索引：
  1. `UNIQUE(fabric_id, color_id, dye_lot_no, batch_no)` 在 dye_batch 表
  2. `UNIQUE(warehouse_id, product_id, color_id, batch_no, dye_lot_no)` 在 inventory_stock 表
  3. `UNIQUE(order_id, item_id, batch_no)` 在 sales_delivery_item 表
  4. `UNIQUE(receipt_id, item_id, batch_no)` 在 purchase_receipt_item 表
- **关联文件**（9+）：migrations + fabric_inspection_service / inventory_stock_service / purchase_receipt_service / so/delivery / sales_return_service / purchase_return_service / inventory_count_service

##### P0-F03 色卡发放专项——旧"借出/归还"模式完全存在（类九）

- **来源**：batch-09 P0-09-1
- **修复方案**：删除 fabric_color_card_lend_return 表的 lend_return 语义，重命名为 `fabric_color_card_lend_return_legacy`；新表走"发放"模式
- **关联文件**：schema migrations / color_card_service.rs

##### P0-F04 色卡发放——新"发放"模式后端文件完全缺失（类九）

- **来源**：batch-09 P0-09-2
- **修复方案**：创建 4 个后端新文件：
  1. `color_card_issue_service.rs`（发放业务逻辑）
  2. `color_card_issue_handler.rs`（HTTP handler）
  3. `color_card_issue.rs`（SeaORM model）
  4. migration（color_card_issue 表 DDL）
- **关联文件**：backend/src/services/color_card_issue_service.rs / backend/src/handlers/color_card_issue_handler.rs / backend/src/models/color_card_issue.rs / migrations/

##### P0-F05 色卡发放——旧路由未删除，新路由未注册（类九）

- **来源**：batch-09 P0-09-3
- **修复方案**：删除 `/color-cards/lend-return` 路由组，新增 `/color-cards/issues` 路由组（POST /issues, GET /issues, GET /issues/:id, POST /issues/:id/return）
- **关联文件**：[color_card_routes.rs](file:///workspace/backend/src/routes/color_card_routes.rs)

##### P0-F06 色卡发放——旧表未重命名为 legacy，新表未创建（类九）

- **来源**：batch-09 P0-09-4
- **修复方案**：
  1. RENAME TABLE `fabric_color_card_lend_return` TO `fabric_color_card_lend_return_legacy`
  2. CREATE TABLE `color_card_issue`（id, card_id, customer_id, issue_qty, issue_date, expected_return_date, actual_return_date, status, issued_by, returned_by, remark）
- **关联文件**：migrations

##### P0-F07 色卡发放——前端仍是借还模式（类九）

- **来源**：batch-09 P0-09-5
- **修复方案**：删除 ColorCardLendReturn.vue，创建 ColorCardIssue.vue（发放视图）
- **关联文件**：[frontend/src/views/fabric/](file:///workspace/frontend/src/views/fabric/)

##### P0-F08 色卡发放——发放前 5 道闸门校验未实现（类九）

- **来源**：batch-09 P0-09-6
- **修复方案**：在 issue handler 实现闸门校验：
  1. 卡片状态 = active
  2. 库存数量 >= 发放数量
  3. 客户信用额度未超
  4. 客户无未归还超期记录
  5. 客户白名单校验
- **关联文件**：color_card_issue_service.rs

##### P0-F09 色卡发放——新状态流转校验未实现（类九）

- **来源**：batch-09 P0-09-7
- **修复方案**：状态机 `issued → returned / lost / scrapped`，校验流转合法性
- **关联文件**：color_card_issue_service.rs

##### P0-F10 色卡发放——库存联动未实现（类九）

- **来源**：batch-09 P0-09-8
- **修复方案**：发放时 inventory_stock 扣减，归还时增加，丢失时调拨到报废仓
- **关联文件**：color_card_issue_service.rs / inventory_stock_service.rs

##### P0-F11 色卡发放——前端文件结构完全未创建（类九）

- **来源**：batch-09 P0-09-9
- **修复方案**：创建 7 个前端新文件：
  1. `ColorCardIssue.vue`（发放列表视图）
  2. `ColorCardIssueForm.vue`（发放表单）
  3. `ColorCardIssueDetail.vue`（发放详情）
  4. `useColorCardIssue.ts`（composable）
  5. `colorCardIssue.ts`（API 模块）
  6. `colorCardIssue.ts`（类型定义）
  7. `colorCardIssue.ts`（store）
- **关联文件**：frontend/src/views/fabric/ + composables/ + api/ + types/ + stores/

##### P0-F12 色卡发放——前端类型/API/视图组件未实现（类九）

- **来源**：batch-09 P0-09-10/11/12
- **修复方案**：实现 TypeScript 类型定义、API 调用、Vue 视图组件
- **关联文件**：见 P0-F11

##### P0-F13 色卡发放——数据迁移策略未实现（类九）

- **来源**：batch-09 P0-09-13
- **修复方案**：编写 SQL 迁移脚本，将 legacy 表的 lend 记录转为 issue 记录（status='returned' 或 'lost'）
- **关联文件**：migrations/color_card_migrate_legacy.sql

##### P0-F14 色卡发放——代码层旧文件处理未实现（类九）

- **来源**：batch-09 P0-09-14
- **修复方案**：删除旧的 color_card_lend_return_service.rs / handler / model，改为 legacy_ 前缀保留只读
- **关联文件**：backend/src/services/ + handlers/ + models/

##### P0-F15 大货批色——bulk_color_approval 表完全不存在（类十一）

- **来源**：batch-10 P0-10-1
- **修复方案**：CREATE TABLE `bulk_color_approval`（id, sales_order_id, dye_batch_id, customer_id, sample_type=cut_sample/lab_sample, approval_status=pending/approved/rejected/rework, approver_id, approval_date, reject_reason, attachment_url, remark）
- **关联文件**：migrations + bulk_color_approval.rs model

##### P0-F16 大货批色——剪大货样业务规则未实现（类十一）

- **来源**：batch-10 P0-10-2
- **修复方案**：实现剪大货样 handler，从 dye_batch 剪取样品，扣减库存
- **关联文件**：bulk_color_approval_service.rs / bulk_color_approval_handler.rs

##### P0-F17 大货批色——客户批色确认流程未实现（类十一）

- **来源**：batch-10 P0-10-3
- **修复方案**：客户通过链接/小程序确认批色，更新 approval_status
- **关联文件**：bulk_color_approval_handler.rs / 前端 customer_portal

##### P0-F18 大货批色——返工/降级/报废未实现（类十一）

- **来源**：batch-10 P0-10-4
- **修复方案**：批色不通过时触发：返工（走生产订单）/ 降级（等级降 A→B→C）/ 报废（库存转报废仓）
- **关联文件**：bulk_color_approval_service.rs / production_order_service.rs / inventory_stock_service.rs

##### P0-F19 大货批色——ship_order 不校验批色状态（类十一）

- **来源**：batch-10 P0-10-5
- **修复方案**：发货前校验所有 dye_batch 的 bulk_color_approval.status='approved'，否则拒绝发货
- **关联文件**：[ship_order_service.rs](file:///workspace/backend/src/services/ship_order_service.rs)

##### P0-F20 8D 质量管理流程完全缺失（类二十一）

- **来源**：batch-18 P0-18-1
- **证据**：quality_issue 表仅 open/resolved/closed 三态
- **修复方案**：实现 D0~D8 八步流程：
  - D0 准备阶段 / D1 组队 / D2 描述问题 / D3 临时措施 / D4 根因分析 / D5 永久措施 / D6 实施 / D7 预防 / D8 表彰
  - quality_issue 表新增 8D 字段，状态机扩展为 11 态
- **关联文件**：quality_issue_service.rs / quality_issue_handler.rs / schema migrations

##### P0-F21 返工未走生产订单（类二十一）

- **来源**：batch-18 P0-18-2
- **修复方案**：返工必须创建 production_order（type='rework'），关联原 dye_batch，扣减库存
- **关联文件**：[rework_service.rs](file:///workspace/backend/src/services/rework_service.rs) / production_order_service.rs

---

#### 优先级 3：测试体系（约 8 P0）

##### P0-T01 核心 service 零单元测试（类六）

- **来源**：batch-06 P0-06-1
- **证据**：quotation_service.rs / purchase_receipt_service.rs 零单元测试
- **修复方案**：为两个 service 编写完整单元测试（覆盖率 ≥80%），抽取 mock 数据到 fixtures
- **关联文件**：backend/tests/quotation_service_test.rs / purchase_receipt_service_test.rs / tests/fixtures/

##### P0-T02 7 项关键业务路径无集成测试（类六）

- **来源**：batch-06 P0-06-2
- **修复方案**：编写 7 项集成测试：生产订单→染色→质检→入库 / 采购订单→收货→付款 / 销售订单→发货→收款 / 染整全流程 / 化验室打样 / 大货处方 / 库存调拨
- **关联文件**：backend/tests/integration/

##### P0-T03 CI baseline 机制掩盖编译失败（类六）

- **来源**：batch-06 P0-06-3
- **证据**：bi_analysis_test.rs 16 个测试 API 与源码脱节但 CI 通过
- **修复方案**：移除 baseline 机制，所有失败必须真实修复
- **关联文件**：[.github/workflows/ci-cd.yml](file:///workspace/.github/workflows/ci-cd.yml) / backend/tests/bi_analysis_test.rs

##### P0-T04 mockBusinessApi 未移除（类六）

- **来源**：batch-06 P0-06-4
- **证据**：22+ E2E spec 使用 mockBusinessApi 走 mock
- **修复方案**：移除 mockBusinessApi，E2E 全部走真实后端 API
- **关联文件**：[frontend/e2e/fixtures/mockBusinessApi.ts](file:///workspace/frontend/e2e/fixtures/mockBusinessApi.ts) + 22+ spec

##### P0-T05 E2E 通过率 0%（类六）

- **来源**：batch-06 P0-06-5
- **证据**：95 个 E2E 测试 88 个失败
- **修复方案**：逐个修复 E2E 失败用例，目标通过率 ≥90%
- **关联文件**：frontend/e2e/specs/

##### P0-T06 bi_analysis_test.rs 16 个测试 API 脱节（类六）

- **来源**：batch-06 P0-06-6
- **修复方案**：更新 16 个测试用例的 API 调用，与源码对齐
- **关联文件**：backend/tests/bi_analysis_test.rs

##### P0-T07 4 项关键 service 性能基准测试缺失（类六）

- **来源**：batch-06 P0-06-7
- **修复方案**：为 inventory_stock_service / sales_order_service / dye_batch_service / report_service 编写性能基准测试（P95 ≤2s）
- **关联文件**：backend/tests/bench/

##### P0-T08 CI 不集成覆盖率工具（类六）

- **来源**：batch-06 P0-06-8
- **修复方案**：CI 新增 `cargo tarpaulin` 步骤，上传 codecov；前端新增 `vitest --coverage`
- **关联文件**：.github/workflows/ci-cd.yml / codecov.yml

---

#### 优先级 4：部署与运维（约 17 P0）

##### P0-D01 Docker 文件违规（类七）

- **来源**：batch-07 P0-07-1
- **证据**：4 个 Docker 文件违反禁止 Docker 规则
- **修复方案**：删除所有 Docker 文件（Dockerfile / docker-compose.yml / .dockerignore / docker-entrypoint.sh）
- **关联文件**：项目根 / deploy/ 下的 Docker 文件

##### P0-D02 快速部署脚本安装 PostgreSQL 客户端（类七）

- **来源**：batch-07 P0-07-2
- **证据**：[install.sh](file:///workspace/deploy/install.sh) 安装 postgresql-client
- **修复方案**：移除 postgresql-client 安装步骤（数据库连接走远程模式）
- **关联文件**：deploy/install.sh / deploy/deploy.sh

##### P0-D03 5 个 service 全部未接入缓存层（类七）

- **来源**：batch-07 P0-07-3
- **证据**：user/product/customer/supplier/role_service 直接走 DB
- **修复方案**：5 个 service 接入 Redis 缓存（5min TTL + 主动失效）
- **关联文件**：user_service.rs / product_service.rs / customer_service.rs / supplier_service.rs / role_service.rs

##### P0-D04 缓存是内存缓存(moka)非 Redis（类七）

- **来源**：batch-07 P0-07-4
- **修复方案**：将 moka 内存缓存迁移到 Redis（多实例共享 + 持久化）
- **关联文件**：[cache.rs](file:///workspace/backend/src/utils/cache.rs) + 所有使用 moka 的 service

##### P0-D05 useI18n 接入率仅 3.2%（类七）

- **来源**：batch-07 P0-07-5
- **修复方案**：85+ 视图组件全部接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts
- **关联文件**：frontend/src/views/ + locales/

##### P0-D06 aria-label 严重不足（类七）

- **来源**：batch-07 P0-07-6
- **证据**：仅 2 个文件 8 处 aria-label
- **修复方案**：所有交互元素补 aria-label（WCAG 2.1 AA）
- **关联文件**：所有 .vue 文件

##### P0-D07 图片 alt 属性完全缺失（类七）

- **来源**：batch-07 P0-07-7
- **证据**：0 处 alt 属性
- **修复方案**：所有 `<img>` 补 alt 描述
- **关联文件**：所有 .vue 文件

##### P0-D08 130+ 超长函数（类七）

- **来源**：batch-07 P0-07-8
- **证据**：130+ 函数超过 50 行（最长 event_bus.rs:412 start_event_listener 586 行）
- **修复方案**：拆分超长函数为单一职责小函数（每个 ≤50 行）
- **关联文件**：event_bus.rs / ar_service.rs（1972 行）/ business_mode_service.rs / 等 26 个 >1000 行的文件

##### P0-D09 30+ 函数超过 100 行（类二）

- **来源**：batch-02 P0-02-01
- **修复方案**：拆分为 ≤50 行小函数
- **关联文件**：同 P0-D08

##### P0-D10 26 个后端文件超过 1000 行（类二）

- **来源**：batch-02 P0-02-02
- **修复方案**：按职责拆分为多个文件（如 ar_service.rs 拆分为 ar_service / ar_aging_service / ar_collection_service）
- **关联文件**：26 个 >1000 行的文件

##### P0-D11 setup_test_db 在 14 个文件重复定义（类二）

- **来源**：batch-02 P0-02-03
- **修复方案**：抽取到 backend/tests/common/mod.rs，所有测试文件引用
- **关联文件**：backend/tests/common/mod.rs + 14 个测试文件

##### P0-D12 8 个函数圈复杂度 >15（类二）

- **来源**：batch-02 P0-02-04
- **修复方案**：重构降低复杂度（如 business_mode_service.rs:179 check_module_consistency ~35 → 拆分为多个 match 分支函数）
- **关联文件**：business_mode_service.rs / 等 8 个文件

##### P0-D13 前端 60+ 组件缩写命名（类二）

- **来源**：batch-02 P0-02-05
- **修复方案**：重命名为描述性名称（如 SOList → SalesOrderList）
- **关联文件**：60+ .vue 文件

##### P0-D14 前端 api 命名不统一（类二）

- **来源**：batch-02 P0-02-06
- **修复方案**：统一为 `getXxxList / createXxx / updateXxx / deleteXxx` 命名
- **关联文件**：90+ api/*.ts 文件

##### P0-D15 升级流程非零停机（类二十五）

- **来源**：batch-21 P0-21-1
- **证据**：[upgrade.sh](file:///workspace/deploy/upgrade.sh) `systemctl stop` 导致 2-5s 服务中断
- **修复方案**：改为蓝绿部署 / 滚动重启，使用 systemctl reload nginx + 双实例切换
- **关联文件**：deploy/upgrade.sh / deploy/deploy.sh

##### P0-D16 报表订阅无后台调度任务（类十九）

- **来源**：batch-16 P0-16-1
- **证据**：report_subscription 表有 next_run_at 字段但无 cron 任务触发
- **修复方案**：新增 report_subscription_scheduler_service，每分钟扫描 next_run_at 到期的订阅，生成报表并发送通知
- **关联文件**：report_subscription_scheduler_service.rs / main.rs（启动 cron）

##### P0-D17 OA 公告完全未实现（类十九）

- **来源**：batch-16 P0-16-3
- **证据**：oa_announcement 仅有 Model，无 service/handler/路由
- **修复方案**：实现 oa_announcement_service / handler / 路由（CRUD + 可见性 + 权限）
- **关联文件**：oa_announcement_service.rs / oa_announcement_handler.rs / routes/

---

#### 优先级 5：财务与业务流程（约 30 P0）

##### P0-B01 坏账准备计提功能缺失（类十七）

- **来源**：batch-15 P0-15-1
- **修复方案**：实现坏账准备计提（账龄法：1年内 5% / 1-2年 20% / 2-3年 50% / 3年以上 100%），月末 cron 自动计提
- **关联文件**：bad_debt_service.rs / schema migrations / cron

##### P0-B02 坏账核销与审批流缺失（类十七）

- **来源**：batch-15 P0-15-2
- **修复方案**：实现坏账核销二级审批（申请人→财务经理→总经理），核销后更新 ar_balance
- **关联文件**：bad_debt_service.rs / approval_service.rs

##### P0-B03 催收任务管理缺失（类十七）

- **来源**：batch-15 P0-15-3
- **修复方案**：新增 collection_task 表，按账龄自动生成催收任务，分配给销售员，记录催收结果
- **关联文件**：collection_task_service.rs / collection_task_handler.rs / schema migrations

##### P0-B04 财务预警机制缺失（类十七）

- **来源**：batch-15 P0-15-4
- **修复方案**：实现财务预警（应收超额 / 库存积压 / 现金流不足 / 预算超支 4 类），触发通知
- **关联文件**：finance_alert_service.rs / notification_service.rs

##### P0-B05 大额调拨无额外验证（类十七）

- **来源**：batch-15 P0-15-5
- **修复方案**：资金调拨金额 > 阈值（如 10 万）需二级审批 + 短信验证码
- **关联文件**：fund_transfer_service.rs / approval_service.rs

##### P0-B06 预算超支无拦截（类十七）

- **来源**：batch-15 P0-15-6
- **修复方案**：费用报销 / 采购订单创建时校验预算余额，超支拦截
- **关联文件**：budget_service.rs / expense_service.rs / purchase_order_service.rs

##### P0-B07 回收规则无自动执行（类十七）

- **来源**：batch-15 P0-15-7
- **修复方案**：CRM 客户回收规则（30 天未联系 / 90 天无商机）自动执行，客户转入公海
- **关联文件**：customer_pool_service.rs / cron

##### P0-B08 赢率手填无自动计算（类十七）

- **来源**：batch-15 P0-15-8
- **修复方案**：商机赢率按阶段自动计算（prospecting 10% / qualification 25% / proposal 50% / negotiation 75% / closed_won 100%）
- **关联文件**：crm_opportunity_service.rs

##### P0-B09 输单原因未记录（类十七）

- **来源**：batch-15 P0-15-9
- **修复方案**：商机 closed_lost 时必填输单原因（价格/质量/服务/竞争对手/其他）
- **关联文件**：crm_opportunity_service.rs / 前端 opportunity_form.vue

##### P0-B10 BI 无数据权限过滤（类十九）

- **来源**：batch-16 P0-16-2
- **证据**：销售员可看所有销售数据
- **修复方案**：BI 查询注入 apply_data_scope，按 user_id / department_id 过滤
- **关联文件**：bi_analysis_service.rs / dashboard_service.rs

##### P0-B11 定制订单流程缺失打样和报价环节（类二十三）

- **来源**：batch-19 P0-19-1
- **修复方案**：定制订单流程补齐：需求确认 → 打样 → 客户确认 → 报价 → 生产订单
- **关联文件**：custom_order_service.rs / sample_service.rs / quotation_service.rs

##### P0-B12 售后与质量集成完全缺失（类二十三）

- **来源**：batch-19 P0-19-2
- **证据**：after_sales 表无 quality_issue_id 关联
- **修复方案**：after_sales 表新增 quality_issue_id 字段，售后工单触发 8D 流程
- **关联文件**：after_sales_service.rs / quality_issue_service.rs / schema migrations

##### P0-B13 物流签收无电子签收单（类二十三）

- **来源**：batch-19 P0-19-3
- **修复方案**：
  1. 新增 electronic_signature 表（签收人/签收时间/签收图片/GPS 定位）
  2. 签收触发应收确认（ar_balance 增加 + 凭证生成）
- **关联文件**：logistics_service.rs / ar_service.rs / schema migrations

##### P0-B14 Incoterms 2020 仅支持 5 种（类二十三）

- **来源**：batch-19 P0-19-4
- **证据**：当前仅 EXW/FOB/CIF/DAT/DDP
- **修复方案**：补齐 6 种（FCA/CPT/CIP/DPU/FAS/CFR），共 11 种
- **关联文件**：[incoterms.rs](file:///workspace/backend/src/models/incoterms.rs) / incoterms_service.rs / 前端选项

##### P0-B15 缺料预警状态不持久化（类二十二）

- **来源**：batch-18 P0-18-3
- **证据**：缺料预警仅内存计算，无法形成处理闭环
- **修复方案**：新增 material_shortage_alert 表（持久化预警记录 + 处理状态 + 责任人 + 月报）
- **关联文件**：material_shortage_service.rs / schema migrations

##### P0-B16 自动故障检测机制缺失（类二十）

- **来源**：batch-17 P0-17-1
- **修复方案**：实现自动故障检测（5s 间隔 / 连续 3 次失败触发告警）
- **关联文件**：[health_check_service.rs](file:///workspace/backend/src/observability/health_check_service.rs)

##### P0-B17 主备切换自动完成缺失（类二十）

- **来源**：batch-17 P0-17-2
- **修复方案**：主备切换 10s 内自动完成（心跳检测 + VIP 漂移 + 数据同步）
- **关联文件**：failover_service.rs / deploy/ha/

##### P0-B18 自动故障检测（重复，归并到 P0-B16）

##### P0-B19 报表订阅后台调度（归并到 P0-D16）

##### P0-B20 BI 数据权限过滤（归并到 P0-B10）

##### P0-B21 缺料预警状态持久化（归并到 P0-B15）

##### P0-B22 自动故障检测（归并到 P0-B16）

##### P0-B23 主备切换（归并到 P0-B17）

##### P0-B24 大货批色——ship_order 校验（归并到 P0-F19）

##### P0-B25 售后与质量集成（归并到 P0-B12）

##### P0-B26 物流签收（归并到 P0-B13）

##### P0-B27 Incoterms 补齐（归并到 P0-B14）

##### P0-B28 定制订单打样报价（归并到 P0-B11）

##### P0-B29 报表订阅后台调度（归并到 P0-D16）

##### P0-B30 BI 数据权限过滤（归并到 P0-B10）

---

### 2.2 阶段二：P1 高优先级修复（257 项，按类别分组）

> 每批 5-6 文件，遵循规则 13 连续执行流程。

#### P1 类一~类四（21 P1）

- **类一 回归验证**：0 P1
- **类二 通用代码质量**（3 P1）：
  - 前端 api 命名不统一（90+ 文件）
  - 前端视图+组件 60+ 缩写命名
  - 11 处 DbErr 返回（应包装为 AppError）
- **类三 安全性**（6 P1）：
  - refresh_token Cookie max_age 不一致
  - PUBLIC_PATHS 子路径放行过宽
  - request_validator 名不副实
  - Webhook payload 完整记录（敏感信息泄露）
  - crm import_leads 缺 magic bytes 校验
  - system_update 缺 zip bomb 防护
- **类四 面料行业深化**（11 P1）：
  - batch_trace_log 字段不足
  - 面料检验十项指标无建模
  - 工资确认未生成财务凭证
  - 能耗分摊简化逻辑
  - 委外加工无事件发布
  - 业务模式无事件发布
  - 直接人工成本无法自动归集
  - QualityInspectionCompleted 无发布者
  - 事件监听器仅打印日志
  - 多模块无事件发布
  - 染整工序无标准工时

#### P1 类五~类八（49 P1）

- **类五 运行逻辑闭环**（11 P1）：
  - 缸号状态机缺 Failed+OnHold
  - 面料行业配置环境变量缺失（6 个）
  - 6 个核心业务事件缺失
  - 生产订单成本归集未按缸号
  - 染色成本归集草稿 dye_lot_no=None
  - 销售成本未按移动加权平均法
  - 其他 5 项
- **类六 测试体系**（11 P1）：测试覆盖率 / mock 数据 / fixtures / 测试文档等
- **类七 可维护性**（11 P1）：i18n / aria-label / 缓存 / 文档等
- **类八 法律合规**（16 P1）：
  - 用户协议/隐私政策未接入
  - HTTPS 强制配置缺失
  - 手机号展示脱敏缺失
  - 染整报表导出缺失
  - .docx 格式不支持
  - 面料执行标准登记缺失
  - 合同电子签章缺失
  - 进项税转出缺失
  - 出口退税缺失
  - 环保税缺失
  - 排污许可证缺失
  - 废水废气固废监测缺失
  - 劳动合同电子化缺失
  - 工时加班合规缺失
  - 社保公积金合规缺失
  - 职业健康合规缺失

#### P1 类九~类十二（16 P1）

- **类九 色卡发放**（9 P1）：发放清单 / 通知 / 报表等
- **类十 大货批色**（7 P1）：批色提醒 / 报表 / 统计等

#### P1 类十三~类十四（28 P1）

- **类十三 打印导出**（14 P1）：14 端点审计字段补齐 / 水印增强 / 性能优化等
- **类十四 权限维度**（14 P1）：权限测试 / 审计日志 / 缓存优化等

#### P1 类十五~类十六（25 P1）

- **类十五 业务主体**（1 P1）：supplier_evaluation_records 表无 migration
- **类十六 AI 模块**（24 P1）：
  - 染料配伍性校验缺失
  - 化验室集成缺失
  - 质量预测准确率监控缺失
  - 模型版本管理缺失
  - AI 端点权限码未注册
  - AI 推理数据范围未过滤
  - AI 推理无超时
  - AI 并发控制缺失
  - AI 缓存策略缺失
  - AI 数据未脱敏
  - MLOps 完全缺失
  - 等 13 项

#### P1 类十七~类十九（52 P1）

- **类十七 财务深化**（35 P1）：
  - 会计期间缺 CLOSING 状态
  - 反结账缺失
  - 年结缺失
  - 坏账准备回转缺失
  - 账龄快照缺失
  - 杜邦分析缺失
  - 资金预测缺失
  - 预算差异分析缺失
  - 折旧方法单一
  - 等 26 项
- **类十八 CRM**（约 12 P1）：线索评分 / 去重 / 客户转移审批等
- **类十九 报表 BI**（约 5 P1）：报表版本管理 / BI 缓存等

#### P1 类二十~类二十二（28 P1）

- **类二十 可观测性**（9 P1）：trace 采样 / metrics 告警 / WebSocket ACK 等
- **类二十一 胚布拆匹**（约 10 P1）：胚布库存 / 委外加工 / 拆匹继承等
- **类二十二 库存排程**（约 9 P1）：调拨审批 / 安全库存 / 排程算法等

#### P1 类二十三~类二十五（38 P1）

- **类二十三 组织物流**（11 P1）：组织树 / 售后工单 / 运费核算等
- **类二十四 前端架构**（16 P1）：PWA / 移动端 / manualChunks / ErrorBoundary / CSP / keep-alive / CSS 变量 / 暗黑模式等
- **类二十五 部署升级**（11 P1）：set -euo pipefail / SHA256 校验 / schema 兼容性 / 蓝绿部署 / 健康检查门禁 / 优雅退出 / 自动回滚等

---

### 2.3 阶段三：P2 中优先级修复（248 项，按类别分批）

> 按类别分批修复，每批 5-6 文件。

| 类别 | P2 数 | 主要内容 |
|------|-------|----------|
| 类一~类四 | 19 | 代码质量 / 安全防护 / 面料行业字段补齐 |
| 类五~类八 | 47 | 运行逻辑 / 测试补充 / 可维护性 / 法律合规细节 |
| 类九~类十二 | 33 | 色卡发放细节 / 大货批色细节 / 打印导出 / 权限细节 |
| 类十三~类十四 | 25 | 打印导出 P2 / 权限 P2 |
| 类十五~类十六 | 53 | 业务主体 P2 / AI 模块 P2 |
| 类十七~类十九 | 39 | 财务 P2 / CRM P2 / 报表 BI P2 |
| 类二十~类二十二 | 25 | 可观测性 / 胚布 / 库存 P2 |
| 类二十三~类二十五 | 83 | 组织物流 / 前端架构 / 部署升级 P2 |
| **合计** | **248** | |

---

### 2.4 阶段四：P3 低优先级增强（123 项，按需修复）

| 类别 | P3 数 | 主要内容 |
|------|-------|----------|
| 类一~类四 | 11 | 文档 / 注释 / 命名优化 |
| 类五~类八 | 17 | 测试增强 / 可维护性增强 / 法律合规增强 |
| 类九~类十二 | 9 | 色卡 / 批色 / 打印 / 权限增强 |
| 类十三~类十四 | 5 | 打印导出 / 权限增强 |
| 类十五~类十六 | 25 | 业务主体增强 / AI 增强 |
| 类十七~类十九 | 11 | 财务 / CRM / 报表增强 |
| 类二十~类二十二 | 12 | 可观测性 / 胚布 / 库存增强 |
| 类二十三~类二十五 | 41 | 组织物流 / 前端架构 / 部署升级增强 |
| **合计** | **123** | |

---

## 三、规则节点提醒

- **规则 5（E2E 独立工作流，每 30 批次）**：V15 修复阶段每 30 批次触发（批次 30/60/90...）
- **规则 10（每 15 批次记忆整理 + 实时归档）**：V15 修复阶段每 15 批次整理；**实时归档要求**：每批完成后立即归档到 doto-su.md，doto.md 只保留未完成任务
- **规则 13（修复流程自动化）**：CI 全绿后自动开始下一批，无需用户确认
- **规则 14（移除所有警告抑制）**：所有警告视为错误需修复（baseline 213/213 ✅ 全部清零）
- **规则 15（V15 全项目综合审计）**：25 大类 195 维度审计 ✅ 已完成；下一步为 V15 修复阶段
- **规则 0/1/2/8（真实实现强制）**：所有 P0/P1 修复必须真实实现，禁止占位符
- **规则 3（成品文档格式）**：导出必须 .xlsx / 报表必须 .docx
- **规则 6（测试 mock 数据禁止硬编码）**：所有测试 mock 数据抽取到 fixtures
- **规则 11/12（法律合规与安全标准）**：所有修复必须符合中国法律法规 + 安全标准
- **§10.0.1 复用现有功能原则**：修复前必须调研现有实现，禁止重复造轮子

---

## 四、历史任务（全部完成，详细记录已归档）

> 以下阶段全部完成，详细记录已归档到 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)。

| 阶段 | 批次范围 | 内容 | 归档位置 |
|------|----------|------|----------|
| v13 复审修复 | 270-394 | 213 baseline + 业务/财务/运行逻辑闭环 | doto-su.md §v13 |
| v14 复审修复 | 395-432 | 12 P0 + 31 P1 + 12 P2 + 6 P3 + 213 baseline | doto-su.md §v14 |
| V15 审计 | 2026-07-16 | 25 大类 195 维度 21 批并行子代理审计 | docs/audits/v15/ |

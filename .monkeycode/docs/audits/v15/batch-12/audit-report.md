# V15 权限维度审计报告（类十四·批次 12）

- **审计子代理**：V15 审计子代理（类十四权限维度审计与角色合理性）
- **审计范围**：12 维度（类十四 14.1–14.12）
- **审计依据**：
  - `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md` 第 4759-5596 行（类十四审计计划）
  - `/workspace/backend/src/models/role.rs` / `role_permission.rs` / `field_permission.rs`
  - `/workspace/backend/src/services/init_service.rs` / `role_service.rs` / `role_permission_service.rs` / `data_permission_service.rs` / `field_permission_service.rs`
  - `/workspace/backend/src/middleware/permission.rs` / `auth_context.rs`
  - `/workspace/backend/src/utils/admin_checker.rs` / `path_utils.rs`
  - `/workspace/backend/src/handlers/auth_handler.rs` / `role_handler.rs`
  - `/workspace/backend/src/routes/mod.rs` / `iam.rs` / `production.rs` / `finance.rs`
  - `/workspace/backend/database/init_admin_permissions.sql` / `init_data.sql`
  - `/workspace/backend/migrations/20260323000001_initial_schema/up.sql`
  - `/workspace/backend/migrations/20260520000001_add_field_permissions/up.sql`
  - `/workspace/backend/migrations/20260527000001_add_basic_data_and_system_tables/up.sql`
  - `/workspace/frontend/src/router/index.ts` / `directives/permission.ts`
- **审计方法**：Read 审计计划 + Grep 检索 + Read 关键文件 + 对照审计计划核对
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码

---

## 维度 1：14.1 角色清单合理性审计

### 检查方法

- Read `/workspace/backend/src/services/init_service.rs` 第 342-418 行（`create_default_roles` 函数）
- Grep `INSERT INTO roles` 查找种子数据
- Read `/workspace/backend/database/init_admin_permissions.sql`（admin 权限种子）
- 对照审计计划 14.1.1 现有角色合理性矩阵

### 发现

#### ✅ 已落实的项

1. **admin 角色定义合理**：`init_service.rs:362-376` 创建 `code="admin"`、`is_system=true` 的系统管理员角色，符合审计计划 14.1.1 中"保留，但拆分审计职责"的建议基础。
2. **admin 角色使用常量定义**：`init_service.rs:367` 使用 `ADMIN_ROLE_CODE` 常量（来自 `admin_checker.rs:10`），消除硬编码字符串。
3. **角色 model 完整**：`models/role.rs:10-20` 包含 `id/name/code/description/permissions/is_system/created_at/updated_at` 字段。
4. **role_permissions 表结构完整**：`migrations/20260527000001_add_basic_data_and_system_tables/up.sql:177-187` 含 `role_id/resource_type/resource_id/action/allowed` 字段，支持资源 ID 级粒度。

#### ❌ 缺陷项

**缺陷 14.1-A：manager / operator 角色被错误标记为 is_system=true**
- **风险等级：P0**
- **证据**：`init_service.rs:379-403`
  ```rust
  let manager_role = role::ActiveModel {
      // ...
      code: Set("manager".to_string()),
      // ...
      is_system: Set(true),  // 🔴 不合理：部门经理不应为系统角色
      // ...
  };
  let operator_role = role::ActiveModel {
      // ...
      code: Set("operator".to_string()),
      // ...
      is_system: Set(true),  // 🔴 不合理：操作员不应为系统角色
      // ...
  };
  ```
- **业务影响**：`is_system=true` 会被 `auth_handler.rs:130` 的 `build_with_permissions` 函数识别为系统角色并注入 `*:*` 超级通配权限，使 manager 和 operator 实际等同于 admin，权限模型形同虚设。任何被分配 manager/operator 角色的用户都拥有全部资源的全部操作权限。
- **修复建议**：将 `manager` 和 `operator` 角色的 `is_system` 改为 `false`，并按业务域重新分配具体权限。

**缺陷 14.1-B：14 类面料行业业务专用角色完全缺失**
- **风险等级：P0**
- **证据**：
  - `init_service.rs:342-418` 仅创建 3 个角色（admin/manager/operator）
  - Grep `sales_manager|purchase_manager|warehouse_manager|production_manager|dye_operator|lab_technician|color_card_manager|finance_accountant|finance_reviewer|hr|auditor` 在 `backend/src/services/init_service.rs` 中无任何匹配
- **业务影响**：面料行业 ERP 应有销售经理/销售员/采购经理/采购员/仓库经理/仓库员/生产经理/染色操作员/化验室员/色卡管理员/财务会计/财务审核/HR/审计员等 14 类业务角色，当前完全缺失导致：
  - 无法按业务域分配权限
  - 所有非 admin 用户被迫分配 manager/operator 角色，间接获得 `*:*` 超级权限
  - 无法实现职责分离（SoD）
- **修复建议**：按审计计划 14.1.2 缺失业务角色补齐清单，在 `create_default_roles` 中补齐 14 类业务角色定义，所有业务角色 `is_system=false`。

**缺陷 14.1-C：审计计划提及的 014_init_role_permissions.sql 实际不存在**
- **风险等级：P2**
- **证据**：
  - 审计计划 14.1.1 提到 "014_init_role_permissions.sql 引用 role_id=4/5（采购经理/财务经理）角色根本不存在"
  - 实际 Grep `014_init_role_permissions|init_role_permissions` 在 `/workspace/backend` 中无任何匹配
  - 实际存在的种子文件为 `/workspace/backend/database/init_admin_permissions.sql`，仅定义 role_id=1（admin）的权限
- **业务影响**：审计计划描述的"014 引用不存在的 role_id=4/5"问题在当前代码中不存在，但说明审计计划基于的快照与当前代码状态可能不一致；同时 `init_admin_permissions.sql` 是手动执行的脚本，未接入迁移流程，存在初始化遗漏风险。
- **修复建议**：将 `init_admin_permissions.sql` 内容转化为正式 migration（如 `20260716000001_init_admin_permissions/up.sql`），由 `Migrator::up` 自动执行，避免依赖手动运行。

**缺陷 14.1-D：角色命名规范未校验**
- **风险等级：P3**
- **证据**：`role_service.rs:40-60` `create_role` 函数未校验 `code` 是否符合 `{业务域}_{职责}` 命名规范；`role_handler.rs:186-257` `create_role` 处理器也未校验
- **业务影响**：角色 code 可任意命名（如 "test123"、"abc"），无法保证角色清单的语义一致性，后续维护困难。
- **修复建议**：按审计计划 14.1.3 在 `RoleService::create_role` 中增加 `ROLE_CODE_PATTERN` 正则校验。

---

## 维度 2：14.2 权限分配矩阵审计

### 检查方法

- Read `/workspace/backend/database/init_admin_permissions.sql`（admin 权限种子）
- Read `/workspace/backend/src/handlers/auth_handler.rs:105-184`（`build_with_permissions` 函数）
- Read `/workspace/backend/src/middleware/permission.rs:164-215`（`check_permission` 函数）
- 对照审计计划 14.2.1 现有权限分配问题矩阵

### 发现

#### ✅ 已落实的项

1. **admin 权限种子覆盖 11 类核心资源**：`init_admin_permissions.sql:4-64` 为 admin 角色定义了 purchases/sales/inventory/finance/customers/suppliers/products/warehouses/users/audit/dashboard 共 11 类资源的 R/C/U/D 权限。
2. **权限查询使用参数化**：`permission.rs:193-200` 使用 SeaORM `QueryFilter` 参数化查询，防止 SQL 注入。
3. **action 通配符支持**：`permission.rs:223-236` `matches_permission` 函数支持 `action == "*"` 通配符匹配。
4. **resource_id 精确匹配防垂直越权**：`permission.rs:231-235` `None` 匹配 `None`、`Some(pid)` 匹配 `Some(rid)` 且必须相等，防止垂直越权。

#### ❌ 缺陷项

**缺陷 14.2-A：manager / operator 持有 `*:*` 超级通配权限**
- **风险等级：P0**
- **证据**：`auth_handler.rs:120-131`
  ```rust
  let permissions: Vec<String> = if let Some(role_id) = user.role_id {
      let role_model = crate::models::role::Entity::find_by_id(role_id)
          .one(db).await.ok().flatten();
      if let Some(ref role) = role_model {
          if role.is_system {  // 🔴 仅检查 is_system，未检查 role.code == "admin"
              vec!["*:*".to_string()]
          } else {
              // 查询 role_permissions 表
          }
      }
  }
  ```
- **业务影响**：由于 manager/operator 的 `is_system=true`（见缺陷 14.1-A），登录后会被注入 `*:*` 超级通配权限，等同于 admin。审计计划 14.2.1 中"manager 持有 `*:*`（因 is_system=true）"和"operator 持有 `*:*`（因 is_system=true）"两个 P0 问题确认存在。
- **修复建议**：按审计计划 14.5.2 修改 `build_with_permissions`：
  ```rust
  if role.code == ADMIN_ROLE_CODE && role.is_system {
      return vec!["*:*".to_string()];
  }
  ```

**缺陷 14.2-B：非 admin 角色在 60+ 类资源上无法通过权限检查**
- **风险等级：P0**
- **证据**：
  - `init_admin_permissions.sql` 仅定义 11 类资源权限
  - 后端路由暴露的资源类型远超 11 类（详见维度 4 的 14.4 缺口分析）
  - `permission.rs:164-215` `check_permission` 函数对非 admin 角色查询 `role_permissions` 表，若资源类型不在表中则返回 `false`（拒绝）
- **业务影响**：审计计划 14.2.1 中"manager（014 期望）权限过小"和"operator（014 期望）权限过小"问题确认存在。即使修复 is_system 滥用，非 admin 用户在 dye-batches/dye-recipes/lab-dip/flow-cards/cost-collections/production-orders/mrp/capacity/gl/ap/ar/vouchers/fixed-assets/budgets 等 60+ 类资源上仍会被 403 拒绝，前端路由可见但点击报错。
- **修复建议**：按审计计划 14.4.1 补齐 60+ 类资源的权限定义，或修改 `extract_resource_info` 支持业务域前缀映射（如 `sales/orders` 映射到 `sales:*` 权限）。

**缺陷 14.2-C：admin 持有 audit:read 违反职责分离**
- **风险等级：P1**
- **证据**：`init_admin_permissions.sql:60-61`
  ```sql
  (1, 'audit', 'read', true, NOW(), NOW()),
  ```
- **业务影响**：审计计划 14.2.1 指出"admin 既是操作者又能审计自己"。admin 拥有所有资源的 C/U/D 权限，同时持有 audit:read，可审计自己的操作，违反职责分离原则。
- **修复建议**：将审计职责独立到 `auditor` 角色（仅 R 权限），admin 不再持有 audit:read；或拆分超级管理员（system_admin）与业务管理员（business_admin）。

**缺陷 14.2-D：权限过大/过小检测规则未实现**
- **风险等级：P2**
- **证据**：Grep `detect_over_permission|detect_under_permission|OverPermissionIssue|UnderPermissionIssue` 在 `/workspace/backend/src` 中无任何匹配
- **业务影响**：审计计划 14.2.3 和 14.2.4 设计的权限过大/过小自动检测规则未实现，无法在角色权限分配时自动识别异常。
- **修复建议**：按审计计划 14.2.3/14.2.4 在 `role_permission_service.rs` 中实现检测函数，并在 `assign_permission` 时调用。

---

## 维度 3：14.3 职责分离（SoD）审计

### 检查方法

- Grep `role_conflict|RoleConflict|conflicting_roles` 在 `/workspace/backend/src` 中查找
- Read `/workspace/backend/src/services/role_service.rs` 和 `role_permission_service.rs` 查找互斥校验
- Grep `assign_role_to_user|user_role` 查找角色分配逻辑
- 对照审计计划 14.3.1 职责冲突矩阵和 14.3.2 角色互斥规则

### 发现

#### ✅ 已落实的项

1. **角色更新/删除有事务和锁保护**：`role_service.rs:67-112` `update_role` 和 `role_service.rs:117-136` `delete_role` 使用 `begin()` + `lock_exclusive()` 串行化并发状态变更。
2. **系统角色禁止修改/删除**：`role_service.rs:86-88` 和 `role_handler.rs:371-375` 检查 `is_system` 字段，系统角色不允许修改/删除。
3. **角色写操作有 admin 二次校验**：`role_handler.rs:27-41` `require_admin_role` 函数在 create_role/update_role/delete_role/assign_permission/remove_permission 顶部调用，强制要求 `role.code == "admin"`。

#### ❌ 缺陷项

**缺陷 14.3-A：角色互斥表（role_conflict）完全未实现**
- **风险等级：P0**
- **证据**：Grep `role_conflict|RoleConflict` 在 `/workspace/backend/src` 中无任何匹配；Grep `CREATE TABLE.*role_conflict` 在 `/workspace/backend/migrations` 中无任何匹配
- **业务影响**：审计计划 14.3.2 设计的 8 条互斥规则（如 finance_accountant 与 finance_reviewer 互斥、admin 与 auditor 互斥等）完全未实现。同一用户可同时拥有会计和审核角色，自审自批凭证；同时拥有 admin 和 auditor 角色，既操作又审计自己。
- **修复建议**：按审计计划 14.3.2 创建 `role_conflict` 表并插入 8 条互斥规则，在 `user_service::assign_role_to_user` 中实现互斥校验。

**缺陷 14.3-B：用户角色分配无互斥校验**
- **风险等级：P0**
- **证据**：Grep `assign_role_to_user` 在 `/workspace/backend/src` 中无任何匹配；用户角色分配通过 `user.role_id` 单字段实现（`models/user.rs`），不支持多角色分配
- **业务影响**：当前用户表只有 `role_id` 单字段，不支持同一用户拥有多个角色，无法实现互斥校验。但这也意味着无法实现"销售员 + 审计员"等组合角色，业务灵活性受限。
- **修复建议**：按审计计划 14.3.3 创建 `user_role` 关联表支持多角色，并在分配时进行互斥校验。

**缺陷 14.3-C：财务开凭证+审核凭证+删凭证三权未分立**
- **风险等级：P0**
- **证据**：`init_admin_permissions.sql:24-28` 为 admin 分配了 `finance:create/update/delete` 全部写权限
- **业务影响**：审计计划 14.3.1 指出"开凭证 + 删凭证"和"开凭证 + 审核凭证"冲突。当前 admin 一人可完成开凭证→审核凭证→删凭证全流程，无任何制衡。
- **修复建议**：拆分为 `finance_accountant`（仅 C）+ `finance_reviewer`（仅 U 审核）双角色制衡，删除需 reviewer 审批。

**缺陷 14.3-D：采购/销售审批与创建未分离**
- **风险等级：P1**
- **证据**：`init_admin_permissions.sql:7-16` admin 同时持有 `purchases:create/update/delete` 和 `sales:create/update/delete`
- **业务影响**：审计计划 14.3.1 指出"创建采购 + 审批采购"和"创建销售 + 审批销售"冲突。当前 admin 可自审自批采购单和销售单。
- **修复建议**：拆分 `purchase_staff`（C）+ `purchase_manager`（approve）和 `sales_staff`（C）+ `sales_manager`（approve）。

---

## 维度 4：14.4 权限-路由匹配审计

### 检查方法

- Grep `\.route\(` 在 `/workspace/backend/src/routes` 中提取所有路由
- Read `/workspace/backend/src/utils/path_utils.rs`（模块前缀白名单）
- Read `/workspace/backend/src/middleware/permission.rs:83-117`（`extract_resource_info` 函数）
- 对照 `init_admin_permissions.sql` 的 11 类资源权限

### 发现

#### ✅ 已落实的项

1. **路由资源类型提取逻辑完整**：`permission.rs:83-117` `extract_resource_info` 函数支持标准路径（`/api/v1/erp/users`）和模块前缀路径（`/api/v1/erp/sales/orders`）两种格式。
2. **模块前缀白名单已建立**：`path_utils.rs:1-31` `is_module_prefix` 函数列出 28 个模块前缀。
3. **公共路径白名单已实现**：`public_routes.rs` 中 `is_public_path` 函数（由 `permission.rs:32-36` 调用）跳过公共路径的权限检查。
4. **未知路径 fail-closed**：`permission.rs:114-116` 非 `/api/v1/erp/` 前缀的路径返回 `("unknown", None)`，`check_permission` 查询 `role_permissions` 表无匹配时返回 `false`（拒绝）。

#### ❌ 缺陷项

**缺陷 14.4-A：模块前缀白名单严重不足（28 个 vs 实际 50+ 个）**
- **风险等级：P0**
- **证据**：
  - `path_utils.rs:1-31` 仅列出 28 个模块前缀
  - 实际路由中存在但不在白名单的模块前缀：
    - `iam` 域：`/users`、`/roles`、`/departments`、`/permissions`、`/field-permissions`（无模块前缀，直接在 `/api/v1/erp/` 下）
    - `catalog` 域：`/products`、`/categories`、`/warehouses`、`/boms` 等
    - `production` 域：`/dye-batches`、`/greige-fabrics`、`/dye-recipes`、`/lab-dip`、`/production-recipes`、`/flow-cards`、`/fabric-inspections`、`/wage-rates`、`/energy-meters`、`/outsourcing-orders`、`/business-modes`、`/dye-batch-lifecycle-logs`、`/quality-inspection`、`/cost-collections`、`/production-orders`、`/mrp`、`/mrp-history`、`/capacity`（共 18 个子资源，均在 `/api/v1/erp/production/` 下）
    - `analytics` 域：18 个子资源
    - `system` 域：`/ws`、`/bpm`、`/audit-logs`、`/slow-queries` 等
- **业务影响**：审计计划 14.4.3 指出的 `iam`/`catalog`/`production`/`analytics`/`system`/`custom-orders`/`color-cards` 7 个模块前缀缺失确认存在。这些模块下的资源路径会被 `extract_resource_info` 错误提取资源类型（如 `/api/v1/erp/production/dye-batches` 被提取为 `dye-batches` 而非 `production` 域下的资源），导致权限检查与权限表不匹配。
- **修复建议**：按审计计划 14.4.3 补齐 `path_utils.rs` 模块前缀白名单至覆盖所有 50+ 资源；或重构 `extract_resource_info` 支持业务域前缀映射。

**缺陷 14.4-B：60+ 类资源缺失权限定义**
- **风险等级：P0**
- **证据**：
  - `init_admin_permissions.sql` 仅定义 11 类资源权限（purchases/sales/inventory/finance/customers/suppliers/products/warehouses/users/audit/dashboard）
  - 后端路由实际暴露的资源类型（部分列举）：
    - 销售域：`orders`、`sales-contracts`、`sales-prices`、`sales-returns`、`fabric-orders`、`quotations`
    - 采购域：`orders`（采购）、`receipts`、`inspections`、`returns`、`purchase-contracts`、`purchase-prices`、`supplier-evaluations`
    - 库存域：`stock`、`transfers`、`adjustments`、`reservations`、`counts`、`batches`、`logistics`
    - 财务域：`gl`、`fixed-assets`、`budgets`、`financial-analysis`、`fund-management`、`ap`、`ar`、`ar-reconciliations`、`currencies`、`exchange-rates`、`vouchers`、`payments`、`invoices`、`accounting-periods`
    - 生产域：`dye-batches`、`greige-fabrics`、`dye-recipes`、`lab-dip`、`production-recipes`、`flow-cards`、`fabric-inspections`、`quality-inspection`、`cost-collections`、`production-orders`、`mrp`、`mrp-history`、`capacity`、`wage-rates`、`energy-meters`、`outsourcing-orders`、`business-modes`
    - CRM 域：`customer-credits`、`five-dimension`、`sales-analysis`、`crm-customers`、`crm-tags`、`crm-pool`、`crm-assignments`
    - IAM 域：`roles`、`departments`、`permissions`、`field-permissions`
    - 分析域：`dual-unit`、`assist-accounting`、`business-trace`、`scanner`、`reports-enhanced`、`imports`、`exports`、`security`、`emails`、`ai`、`reports`、`webhooks`、`api-gateway`、`data-permissions`、`notifications`
    - 系统域：`ws`、`system-update`、`bpm`、`audit-logs`、`slow-queries`、`init`
- **业务影响**：审计计划 14.4.1 列出的 60+ 类资源缺口确认存在。非 admin 用户（修复 is_system 后）在这些资源上会被 403 拒绝，前端路由可见但点击报错。
- **修复建议**：按审计计划 14.4.1 补齐 60+ 类资源的权限定义；或修改 `extract_resource_info` 支持"业务域前缀映射"（如 `sales/orders` 映射到 `sales:*` 权限）。

**缺陷 14.4-C：权限码与路由资源类型不匹配**
- **风险等级：P1**
- **证据**：
  - `init_admin_permissions.sql:7-10` 定义 `purchases:read/create/update/delete`
  - 但 `purchase.rs:28-32` 路由为 `/orders`（在 `/api/v1/erp/purchase/` 前缀下）
  - `extract_resource_info` 提取资源类型为 `orders`（因 `purchases` 在白名单中，取第 5 段 `orders`）
  - `role_permissions` 表中无 `orders:read` 权限
- **业务影响**：审计计划 14.4.2 指出的"权限码用业务域 `sales`/`purchases`，路由资源类型用 `orders`"不匹配问题确认存在。非 admin 用户即使持有 `purchases:read` 权限，访问 `/api/v1/erp/purchase/orders` 时也会被 403 拒绝（因 `extract_resource_info` 提取的是 `orders` 而非 `purchases`）。
- **修复建议**：按审计计划 14.4.2 补齐 `orders:*` 权限 OR 修改 `extract_resource_info` 支持"业务域前缀映射"。

**缺陷 14.4-D：模块前缀不在白名单时资源类型提取错误**
- **风险等级：P1**
- **证据**：`permission.rs:95-99`
  ```rust
  let resource_type = if path_parts.len() >= 5 && is_module_prefix(path_parts[3]) {
      path_parts[4].to_string()
  } else {
      path_parts[3].to_string()
  };
  ```
  对于 `/api/v1/erp/production/dye-batches`，由于 `production` 不在白名单中，`is_module_prefix("production")` 返回 `false`，资源类型被错误提取为 `production` 而非 `dye-batches`。
- **业务影响**：审计计划 14.4.3 指出的 `production`/`analytics`/`system` 等模块前缀缺失导致资源类型提取错误确认存在。
- **修复建议**：补齐 `path_utils.rs` 模块前缀白名单至覆盖所有 50+ 资源。

---

## 维度 5：14.5 is_system 滥用治理

### 检查方法

- Read `/workspace/backend/src/handlers/auth_handler.rs:105-184`（`build_with_permissions` 函数）
- Read `/workspace/backend/src/services/init_service.rs:342-418`（`create_default_roles` 函数）
- Read `/workspace/backend/src/utils/admin_checker.rs`（`is_admin_role` 函数）
- 对照审计计划 14.5.1 is_system 滥用问题矩阵

### 发现

#### ✅ 已落实的项

1. **`is_admin_role` 严格检查 role.code == "admin"**：`admin_checker.rs:77-78`
   ```rust
   Ok(Some(role)) => role.code == ADMIN_ROLE_CODE,
   ```
   `is_admin_role` 函数不仅检查 `is_system`，还严格检查 `role.code == "admin"`，是 fail-closed 设计。
2. **DB 错误时 fail-closed**：`admin_checker.rs:80-89` 数据库查询失败时返回 `false`（拒绝访问），防止系统未初始化时放行。
3. **admin 角色缓存清理机制**：`admin_checker.rs:46-57` `clear_admin_role_cache` 函数在角色更新/删除后清理缓存，避免过期判定。
4. **`is_admin_role` 有 5 分钟 TTL 缓存**：`admin_checker.rs:40` `ADMIN_CACHE_TTL_MINUTES = 5`，减少 DB 查询压力。

#### ❌ 缺陷项

**缺陷 14.5-A：`build_with_permissions` 仅检查 is_system 未检查 role.code == "admin"**
- **风险等级：P0**
- **证据**：`auth_handler.rs:129-131`
  ```rust
  if let Some(ref role) = role_model {
      if role.is_system {  // 🔴 仅检查 is_system，未检查 role.code == "admin"
          vec!["*:*".to_string()]
      } else {
          // 查询 role_permissions 表
      }
  }
  ```
- **业务影响**：审计计划 14.5.1 指出的"manager 登录后注入 `*:*`"和"operator 登录后注入 `*:*`"问题确认存在。由于 manager/operator 的 `is_system=true`（见缺陷 14.1-A），登录后会被注入 `*:*` 超级通配权限。这与 `is_admin_role`（严格检查 role.code == "admin"）的行为不一致，导致前后端权限边界不一致（见维度 8 的 14.6）。
- **修复建议**：按审计计划 14.5.2 修改 `build_with_permissions`：
  ```rust
  if role.code == ADMIN_ROLE_CODE && role.is_system {
      return vec!["*:*".to_string()];
  }
  ```

**缺陷 14.5-B：manager/operator 的 is_system=true 未修复**
- **风险等级：P0**
- **证据**：`init_service.rs:386, 400`
  ```rust
  let manager_role = role::ActiveModel {
      // ...
      is_system: Set(true),  // 🔴 应为 false
      // ...
  };
  let operator_role = role::ActiveModel {
      // ...
      is_system: Set(true),  // 🔴 应为 false
      // ...
  };
  ```
- **业务影响**：审计计划 14.5.1 指出的"is_system=true 应仅用于 admin 角色"原则被违反。
- **修复建议**：将 manager/operator 的 `is_system` 改为 `false`；或按审计计划 14.5.3 提供修复脚本：
  ```sql
  UPDATE roles SET is_system = false WHERE code IN ('manager', 'operator');
  ```

**缺陷 14.5-C：is_system 字段无数据库约束**
- **风险等级：P2**
- **证据**：`migrations/20260323000001_initial_schema/up.sql` 中 `roles` 表的 `is_system` 字段无 CHECK 约束（Grep `is_system` 在 migrations 中无 CHECK 约束）
- **业务影响**：数据库层面无法阻止 `is_system=true` 被滥用于非 admin 角色。
- **修复建议**：增加应用层校验（`role_service::create_role` 检查 `is_system=true` 时 `code` 必须为 `admin`），或数据库 CHECK 约束。

---

## 维度 6：14.6 前后端权限边界一致性审计

### 检查方法

- Read `/workspace/frontend/src/router/index.ts`（前端路由守卫和 `hasRoutePermission` 函数）
- Read `/workspace/frontend/src/directives/permission.ts`（前端权限指令）
- Read `/workspace/backend/src/handlers/auth_handler.rs:105-184`（后端 `build_with_permissions` 函数）
- Read `/workspace/backend/src/utils/admin_checker.rs`（后端 `is_admin_role` 函数）
- 对照审计计划 14.6.1 不一致场景矩阵

### 发现

#### ✅ 已落实的项

1. **前端权限码格式与后端一致**：前端 `hasRoutePermission`（`router/index.ts:849-874`）和后端 `build_with_permissions`（`auth_handler.rs:138-141`）都使用 `"{resource}:{action}"` 格式。
2. **前端支持 `*:*` 超级通配符**：`router/index.ts:864` `if (upResource === '*' && upAction === '*') return true`，与后端 `*:*` 注入逻辑对齐。
3. **前端支持 `resource:*` 资源通配符**：`router/index.ts:866` `if (upAction === '*') return true`，与后端 `matches_permission` 的 `action == "*"` 通配符对齐。
4. **前端 read/view 等价、update/edit 等价**：`router/index.ts:869-870` 兼容后端两套 action 命名。
5. **前端删除了 `role_name === 'admin'` 硬编码绕过**：`directives/permission.ts:25-28` 注释说明"P2 1-12 修复：删除 role_name === 'admin' 硬编码绕过"，改为后端注入 `*:*` 通配权限。
6. **前端路由 meta.permission 全部补齐**：`router/index.ts` 中所有业务路由（约 100+ 条）都配置了 `meta.permission` 字段。

#### ❌ 缺陷项

**缺陷 14.6-A：前端 `*:*` 放行与后端 `is_admin_role` 拒绝的不一致**
- **风险等级：P0**
- **证据**：
  - 前端：`auth_handler.rs:129-131` 为 manager/operator 注入 `*:*`（因 is_system=true）
  - 前端：`router/index.ts:864` `if (upResource === '*' && upAction === '*') return true` 放行所有路由
  - 后端：`admin_checker.rs:77-78` `is_admin_role` 严格检查 `role.code == "admin"`，manager/operator 不被承认为 admin
  - 后端：`role_handler.rs:27-41` `require_admin_role` 调用 `is_admin_role`，manager/operator 访问角色管理接口时返回 403
- **业务影响**：审计计划 14.6.1 指出的"前端可见菜单但后端返回 403"问题确认存在。manager/operator 用户在前端可看到角色管理/用户管理/系统设置等菜单（因 `*:*` 通过 `hasRoutePermission`），但点击后后端 `require_admin_role` 返回 403。
- **修复建议**：按审计计划 14.6.2 方案 A 统一前后端权限模型：
  1. 前端 `hasRoutePermission` 不再特殊处理 `*:*`，改为逐权限匹配
  2. 后端 `build_with_permissions` 不再注入 `*:*`，改为注入实际权限列表
  3. `require_admin_role` 改为 `require_permission("system:admin")`
  4. 删除 `is_admin_role` 绕过机制，所有权限走统一 `permission_middleware`

**缺陷 14.6-B：前端权限码单复数不一致**
- **风险等级：P2**
- **证据**：
  - 前端路由 `meta.permission` 使用复数：`users:read`、`customers:read`、`suppliers:read`、`products:read`、`warehouses:read`（`router/index.ts:59, 119, 125, 131, 137`）
  - 后端 `init_admin_permissions.sql` 也使用复数：`users:read`、`customers:read`、`suppliers:read`、`products:read`、`warehouses:read`（`init_admin_permissions.sql:55, 31, 37, 43, 49`）
  - 但审计计划 14.6.3 提到"v-permission 文档示例使用 `user:create`（单数）"，与后端 `users:create`（复数）不一致
- **业务影响**：审计计划 14.6.3 指出的单复数不一致问题在文档示例中存在，但实际路由和权限表中已统一为复数。需确认前端业务组件中 `v-permission` 指令的使用是否全部使用复数。
- **修复建议**：审计前端所有 `v-permission` 使用，统一为复数格式。

**缺陷 14.6-C：前端仅控制 `:read` 路由可见性，未控制写按钮可见性**
- **风险等级：P2**
- **证据**：
  - 前端路由 `meta.permission` 全部使用 `:read`（如 `users:read`、`sales:read`、`purchases:read`）
  - 后端权限表有 `:create`/`:update`/`:delete` 写权限
  - 前端 `v-permission` 指令（`directives/permission.ts`）可用于控制按钮可见性，但 Grep `v-permission` 在 `/workspace/frontend/src/views` 中的使用情况未全面审计
- **业务影响**：审计计划 14.6.3 指出的"前端未控制写按钮可见性"问题可能存在。用户无 `:create` 权限但仍能看到"新建"按钮，点击后后端返回 403。
- **修复建议**：审计前端所有写按钮（新建/编辑/删除）是否使用 `v-permission` 控制可见性，补齐缺失的写权限控制。

---

## 维度 7：14.7 业务角色权限矩阵设计审计

### 检查方法

- Read `/workspace/backend/src/services/init_service.rs:342-418`（`create_default_roles` 函数）
- Read `/workspace/backend/database/init_admin_permissions.sql`（admin 权限种子）
- Grep `sales_manager|purchase_manager|warehouse_manager|production_manager|dye_operator|lab_technician|color_card_manager|finance_accountant|finance_reviewer|hr|auditor` 在 `/workspace/backend/src`
- 对照审计计划 14.7.1-14.7.6 业务角色权限矩阵

### 发现

#### ✅ 已落实的项

1. **admin 角色权限矩阵完整**：`init_admin_permissions.sql` 为 admin 定义了 11 类核心资源的 R/C/U/D 权限，覆盖审计计划 14.2.2 中 admin 行的所有资源。
2. **admin 持有 dashboard:read**：`init_admin_permissions.sql:64` `(1, 'dashboard', 'read', true, ...)`，admin 可进入仪表板。

#### ❌ 缺陷项

**缺陷 14.7-A：14 类业务角色的权限矩阵完全未实现**
- **风险等级：P0**
- **证据**：
  - Grep `sales_manager|purchase_manager|warehouse_manager|production_manager|dye_operator|lab_technician|color_card_manager|finance_accountant|finance_reviewer|hr|auditor` 在 `/workspace/backend/src` 中无任何匹配
  - `init_admin_permissions.sql` 仅为 role_id=1（admin）定义权限，无其他角色的权限种子
- **业务影响**：审计计划 14.7.1-14.7.6 设计的 14 类业务角色权限矩阵（销售域/采购域/库存域/生产域/财务域/其他）完全未实现。即使补齐角色定义（缺陷 14.1-B），也无对应的权限种子数据。
- **修复建议**：按审计计划 14.7.1-14.7.6 为 14 类业务角色编写权限种子 SQL，并在 `create_default_roles` 中调用。

**缺陷 14.7-B：业务角色无 dashboard:read 权限**
- **风险等级：P1**
- **证据**：`init_admin_permissions.sql` 仅为 admin 定义 `dashboard:read`，无其他角色的 dashboard 权限
- **业务影响**：审计计划 14.2.4 指出的"业务角色无 dashboard:read，无法进入系统仪表板"问题确认存在。非 admin 用户登录后前端路由守卫（`router/index.ts:916-924`）检查 `dashboard:read` 权限，无此权限则跳转 `/403`。
- **修复建议**：为所有业务角色分配 `dashboard:read` 权限。

**缺陷 14.7-C：销售域角色权限矩阵未实现**
- **风险等级：P0**
- **证据**：Grep `sales-contracts|sales-prices|sales-returns|ar-reconciliations` 在 `init_admin_permissions.sql` 中无匹配
- **业务影响**：审计计划 14.7.1 设计的销售域角色权限矩阵（sales_manager/sales 对 sales/sales-contracts/sales-prices/sales-returns/customers/ar-reconciliations/dashboard 的权限）完全未实现。
- **修复建议**：按审计计划 14.7.1 为 sales_manager 和 sales 角色编写权限种子。

**缺陷 14.7-D：采购域/库存域/生产域/财务域角色权限矩阵未实现**
- **风险等级：P0**
- **证据**：同缺陷 14.7-A
- **业务影响**：审计计划 14.7.2-14.7.5 设计的采购域/库存域/生产域/财务域角色权限矩阵完全未实现。
- **修复建议**：按审计计划 14.7.2-14.7.5 为各业务域角色编写权限种子。

---

## 维度 8：14.8 权限粒度审计（行级/字段级）

### 检查方法

- Grep `apply_data_scope|data_scope` 在 `/workspace/backend/src` 中查找
- Read `/workspace/backend/src/services/data_permission_service.rs`（数据权限服务）
- Read `/workspace/backend/src/services/field_permission_service.rs`（字段权限服务）
- Read `/workspace/backend/src/models/field_permission.rs`（字段权限模型）
- Grep `filter_fields|get_role_data_permission` 查找字段级权限使用情况
- 对照审计计划 14.8.1 行级数据权限缺口和 14.8.2 字段级权限缺口

### 发现

#### ✅ 已落实的项

1. **`data_permissions` 表已建立**：`migrations/20260527000001_add_basic_data_and_system_tables/up.sql:196-208` 创建 `data_permissions` 表，含 `role_id/resource_type/scope_type/custom_condition/allowed_fields/hidden_fields/is_enabled` 字段，支持字段级权限。
2. **`field_permissions` 表已建立**：`migrations/20260520000001_add_field_permissions/up.sql` 创建 `field_permissions` 表，含 `role_id/resource_type/field_name/can_read/can_write/mask_strategy/is_enabled` 字段。
3. **`DataPermissionService` 已实现字段过滤**：`data_permission_service.rs:196-228` `filter_fields` 和 `filter_fields_batch` 函数根据 `allowed_fields`/`hidden_fields` 过滤 JSON 数据。
4. **字段级权限已接入 7 个 handler**：
   - `purchase_receipt_handler.rs:55, 101, 249`（采购收货）
   - `crm_handler.rs:61, 158, 252, 309`（CRM）
   - `inventory_stock_handler.rs:62, 339`（库存）
   - `sales_order_handler.rs:69, 129`（销售订单）
   - `ap_payment_request_handler.rs:81, 135`（AP 付款）
   - `purchase_order_handler.rs:55, 108`（采购订单）
   - `customer_handler.rs:345`（客户）
5. **`FieldPermissionService` 已实现 CRUD**：`field_permission_service.rs` 提供 `list_field_permissions/get_field_permission/create_field_permission/update_field_permission/delete_field_permission` 接口。
6. **`field_permission_handler` 已挂载路由**：`routes/iam.rs:77-90` 暴露 `/field-permissions` CRUD 接口。

#### ❌ 缺陷项

**缺陷 14.8-A：行级数据权限（apply_data_scope）完全未实现**
- **风险等级：P0**
- **证据**：
  - Grep `apply_data_scope` 在 `/workspace/backend/src` 中无任何匹配
  - `data_permission_service.rs:21-23` `data_scope` 模块仅保留 `ALL` 常量，注释说明"批次 119 P2-5 修复：删除 4 个未接入业务的 scope 常量（DEPT/DEPT_AND_BELOW/SELF/CUSTOM），仅保留 ALL（admin 角色使用）"
  - `AuthContext`（`auth_context.rs:46-53`）仅含 `user_id/username/role_id`，无 `customer_id/warehouse_id/department_id` 字段
- **业务影响**：审计计划 14.8.1 指出的 5 个行级数据权限场景全部未实现：
  - 销售员只能看本人订单：无 `salesperson_id` 过滤
  - 采购员只能看本人采购单：无 `buyer_id` 过滤
  - 客户只能看本人订单：无 `customer_id` 过滤
  - 仓库员只能看本仓库库存：无 `warehouse_id` 过滤
  - 化验员只能看本人打样记录：无 `created_by` 过滤
  当前所有非 admin 用户都能看到全表数据，存在数据泄露风险。
- **修复建议**：按审计计划 14.8.3 实现 `apply_data_scope` 函数，扩展 `AuthContext` 增加 `customer_id/warehouse_id/department_id` 字段，并在各 handler 的查询中调用 `apply_data_scope`。

**缺陷 14.8-B：字段级权限种子数据完全为空**
- **风险等级：P1**
- **证据**：
  - `field_permissions` 表已建立但无种子数据（Grep `INSERT INTO field_permissions` 在 `/workspace/backend` 中无匹配）
  - `data_permissions` 表同样无种子数据
- **业务影响**：审计计划 14.8.2 指出的 4 个字段级权限场景未实现：
  - 销售员不能看成本价：无 `sales_order.cost_price HIDDEN` 配置
  - 销售员不能看客户信用额度：无 `customer.credit_limit HIDDEN` 配置
  - 采购员不能看供应商底价：无 `supplier.floor_price HIDDEN` 配置
  - 化验员不能看配方用量：无 `dye_recipe.quantity HIDDEN` 配置
  当前所有非 admin 用户都能看到所有字段，存在敏感数据泄露风险。
- **修复建议**：按审计计划 14.8.4 编写 `field_permissions` 表种子数据。

**缺陷 14.8-C：`FieldPermissionService::filter_fields_by_read_permission` 未接入业务**
- **风险等级：P2**
- **证据**：
  - `field_permission_service.rs:233` 定义了 `filter_fields_by_read_permission` 函数
  - 但 Grep `filter_fields_by_read_permission` 在 `/workspace/backend/src/handlers` 中无任何匹配
- **业务影响**：`FieldPermissionService` 的字段过滤功能未接入业务 handler，字段级权限实际未生效。
- **修复建议**：将 `filter_fields_by_read_permission` 接入各业务 handler，替代或补充 `DataPermissionService::filter_fields`。

**缺陷 14.8-D：`data_scope` 模块仅保留 ALL 常量**
- **风险等级：P2**
- **证据**：`data_permission_service.rs:18-23`
  ```rust
  /// 数据范围类型常量
  ///
  /// 批次 119 P2-5 修复：删除 4 个未接入业务的 scope 常量（DEPT/DEPT_AND_BELOW/SELF/CUSTOM），
  /// 仅保留 ALL（admin 角色使用）。如未来需要行级权限校验，应通过 data_permission 表的
  /// scope_type 字段动态读取，而非硬编码常量。
  pub mod data_scope {
      pub const ALL: &str = "ALL";
  }
  ```
- **业务影响**：行级数据权限的 scope 类型被简化为仅 ALL，无法表达 DEPT/SELF/CUSTOM 等行级范围。
- **修复建议**：按审计计划 14.8.3 恢复 DEPT/DEPT_AND_BELOW/SELF/CUSTOM scope 常量并接入业务。

---

## 维度 9：14.9 权限缓存与性能审计

### 检查方法

- Read `/workspace/backend/src/middleware/permission.rs:131-215`（权限缓存实现）
- Read `/workspace/backend/src/utils/admin_checker.rs:36-99`（admin 角色缓存实现）
- Grep `invalidate_user_permission_cache|invalidate_role_permission_cache` 查找缓存失效机制
- 对照审计计划 14.9.1 缓存问题矩阵

### 发现

#### ✅ 已落实的项

1. **权限缓存已实现**：`permission.rs:154-208` 使用 `DashMap` + `CacheEntry` 实现 5 分钟 TTL 缓存，减少 DB 查询压力。
2. **admin 角色缓存已实现**：`admin_checker.rs:36-99` 使用 `DashMap` + `AdminCacheEntry` 实现 5 分钟 TTL 缓存。
3. **admin 角色缓存清理机制**：`admin_checker.rs:46-57` `clear_admin_role_cache` 函数在角色更新/删除后清理缓存。
4. **权限缓存使用 Arc 包装**：`permission.rs:203-206` 使用 `Arc<Vec<role_permission::Model>>` 包装权限列表，克隆时只增加引用计数不复制数据。
5. **过期缓存自动清理**：`admin_checker.rs:55-57` `cleanup_expired_admin_cache` 函数由 main.rs 后台任务每 10 分钟调用。
6. **权限缓存过期后自动重新加载**：`permission.rs:178-187` 缓存过期时移除并重新从 DB 加载。

#### ❌ 缺陷项

**缺陷 14.9-A：权限变更后缓存不主动失效**
- **风险等级：P0**
- **证据**：
  - Grep `invalidate_user_permission_cache|invalidate_role_permission_cache|PERMISSION_CACHE.remove|PERMISSION_CACHE.clear` 在 `/workspace/backend/src` 中无主动失效调用
  - `role_handler.rs:assign_permission` 和 `remove_permission` 分配/移除权限后未清理 `PERMISSION_CACHE`
  - `role_handler.rs:update_role` 和 `delete_role` 仅清理 `ADMIN_ROLE_CACHE`（`clear_admin_role_cache`），未清理 `PERMISSION_CACHE`
- **业务影响**：审计计划 14.9.1 指出的"权限变更延迟"问题确认存在。管理员撤销某用户的角色权限后，该用户在 5 分钟内仍可使用旧权限访问资源，存在安全窗口。
- **修复建议**：按审计计划 14.9.2 实现 `invalidate_role_permission_cache` 函数，在 `assign_permission`/`remove_permission`/`update_role`/`delete_role` 后调用，清除对应 `role_id` 的 `PERMISSION_CACHE`。

**缺陷 14.9-B：用户禁用后缓存不主动失效**
- **风险等级：P0**
- **证据**：
  - Grep `is_active.*cache|invalidate_user` 在 `/workspace/backend/src` 中无主动失效调用
  - 用户禁用后 `is_active=false`，但 `permission_middleware` 不检查 `is_active`（仅 `auth_middleware` 检查）
  - 若用户已有有效 JWT，禁用后 JWT 在有效期内（30 分钟）仍可访问
- **业务影响**：审计计划 14.9.1 指出的"用户禁用延迟"问题确认存在。被禁用用户在 JWT 有效期内仍可访问系统。
- **修复建议**：实现 Redis 黑名单实时同步，用户禁用时立即吊销所有 JWT（调用 `revoke_user_jtis`）。

**缺陷 14.9-C：权限缓存无 Redis pub/sub 热更新**
- **风险等级：P1**
- **证据**：Grep `redis.*publish|permission_invalidation` 在 `/workspace/backend/src` 中无匹配
- **业务影响**：审计计划 14.9.1 指出的"角色权限变更不通知"问题确认存在。多实例部署时，一个实例清理了缓存，其他实例仍使用旧缓存 5 分钟。
- **修复建议**：按审计计划 14.9.2 实现 Redis pub/sub 通知所有实例清除缓存。

**缺陷 14.9-D：权限缓存仅 5 分钟 TTL，无法配置**
- **风险等级：P3**
- **证据**：`permission.rs:160` `const PERMISSION_CACHE_TTL: i64 = 5;` 硬编码 5 分钟
- **业务影响**：TTL 无法根据业务需求动态调整。
- **修复建议**：将 TTL 改为可配置项（从 `config.yaml` 读取）。

---

## 维度 10：14.10 权限审计日志与合规审查

### 检查方法

- Grep `permission_change_audit|PermissionChangeAudit` 在 `/workspace/backend/src` 中查找
- Read `/workspace/backend/src/handlers/role_handler.rs`（角色管理审计日志）
- Grep `weekly_permission_compliance_review|compliance_review_report` 查找定期合规审查
- 对照审计计划 14.10.1-14.10.4

### 发现

#### ✅ 已落实的项

1. **角色 CRUD 有审计日志**：`role_handler.rs:208-232`（create_role）、`role_handler.rs:301-325`（update_role）、`role_handler.rs:396-414`（delete_role）都调用 `AuditLogService::record_async` 记录审计日志，含 `before_snapshot`/`after_snapshot`。
2. **权限分配/移除有审计日志**：`role_handler.rs:444-472`（assign_permission）、`role_handler.rs:499-520`（remove_permission）都记录审计日志，含 `after_snapshot`。
3. **审计日志使用 `record_async` 异步落库**：避免阻塞主流程。
4. **审计日志含操作人信息**：`role_handler.rs:209-211` 等处记录 `user_id`/`username`。
5. **审计日志含资源详情**：`role_handler.rs:216-219` 等处记录 `resource_type`/`resource_id`/`resource_name`。

#### ❌ 缺陷项

**缺陷 14.10-A：`permission_change_audit` 表完全未实现**
- **风险等级：P0**
- **证据**：
  - Grep `permission_change_audit|PermissionChangeAudit` 在 `/workspace/backend/src` 中无任何匹配
  - Grep `CREATE TABLE.*permission_change_audit` 在 `/workspace/backend/migrations` 中无任何匹配
- **业务影响**：审计计划 14.10.1 设计的权限变更审计日志表（含 `operator_id/target_type/target_id/change_type/permission_code/role_code/old_value/new_value/reason/ip_address` 字段）完全未实现。当前权限变更审计仅记录到通用 `audit_log` 表，无法支持审计计划 14.10.2 的异常权限分配识别规则（如非工作时间权限变更、批量权限授予、超级权限授予等）。
- **修复建议**：按审计计划 14.10.1 创建 `permission_change_audit` 表，并在角色权限变更时写入。

**缺陷 14.10-B：异常权限分配识别规则未实现**
- **风险等级：P1**
- **证据**：Grep `security_alert|非工作时间|批量权限授予|超级权限授予` 在 `/workspace/backend/src` 中无匹配
- **业务影响**：审计计划 14.10.2 设计的 6 条异常权限分配识别规则（非工作时间变更/批量授予/超级权限授予/互斥角色分配/离职用户权限未撤销/权限回滚）完全未实现。
- **修复建议**：按审计计划 14.10.2 实现异常权限分配识别规则，并接入告警系统。

**缺陷 14.10-C：定期合规审查机制未实现**
- **风险等级：P1**
- **证据**：Grep `weekly_permission_compliance_review|compliance_review_report` 在 `/workspace/backend/src` 中无匹配
- **业务影响**：审计计划 14.10.3 设计的每周一 02:00 定期合规审查机制完全未实现。无法自动检测 is_system=true 的非 admin 角色、互斥角色冲突、离职用户权限未撤销等问题。
- **修复建议**：按审计计划 14.10.3 实现定期合规审查定时任务。

**缺陷 14.10-D：审计日志保留期限未配置**
- **风险等级：P2**
- **证据**：Grep `permission_change_audit.*7.*年|compliance_review_report.*3.*年|security_alert_log.*7.*年` 在 `/workspace/backend` 中无匹配
- **业务影响**：审计计划 14.10.4 设计的审计日志保留期限（权限变更审计日志 7 年/合规审查报告 3 年/安全告警记录 7 年）未配置，可能存在日志被过早清理的合规风险。
- **修复建议**：按审计计划 14.10.4 配置审计日志保留期限，并实现自动归档/清理机制。

---

## 维度 11：14.11 权限测试覆盖率审计

### 检查方法

- Glob `/workspace/backend/tests/**/*.rs` 查找集成测试文件
- Grep `#\[tokio::test\]|#\[test\]` 在 `/workspace/backend/src/middleware/permission.rs` 查找单元测试
- Grep `permission|role|is_admin` 在 `/workspace/backend/tests` 查找权限相关集成测试
- 对照审计计划 14.11.1 权限测试缺口和 14.11.2 权限测试清单

### 发现

#### ✅ 已落实的项

1. **`permission.rs` 有 21 个单元测试**：`permission.rs:238-432` 覆盖 `extract_resource_info`/`method_to_action`/`CacheEntry`/`matches_permission` 函数，包括：
   - 标准路径无 ID/带 ID
   - 模块前缀路径无 ID/带 ID
   - 嵌套路径带 ID 和动作
   - 非 API 路径/短路径/空路径
   - GET/POST/PUT/PATCH/DELETE/OPTIONS 方法映射
   - CacheEntry 新建未过期/已过期
   - matches_permission 类型不匹配/全部匹配/action 通配符/ID 精确匹配/ID 不等/权限无 ID 请求有 ID/权限有 ID 请求无 ID/action 不匹配/通配符加 ID 精确匹配
2. **`admin_checker.rs` 有 2 个单元测试**：`admin_checker.rs:101-131` 覆盖 `AdminCacheEntry` 过期和 `clear_admin_role_cache` 清理。
3. **`auth_handler.rs` 有 4 个单元测试**：`auth_handler.rs:680-806` 覆盖 `LoginResponse` 序列化（不含 token/refresh_token、permissions 为字符串数组、字段白名单）。

#### ❌ 缺陷项

**缺陷 14.11-A：无非 admin 角色权限拒绝的集成测试**
- **风险等级：P1**
- **证据**：
  - Glob `/workspace/backend/tests/**/*.rs` 返回 40 个测试文件，无 `test_permission.rs`/`test_role.rs`/`test_auth.rs` 等权限相关集成测试
  - Grep `permission_denied|403|forbidden` 在 `/workspace/backend/tests` 中无匹配
- **业务影响**：审计计划 14.11.1 指出的"非 admin 角色权限拒绝"测试缺口确认存在。无法验证非 admin 用户访问受限资源时返回 403。
- **修复建议**：按审计计划 14.11.2 补充 `test_non_admin_denied_without_permission` 和 `test_non_admin_allowed_with_exact_permission` 集成测试。

**缺陷 14.11-B：无 is_system=true 注入 `*:*` 的测试**
- **风险等级：P1**
- **证据**：Grep `is_system.*\*:\*|build_with_permissions.*test` 在 `/workspace/backend/src/handlers/auth_handler.rs` 测试模块中无匹配
- **业务影响**：审计计划 14.11.1 指出的"is_system=true 注入 `*:*`"测试缺口确认存在。无法验证 manager/operator 登录后是否持有 `*:*`（修复后应不再持有）。
- **修复建议**：补充 `test_manager_not_system_after_fix` 和 `test_operator_not_system_after_fix` 测试。

**缺陷 14.11-C：无权限缓存失效的测试**
- **风险等级：P1**
- **证据**：Grep `cache_invalidation|permission_cache.*test` 在 `/workspace/backend/tests` 中无匹配
- **业务影响**：审计计划 14.11.1 指出的"权限缓存失效"测试缺口确认存在。无法验证权限变更后缓存立即失效。
- **修复建议**：补充 `test_permission_cache_invalidation` 测试。

**缺陷 14.11-D：无行级/字段级权限的测试**
- **风险等级：P2**
- **证据**：Grep `data_scope|field_permission.*test|apply_data_scope.*test` 在 `/workspace/backend/tests` 中无匹配
- **业务影响**：审计计划 14.11.1 指出的"行级数据权限"和"字段级权限"测试缺口确认存在（实现后需补测）。
- **修复建议**：实现行级/字段级权限后补充对应测试。

**缺陷 14.11-E：无角色互斥校验的测试**
- **风险等级：P2**
- **证据**：Grep `role_conflict.*test|conflicting_roles.*test` 在 `/workspace/backend/tests` 中无匹配
- **业务影响**：审计计划 14.11.1 指出的"角色互斥校验"测试缺口确认存在（实现后需补测）。
- **修复建议**：实现角色互斥校验后补充对应测试。

**缺陷 14.11-F：无通配符匹配的集成测试**
- **风险等级：P2**
- **证据**：`permission.rs:384-390` 有 `test_matches_permission_action通配符匹配` 单元测试，但无 `*:*` / `resource:*` / `*:action` 的集成测试
- **业务影响**：审计计划 14.11.1 指出的"权限通配符匹配"测试缺口部分存在。
- **修复建议**：补充通配符匹配的集成测试。

**缺陷 14.11-G：无公共路径白名单的测试**
- **风险等级：P3**
- **证据**：Grep `public_path.*test|is_public_path.*test` 在 `/workspace/backend/tests` 中无匹配
- **业务影响**：审计计划 14.11.1 指出的"公共路径白名单"测试缺口确认存在。
- **修复建议**：补充 `test_public_routes_no_auth_required` 测试。

**缺陷 14.11-H：无 require_admin_role 二次校验的测试**
- **风险等级：P2**
- **证据**：Grep `require_admin_role.*test` 在 `/workspace/backend/tests` 中无匹配
- **业务影响**：审计计划 14.11.1 指出的"require_admin_role 二次校验"测试缺口确认存在。
- **修复建议**：补充 `test_require_admin_role_rejects_non_admin` 测试。

---

## 维度 12：14.12 权限安全审计

### 检查方法

- Read `/workspace/backend/src/middleware/permission.rs`（权限校验逻辑）
- Read `/workspace/backend/src/utils/admin_checker.rs`（admin 角色检查）
- Read `/workspace/backend/src/handlers/auth_handler.rs:105-184`（权限注入逻辑）
- Read `/workspace/backend/src/middleware/public_routes.rs`（公共路径白名单）
- Read `/workspace/backend/src/utils/path_utils.rs`（模块前缀白名单）
- 对照审计计划 14.12.1-14.12.3

### 发现

#### ✅ 已落实的项

1. **`is_admin_role` fail-closed**：`admin_checker.rs:80-89` 数据库查询失败时返回 `false`（拒绝访问），防止系统未初始化时放行。
2. **`matches_permission` resource_id 精确匹配防垂直越权**：`permission.rs:231-235` `None` 匹配 `None`、`Some(pid)` 匹配 `Some(rid)` 且必须相等，防止垂直越权。
3. **`matches_permission` 权限无 ID 请求有 ID 返回 false**：`permission.rs:406-411` 测试 `test_matches_permission_权限无ID请求有ID返回false` 验证 M-6 修复点。
4. **权限查询使用 SeaORM 参数化防 SQL 注入**：`permission.rs:193-200` 使用 `QueryFilter` 参数化查询。
5. **公共路径白名单精确匹配**：`public_routes.rs` 中 `is_public_path` 使用精确匹配（审计计划 14.12.2 提到"已修复（精确匹配）"）。
6. **JWT 签名验证**：`auth_handler.rs:457-461` 使用 `AuthService::validate_token_static` 验证 JWT 签名。
7. **CSRF Token IP 绑定**：`auth_handler.rs:464-494` CSRF Token 绑定 IP 地址，防止跨站请求伪造。
8. **登录失败次数限制**：`auth_handler.rs:290-307` 实现 IP 级别（5 次）和用户名级别（10 次）失败次数限制，30 分钟锁定。
9. **强制轮换 CSRF Token**：`auth_handler.rs:473-481` 登录前清除该用户的旧 CSRF Token。

#### ❌ 缺陷项

**缺陷 14.12-A：is_system 注入 `*:*` 导致权限提升**
- **风险等级：P0**
- **证据**：`auth_handler.rs:129-131`（见缺陷 14.5-A 详细证据）
- **业务影响**：审计计划 14.12.1 指出的"is_system 注入 `*:*`"漏洞确认存在。manager/operator 登录后注入超级权限，权限提升为 admin。
- **修复建议**：按缺陷 14.5-A 修复方案，仅 admin 注入 `*:*`。

**缺陷 14.12-B：模块前缀不在白名单时资源类型提取错误（可能导致权限绕过）**
- **风险等级：P1**
- **证据**：`permission.rs:95-99`（见缺陷 14.4-D 详细证据）
- **业务影响**：审计计划 14.12.2 指出的"模块前缀不在白名单"问题确认存在。对于 `production`/`analytics`/`system` 等模块前缀，`extract_resource_info` 错误提取资源类型为模块名（如 `production`），若 `role_permissions` 表中恰好有 `production:read` 权限（目前没有），则可能放行不该放行的请求。当前因权限表中无这些资源类型，实际是 fail-closed（拒绝），但存在潜在风险。
- **修复建议**：按审计计划 14.12.2 补齐白名单 + None 时 fail-closed。

**缺陷 14.12-C：`extract_resource_info` 返回 "unknown" 时 fail-closed 但未明确拒绝**
- **风险等级：P2**
- **证据**：`permission.rs:114-116`
  ```rust
  } else {
      ("unknown".to_string(), None)
  };
  ```
  对于非 `/api/v1/erp/` 前缀的路径，返回 `("unknown", None)`，`check_permission` 查询 `role_permissions` 表无 `unknown` 资源类型时返回 `false`（拒绝）。
- **业务影响**：当前是 fail-closed（拒绝），但 `unknown` 资源类型不够明确，可能掩盖配置错误。
- **修复建议**：将 `unknown` 改为显式拒绝并记录告警日志，便于排查配置错误。

**缺陷 14.12-D：HTTP 方法未映射 OPTIONS/HEAD 等非标准方法**
- **风险等级：P3**
- **证据**：`permission.rs:119-128` `method_to_action` 函数将 OPTIONS 等未明确映射的方法默认为 `read`。
  ```rust
  _ => "read",
  ```
- **业务影响**：审计计划 14.12.2 指出的"HTTP 方法未映射"问题部分存在。OPTIONS/HEAD 等方法被映射为 `read`，可能放行不该放行的预检请求。但 OPTIONS 通常是 CORS 预检请求，不携带敏感数据，风险较低。
- **修复建议**：按审计计划 14.12.2 明确拒绝或映射为 `read`（当前实现），并记录日志。

**缺陷 14.12-E：role.code 可被修改导致权限提升**
- **风险等级：P1**
- **证据**：`role_service.rs:95-96` `update_role` 函数允许修改 `code` 字段
  ```rust
  if let Some(code) = code {
      role_active.code = Set(code);
  }
  ```
  虽然 `role_handler.rs:267` 调用 `require_admin_role` 限制仅 admin 可修改角色，但 admin 可将某角色的 `code` 改为 `admin`，使该角色获得 admin 权限。
- **业务影响**：审计计划 14.12.1 指出的"require_admin_role 绕过：若 role.code 被篡改为 'admin' 则绕过"漏洞确认存在。
- **修复建议**：
  1. `role.code` 不可修改（`update_role` 移除 `code` 字段更新）
  2. 数据库增加 `roles.code` 唯一约束（防止重复 `admin`）
  3. 增加 `code` 修改的特殊审批流程

**缺陷 14.12-F：权限缓存无 Redis 权限隔离**
- **风险等级：P2**
- **证据**：`permission.rs:156` `PERMISSION_CACHE` 使用进程内 `DashMap`，无 Redis 权限隔离
- **业务影响**：审计计划 14.12.3 指出的"Redis 缓存投毒"风险在当前进程内缓存实现下不存在，但若未来迁移到 Redis 共享缓存，需考虑权限隔离。
- **修复建议**：迁移到 Redis 时实现 Redis ACL + 缓存签名。

---

## 审计结果汇总

| 维度 | P0 | P1 | P2 | P3 | 已落实 | 总检查项 |
|------|----|----|----|----|--------|----------|
| 14.1 角色清单合理性 | 2 | 0 | 1 | 1 | 4 | 8 |
| 14.2 权限分配矩阵 | 2 | 1 | 1 | 0 | 4 | 8 |
| 14.3 职责分离 SoD | 3 | 1 | 0 | 0 | 3 | 7 |
| 14.4 权限-路由匹配 | 2 | 2 | 0 | 0 | 4 | 8 |
| 14.5 is_system 滥用治理 | 2 | 0 | 1 | 0 | 4 | 7 |
| 14.6 前后端权限边界一致性 | 1 | 0 | 2 | 0 | 6 | 9 |
| 14.7 业务角色权限矩阵设计 | 3 | 1 | 0 | 0 | 2 | 6 |
| 14.8 权限粒度（行级/字段级） | 1 | 1 | 2 | 0 | 6 | 10 |
| 14.9 权限缓存与性能 | 2 | 1 | 0 | 1 | 6 | 10 |
| 14.10 权限审计日志与合规审查 | 1 | 2 | 1 | 0 | 5 | 9 |
| 14.11 权限测试覆盖率 | 0 | 3 | 4 | 1 | 3 | 11 |
| 14.12 权限安全审计 | 1 | 2 | 2 | 1 | 9 | 15 |
| **合计** | **20** | **14** | **15** | **5** | **56** | **108** |

---

## 修复优先级队列

### P0（阻塞，20 项）

1. **缺陷 14.1-A**：manager/operator 角色被错误标记为 is_system=true（`init_service.rs:386, 400`）
2. **缺陷 14.1-B**：14 类面料行业业务专用角色完全缺失（`init_service.rs:342-418`）
3. **缺陷 14.2-A**：manager/operator 持有 `*:*` 超级通配权限（`auth_handler.rs:129-131`）
4. **缺陷 14.2-B**：非 admin 角色在 60+ 类资源上无法通过权限检查（`init_admin_permissions.sql` 仅 11 类）
5. **缺陷 14.3-A**：角色互斥表（role_conflict）完全未实现
6. **缺陷 14.3-B**：用户角色分配无互斥校验
7. **缺陷 14.3-C**：财务开凭证+审核凭证+删凭证三权未分立（`init_admin_permissions.sql:24-28`）
8. **缺陷 14.4-A**：模块前缀白名单严重不足（28 个 vs 实际 50+ 个）（`path_utils.rs:1-31`）
9. **缺陷 14.4-B**：60+ 类资源缺失权限定义（`init_admin_permissions.sql` 仅 11 类）
10. **缺陷 14.5-A**：`build_with_permissions` 仅检查 is_system 未检查 role.code == "admin"（`auth_handler.rs:129-131`）
11. **缺陷 14.5-B**：manager/operator 的 is_system=true 未修复（`init_service.rs:386, 400`）
12. **缺陷 14.6-A**：前端 `*:*` 放行与后端 `is_admin_role` 拒绝的不一致
13. **缺陷 14.7-A**：14 类业务角色的权限矩阵完全未实现
14. **缺陷 14.7-C**：销售域角色权限矩阵未实现
15. **缺陷 14.7-D**：采购域/库存域/生产域/财务域角色权限矩阵未实现
16. **缺陷 14.8-A**：行级数据权限（apply_data_scope）完全未实现
17. **缺陷 14.9-A**：权限变更后缓存不主动失效
18. **缺陷 14.9-B**：用户禁用后缓存不主动失效
19. **缺陷 14.10-A**：`permission_change_audit` 表完全未实现
20. **缺陷 14.12-A**：is_system 注入 `*:*` 导致权限提升（`auth_handler.rs:129-131`）

### P1（高，14 项）

1. **缺陷 14.2-C**：admin 持有 audit:read 违反职责分离
2. **缺陷 14.3-D**：采购/销售审批与创建未分离
3. **缺陷 14.4-C**：权限码与路由资源类型不匹配
4. **缺陷 14.4-D**：模块前缀不在白名单时资源类型提取错误
5. **缺陷 14.7-B**：业务角色无 dashboard:read 权限
6. **缺陷 14.8-B**：字段级权限种子数据完全为空
7. **缺陷 14.9-C**：权限缓存无 Redis pub/sub 热更新
8. **缺陷 14.10-B**：异常权限分配识别规则未实现
9. **缺陷 14.10-C**：定期合规审查机制未实现
10. **缺陷 14.11-A**：无非 admin 角色权限拒绝的集成测试
11. **缺陷 14.11-B**：无 is_system=true 注入 `*:*` 的测试
12. **缺陷 14.11-C**：无权限缓存失效的测试
13. **缺陷 14.12-B**：模块前缀不在白名单时资源类型提取错误（可能导致权限绕过）
14. **缺陷 14.12-E**：role.code 可被修改导致权限提升

### P2（中，15 项）

1. **缺陷 14.1-C**：审计计划提及的 014_init_role_permissions.sql 实际不存在
2. **缺陷 14.2-D**：权限过大/过小检测规则未实现
3. **缺陷 14.5-C**：is_system 字段无数据库约束
4. **缺陷 14.6-B**：前端权限码单复数不一致（文档示例）
5. **缺陷 14.6-C**：前端仅控制 `:read` 路由可见性，未控制写按钮可见性
6. **缺陷 14.8-C**：`FieldPermissionService::filter_fields_by_read_permission` 未接入业务
7. **缺陷 14.8-D**：`data_scope` 模块仅保留 ALL 常量
8. **缺陷 14.10-D**：审计日志保留期限未配置
9. **缺陷 14.11-D**：无行级/字段级权限的测试
10. **缺陷 14.11-E**：无角色互斥校验的测试
11. **缺陷 14.11-F**：无通配符匹配的集成测试
12. **缺陷 14.11-H**：无 require_admin_role 二次校验的测试
13. **缺陷 14.12-C**：`extract_resource_info` 返回 "unknown" 时 fail-closed 但未明确拒绝
14. **缺陷 14.12-F**：权限缓存无 Redis 权限隔离

### P3（低，5 项）

1. **缺陷 14.1-D**：角色命名规范未校验
2. **缺陷 14.9-D**：权限缓存仅 5 分钟 TTL，无法配置
3. **缺陷 14.11-G**：无公共路径白名单的测试
4. **缺陷 14.12-D**：HTTP 方法未映射 OPTIONS/HEAD 等非标准方法
5. （无）

---

## 关键发现总结

### 1. 权限模型形同虚设（P0 级核心问题）

当前权限系统的核心问题是 **manager/operator 角色被错误标记为 `is_system=true`**，导致登录后被注入 `*:*` 超级通配权限，等同于 admin。这使得整个权限模型形同虚设：

- `init_service.rs:386, 400` 创建 manager/operator 时 `is_system=true`
- `auth_handler.rs:129-131` `build_with_permissions` 仅检查 `is_system` 未检查 `role.code == "admin"`，为所有 `is_system=true` 的角色注入 `*:*`
- 前端 `router/index.ts:864` `hasRoutePermission` 支持 `*:*` 超级通配符，放行所有路由
- 后端 `admin_checker.rs:77-78` `is_admin_role` 严格检查 `role.code == "admin"`，不承认 manager/operator 为 admin

这导致 **前端可见所有菜单但后端角色管理接口返回 403** 的不一致体验（见缺陷 14.6-A）。

### 2. 业务角色完全缺失（P0 级核心问题）

面料行业 ERP 应有 14 类业务角色（销售经理/销售员/采购经理/采购员/仓库经理/仓库员/生产经理/染色操作员/化验室员/色卡管理员/财务会计/财务审核/HR/审计员），当前完全缺失。所有非 admin 用户被迫分配 manager/operator 角色，间接获得 `*:*` 超级权限。

### 3. 权限-路由匹配严重不足（P0 级核心问题）

- `init_admin_permissions.sql` 仅定义 11 类资源权限
- 后端路由实际暴露 70+ 类资源
- 缺口约 60 类，非 admin 用户在 60+ 类资源上无法通过权限检查
- `path_utils.rs` 模块前缀白名单仅 28 个，无法覆盖新增的 production/crm/analytics 等模块

### 4. 职责分离（SoD）完全未实现（P0 级核心问题）

- `role_conflict` 表完全未实现
- 用户角色分配无互斥校验
- 财务开凭证+审核凭证+删凭证三权未分立
- 采购/销售审批与创建未分离

### 5. 行级数据权限完全未实现（P0 级核心问题）

- `apply_data_scope` 函数完全未实现
- `AuthContext` 无 `customer_id/warehouse_id/department_id` 字段
- 销售员可看全部订单、采购员可看全部采购单、客户可看全部订单等数据泄露风险

### 6. 权限缓存不主动失效（P0 级核心问题）

- 权限变更后缓存不主动失效，5 分钟内仍使用旧权限
- 用户禁用后缓存不主动失效，JWT 有效期内仍可访问
- 无 Redis pub/sub 热更新，多实例部署时缓存不一致

### 7. 权限审计与合规审查未实现（P0 级核心问题）

- `permission_change_audit` 表完全未实现
- 异常权限分配识别规则未实现
- 定期合规审查机制未实现
- 审计日志保留期限未配置

### 8. 权限测试覆盖率严重不足

- 无权限相关集成测试文件（`test_permission.rs`/`test_role.rs`/`test_auth.rs` 等）
- 21 个 `permission.rs` 单元测试覆盖纯函数，但无端到端权限校验测试
- 缺失非 admin 权限拒绝、is_system 注入、缓存失效、行级/字段级权限、角色互斥、通配符匹配、公共路径、require_admin_role 等关键测试场景

---

## 修复路线图建议

### 阶段 1：紧急修复 P0（1-2 周）

1. 修复 `is_system` 滥用：manager/operator 改为 `is_system=false`（缺陷 14.1-A、14.5-B）
2. 修复 `build_with_permissions`：仅 admin 注入 `*:*`（缺陷 14.2-A、14.5-A、14.12-A）
3. 补齐模块前缀白名单至 50+ 个（缺陷 14.4-A、14.4-D）
4. 补齐 60+ 类资源的权限定义（缺陷 14.4-B）
5. 实现 `role_conflict` 表和互斥校验（缺陷 14.3-A、14.3-B）

### 阶段 2：业务角色补齐（2-3 周）

1. 补齐 14 类业务角色定义（缺陷 14.1-B）
2. 为 14 类业务角色编写权限种子（缺陷 14.7-A、14.7-C、14.7-D）
3. 实现财务三权分立（缺陷 14.3-C）
4. 实现采购/销售审批分离（缺陷 14.3-D）

### 阶段 3：权限粒度与缓存（3-4 周）

1. 实现行级数据权限 `apply_data_scope`（缺陷 14.8-A）
2. 补齐字段级权限种子数据（缺陷 14.8-B）
3. 实现权限缓存主动失效（缺陷 14.9-A、14.9-B）
4. 实现 Redis pub/sub 热更新（缺陷 14.9-C）

### 阶段 4：审计与测试（4-5 周）

1. 实现 `permission_change_audit` 表（缺陷 14.10-A）
2. 实现异常权限分配识别规则（缺陷 14.10-B）
3. 实现定期合规审查机制（缺陷 14.10-C）
4. 补齐权限相关集成测试（缺陷 14.11-A 至 14.11-H）

---

**审计完成时间**：2026-07-16
**审计子代理**：V15 审计子代理（类十四权限维度审计与角色合理性）
**报告文件**：`/workspace/.monkeycode/docs/audits/v15/batch-12/audit-report.md`

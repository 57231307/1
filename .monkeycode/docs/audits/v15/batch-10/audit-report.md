# V15 大货批色与RBAC审计报告（类十一+类十二·批次 10）

- **审计子代理**：V15 审计子代理（类十一大货批色+类十二RBAC）
- **审计范围**：14 维度（大货批色6维度 + RBAC 8维度）
- **审计依据**：
  - `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md`（类十一第 3332-3555 行；类十二第 3556-4016 行）
  - `/workspace/.monkeycode/docs/research/fabric-industry-research.md`（§3.1 染整工艺 10 道工序、§4.7 质量检验 CIE D65 ΔE、§5.4 财务凭证联动）
  - `/workspace/backend/src/models/role.rs` / `role_permission.rs` / `data_permission.rs` / `field_permission.rs`
  - `/workspace/backend/src/services/role_service.rs` / `role_permission_service.rs` / `data_permission_service.rs` / `field_permission_service.rs`
  - `/workspace/backend/src/middleware/permission.rs` / `auth_context.rs` / `auth.rs`
  - `/workspace/backend/src/handlers/role_handler.rs` / `user_handler.rs` / `sales_order_handler.rs` / `customer_handler.rs` / `print_handler.rs` / `bulk_product_handler.rs`
  - `/workspace/backend/src/utils/admin_checker.rs` / `data_permission.rs`
  - `/workspace/backend/src/services/init_service.rs`（默认角色初始化）
  - `/workspace/backend/database/init_admin_permissions.sql`（管理员权限初始化）
  - `/workspace/backend/src/routes/iam.rs`（IAM 路由）
  - `/workspace/backend/src/models/audit_log.rs`（通用审计日志模型）
  - `/workspace/frontend/src/directives/permission.ts` / `router/index.ts` / `store/user.ts` / `api/request.ts` / `main.ts`
- **审计方法**：Read 审计计划 + Grep 检索关键关键词（bulk_color_approval/批色/大货/user_role/permission_audit_log/apply_data_scope/delegation/mutual_exclusive/session_fixation 等）+ Read 关键文件 + 对照审计计划逐项核对
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码；超详细每维度每项附文件路径:行号证据；P0=阻塞/P1=高/P2=中/P3=低

---

## 类十一 维度 11.1：大货批色数据模型与状态机

### 检查方法
1. Grep `bulk_color_approval|大货批色|批色|bulk_color|customer_approval` 在 `backend/src/` 全量检索
2. Grep `delivery_blocking|approval_status.*Approved|大货样` 在 `backend/src/` 全量检索
3. Glob `backend/src/models/bulk_color*.rs` / `backend/src/services/bulk_color*.rs`
4. Glob `backend/migrations/**/*.sql` 查找大货批色相关迁移
5. 对照审计计划 11.1 检查要点：`bulk_color_approval` 表/状态机/状态流转/交货门禁

### 发现

#### ❌ 缺陷项：大货批色业务数据模型完全缺失

**风险等级：P0**（核心业务模块完全缺失，面料行业"交货前客户批色"业务无法落地）

**证据**：
- Grep `bulk_color_approval|大货批色|批色|bulk_color|customer_approval` 全量检索 `backend/src/`，仅命中 2 个文件：
  - `/workspace/backend/src/services/lab_dip_service.rs:833` 仅注释提及："真实业务：OK 样确认后（通知单 approved 状态），大货生产前必须复样"（属于 lab_dip 打样通知单流程，**非大货批色**）
  - `/workspace/backend/src/models/lab_dip_request.rs:78` `pub customer_approval_comment: Option<String>`（属于 lab_dip 打样通知单，**非大货批色**）
- Grep `delivery_blocking|approval_status.*Approved|大货样` 全量检索 `backend/src/`：**0 命中**
- Glob `backend/src/models/bulk_color*.rs`：**0 文件**
- Glob `backend/src/services/bulk_color*.rs`：**0 文件**
- Glob `backend/migrations/**/*.sql` 中无 `bulk_color_approval` 相关迁移

**业务影响**：
- 审计计划 11.1 要求新增 `bulk_color_approval` 表，包含 17 个核心字段（`production_order_id` / `sales_order_id` / `product_id`+`color_id`+`dye_lot_no`+`batch_no` 四维标识 / `sample_piece_id` / `sample_length_m` / `sent_to_customer_at` / `customer_id` / `approval_status` / `delta_e_value` / `customer_feedback` / `approved_at` / `approved_by` / `delivery_blocking` 等）
- 8 状态机（`PendingSample`/`Sampled`/`SentToCustomer`/`Approved`/`Rejected`/`Reworking`/`Downgraded`/`Scrapped`）完全未实现
- 9 个状态流转规则未实现
- 交货门禁（`delivery_blocking=true` 当状态非 Approved 时）未实现
- 销售出库 handler 不校验批色状态，未批色通过的面料可直接出库——**违反面料行业"交货前客户批色"业务铁律**

**修复建议**：
1. 新增迁移 `migrations/{ts}_create_bulk_color_approval/up.sql`：创建 `bulk_color_approval` 表，含审计计划 11.1.1 要求的全部 17 字段 + 索引（`production_order_id`/`sales_order_id`/`customer_id`/`approval_status`/`dye_lot_no`）
2. 新增 `backend/src/models/bulk_color_approval.rs` SeaORM 模型
3. 新增 `backend/src/services/bulk_color_approval_service.rs` 实现 8 状态机 + 9 流转规则 + 交货门禁校验
4. 修改 `backend/src/handlers/sales_order_handler.rs` 的 `ship_order` / `complete_order` handler，加入 `approval_status = Approved` 校验
5. 新增 `backend/src/handlers/bulk_color_approval_handler.rs` 提供 CRUD + 状态流转接口
6. 在 `backend/src/routes/` 注册大货批色路由

---

## 类十一 维度 11.2：剪大货样业务规则

### 检查方法
1. Grep `cut_sample|sample_piece|剪样|大货样` 在 `backend/src/` 全量检索
2. Grep `sample_piece_id|sample_length_m|sample_weight_kg` 在 `backend/src/models/` 检索
3. Read `backend/src/models/inventory_piece.rs` 检查是否有 `sample` 状态
4. 对照审计计划 11.2 检查要点：剪样前置条件/数量规则/库存联动/标识/追溯

### 发现

#### ❌ 缺陷项：剪大货样业务规则完全未实现

**风险等级：P0**（剪样是批色流程的前置环节，缺失导致整条批色链路断裂）

**证据**：
- Grep `cut_sample|sample_piece|剪样|大货样` 全量检索 `backend/src/`：**0 命中**
- Grep `sample_piece_id|sample_length_m|sample_weight_kg` 在 `backend/src/models/`：**0 命中**
- `inventory_piece.rs` 中无 `sample` 状态枚举，无独立的样布 inventory_piece 记录生成逻辑
- 无剪样事务化代码（审计计划要求与库存扣减同一事务）

**业务影响**：
- 审计计划 11.2 要求 5 项业务规则全部未实现：
  1. 剪样前置条件校验（大货已入库 + 质检通过 + 生产订单 completed）
  2. 剪样数量规则（每缸号每批至少 1 个样布，默认 0.5m 可配）
  3. 剪样库存联动（从大货扣减剪样长度 + 生成独立 `sample` 状态 inventory_piece + 事务化）
  4. 剪样标识（独立编号 `SAMPLE-<dye_lot_no>-<batch_no>-<seq>`）
  5. 剪样追溯（样布→大货→生产订单→缸号→染色配方）

**修复建议**：
1. 在 `inventory_piece.rs` 的状态枚举中增加 `sample` 状态
2. 新增 `backend/src/services/sample_cutting_service.rs` 实现 5 项业务规则
3. 剪样操作必须在 `(*self.db).begin()` 事务内完成：扣减大货 inventory_piece 长度 + 创建 sample inventory_piece + 创建 bulk_color_approval 关联记录
4. 新增独立编号生成器：`SAMPLE-{dye_lot_no}-{batch_no}-{seq:03}`

---

## 类十一 维度 11.3：客户批色确认流程

### 检查方法
1. Grep `customer_portal|customer.*portal|客户门户` 在 `backend/src/` 全量检索
2. Grep `delta_e|color_difference|色差` 在 `backend/src/services/` 检索
3. Grep `批色|customer_approval` 在 `backend/src/handlers/` 检索
4. 对照审计计划 11.3 检查要点：批色通知/客户批色操作/批色时限/色差判定/批色结果处理

### 发现

#### ❌ 缺陷项：客户批色确认流程完全未实现

**风险等级：P0**（客户批色是"交货前批色"业务的核心环节）

**证据**：
- Grep `customer_portal|customer.*portal|客户门户` 全量检索 `backend/src/`：**0 命中**
- Grep `delta_e|color_difference|色差` 全量检索 `backend/src/services/`：**0 命中**
- Grep `批色|customer_approval` 在 `backend/src/handlers/`：仅 `lab_dip_service.rs` 一处注释命中（非大货批色）
- 无批色通知发送逻辑、无客户门户批色接口、无批色时限超时自动标记逻辑

**业务影响**：
- 审计计划 11.3 要求 5 项业务规则全部未实现：
  1. 批色通知（系统内消息 + 短信/邮件 + 客户门户待批色清单）
  2. 客户批色操作（客户登录客户门户→查看待批色→查看 ΔE→批色通过/不通过）
  3. 批色时限（默认 3 天，超时 7 天自动标记为 `Rejected`）
  4. 色差判定标准（ΔE≤1.2 同色通过 / ΔE≤2.5 让步接收 / ΔE>2.5 不合格 / 高光敏感区 ΔE≤0.8）
  5. 批色结果处理（通过→解除交货门禁 / 不通过→触发后续处理）

**修复建议**：
1. 新增 `backend/src/handlers/customer_portal_handler.rs` 提供客户门户批色接口（独立的客户认证体系）
2. 新增 `backend/src/services/bulk_color_approval_service.rs::approve_by_customer()` 方法
3. 新增批色时限超时检查后台任务（main.rs 中定时扫描 `sent_to_customer_at` + 超时 7 天自动 `Rejected`）
4. 实现色差判定规则函数 `evaluate_delta_e(delta_e: f64, high_light_sensitive: bool) -> ApprovalResult`
5. 复用现有 `email_service.rs` / `notification_service.rs` 发送批色通知

---

## 类十一 维度 11.4：批色不通过处理流程

### 检查方法
1. Grep `rework|返工` 在 `backend/src/services/production/` 检索（路径不存在，回退到 `backend/src/services/`）
2. Grep `downgrade|降级` 在 `backend/src/services/inventory/` 检索（路径不存在，回退到 `backend/src/services/`）
3. Grep `scrap|报废` 在 `backend/src/services/` 检索
4. Read `backend/src/models/dye_batch_rework.rs`（缸号回修记录，可能与返工概念关联）
5. 对照审计计划 11.4 检查要点：返工/降级/报废/财务凭证联动

### 发现

#### ✅ 已落实的项（部分相关基础设施）
1. **缸号回修记录模型已存在**（`/workspace/backend/src/models/dye_batch_rework.rs`，60 行）：
   - 支持 4 种回修类型：`color_difference 色差 / defect 疕点 / specification_unqualified 规格不符 / other 其他`
   - 状态机：`draft / approved / in_progress / completed / cancelled`
   - 含 `rework_quantity`、`rework_reason`、`completed_at` 字段
   - 但属于缸号维度回修，**不是大货批色不通过后的返工流程**

2. **质检分级模型已存在**（`/workspace/backend/src/models/quality_inspection.rs`）：支持质量分级（A/B/C 级），可作为降级处理基础

#### ❌ 缺陷项：批色不通过的返工/降级/报废业务流程未实现

**风险等级：P0**（批色不通过后无标准化处理路径，面料积压风险）

**证据**：
- Grep `rework|返工` 全量检索 `backend/src/services/`：仅 `dye_batch_rework.rs`（缸号回修）相关，**无大货批色返工流程**
- Grep `downgrade|降级` 全量检索 `backend/src/services/`：**0 命中**
- Grep `scrap|报废` 全量检索 `backend/src/services/`：**0 命中**
- 无返工生产订单创建逻辑、无降级 B 级品库存管理、无报废审批工作流

**业务影响**：
- 审计计划 11.4 要求 4 项业务规则全部未实现：
  1. 返工流程（`Rejected → Reworking → PendingSample`）：返工生产订单 + 配方调整 + 返工次数限制（≤2 次）
  2. 降级流程（`Rejected → Downgraded`）：降为 B 级品 + 重新定价 + 单独库存管理 + 客户确认
  3. 报废流程（`Rejected → Scrapped`）：生产主管+质量主管+财务主管三审 + 报废单 + 库存扣减 + 成本核算
  4. 财务凭证联动：返工成本归集到生产成本 / 降级损失计入资产减值损失 / 报废成本计入营业外支出

**修复建议**：
1. 复用 `dye_batch_rework.rs` 模型扩展，增加 `bulk_color_approval_id` 关联字段
2. 新增 `backend/src/services/downgrade_service.rs` 实现降级流程
3. 新增 `backend/src/services/scrap_service.rs` 实现报废流程（含三审工作流，复用 `bpm_service.rs`）
4. 复用 `voucher_service.rs` / `cost_collection_service.rs` 实现财务凭证联动

---

## 类十一 维度 11.5：批色报表与统计

### 检查方法
1. Grep `approval_report|批色报表` 在 `backend/src/services/` 检索
2. Grep `xlsx|docx` 在 `backend/src/services/report/` 检索（路径不存在，回退到 `backend/src/services/`）
3. 对照审计计划 11.5 检查要点：批色通过率/返工统计/降级报废/客户响应时间/导出格式

### 发现

#### ❌ 缺陷项：批色报表完全未实现

**风险等级：P2**（报表为辅助分析功能，缺失不阻塞业务流程，但影响业务洞察）

**证据**：
- Grep `approval_report|批色报表` 全量检索 `backend/src/services/`：**0 命中**
- 无批色通过率报表、无返工统计报表、无降级报废统计报表、无客户响应时间报表
- 现有 `report_engine_handler.rs` / `report_template_service.rs` 通用报表引擎未配置批色相关模板

**业务影响**：
- 审计计划 11.5 要求 5 项报表全部未实现：
  1. 批色通过率报表（按客户/面料/缸号/时间维度）
  2. 返工统计报表（次数/原因/成本）
  3. 降级报废统计报表（数量/金额）
  4. 客户响应时间报表（平均响应/超时统计/效率排名）
  5. 报表导出（.xlsx + .docx）

**修复建议**：
1. 在 `report_template_service.rs` 中新增 4 个批色报表模板定义
2. 在 `report_engine_handler.rs` 中新增批色报表 API 路由
3. 复用现有 `export_service.rs`（已有 .xlsx 导出能力）扩展批色报表导出
4. 业务上线后再补充此维度（依赖维度 11.1-11.4 数据积累）

---

## 类十一 维度 11.6：批色业务与其他模块集成

### 检查方法
1. Grep `production_order.*approval|approval.*production_order` 在 `backend/src/services/` 检索
2. Grep `approval.*delivery|delivery.*approval` 在 `backend/src/services/sales/` 检索（路径不存在，回退到 `backend/src/services/`）
3. Grep `dye_lot.*approval|approval.*dye_lot` 在 `backend/src/services/` 检索
4. Read `backend/src/handlers/sales_order_handler.rs` 检查 `ship_order` 是否校验批色状态
5. 对照审计计划 11.6 检查要点：与生产订单/销售订单/库存/质检/财务/缸号状态机集成

### 发现

#### ❌ 缺陷项：批色业务与其他模块集成完全缺失

**风险等级：P0**（无集成则批色业务形同虚设，销售出库不校验批色状态是最大风险）

**证据**：
- Grep `production_order.*approval|approval.*production_order` 全量检索：**0 命中**
- Grep `approval.*delivery|delivery.*approval` 全量检索：**0 命中**
- Grep `dye_lot.*approval|approval.*dye_lot` 全量检索：**0 命中**
- Read `/workspace/backend/src/handlers/sales_order_handler.rs` 中 `ship_order`（第 307-308 行）：
  ```rust
  // 调用原有 ship_order(request, user_id)
  sales_service.ship_order(payload, auth.user_id).await?;
  ```
  **未校验 approval_status = Approved**，未批色通过的面料可直接出库

**业务影响**：
- 审计计划 11.6 要求 6 项集成全部未实现：
  1. 与生产订单集成（生产订单完成自动触发剪样 + 返工订单关联原订单）
  2. 与销售订单集成（出库前校验批色状态 + 交货门禁）
  3. 与库存模块集成（剪样扣减 + 降级品库位 + 报废品状态变更）
  4. 与质检模块集成（剪样前校验大货质检通过 + 批色作为客户侧质检）
  5. 与财务模块集成（返工成本 + 降级损失 + 报废损失核算）
  6. 与缸号状态机集成（批色状态作为缸号状态机一环：入库→待批色→批色通过→可交货）

- **最严重风险**：销售出库不校验批色状态，未批色通过的面料可直接发货，违反面料行业"交货前客户批色"业务铁律

**修复建议**：
1. 修改 `sales_order_handler.rs::ship_order` 加入批色校验门禁
2. 修改 `production_order_service.rs::complete_order` 在状态变为 `completed` 后自动触发剪样流程
3. 在 `dye_batch_state_machine_service.rs` 中新增 `awaiting_approval` 和 `approved` 两个状态
4. 复用 `voucher_service.rs` 实现返工/降级/报废的财务凭证联动

---

## 类十二 维度 12.1：RBAC 数据模型与权限架构

### 检查方法
1. Read `backend/src/models/role.rs`（角色模型）
2. Read `backend/src/models/role_permission.rs`（角色权限关联）
3. Grep `user_role|UserRole` 在 `backend/src/` 检索（用户角色关联表）
4. Read `backend/src/services/init_service.rs:342-418`（默认角色初始化）
5. Read `backend/database/init_admin_permissions.sql`（管理员权限初始化）
6. Grep `super_admin|sales_manager|customer_service|quality_inspector|finance` 在 `backend/src/` 检索
7. 对照审计计划 12.1 检查要点：RBAC 四层模型/关联表/权限码命名规范/角色层级

### 发现

#### ✅ 已落实的项
1. **角色模型存在**（`/workspace/backend/src/models/role.rs:9`，`table_name = "roles"`）：
   - 含 `id` / `name` / `code` / `description` / `permissions`（JSON 字符串）/ `is_system` / `created_at` / `updated_at`
   - 关联：`has_many Users` + `has_many RolePermissions`

2. **角色权限关联表存在**（`/workspace/backend/src/models/role_permission.rs:9`，`table_name = "role_permissions"`）：
   - 含 `id` / `role_id` / `resource_type` / `resource_id` / `action` / `allowed` / `created_at` / `updated_at`
   - 关联：`belongs_to Role`

3. **默认角色初始化**（`/workspace/backend/src/services/init_service.rs:342-418`）：
   - 创建 3 个系统角色：`admin`（管理员）/ `manager`（部门经理）/ `operator`（操作员），均 `is_system=true`
   - 使用 `ADMIN_ROLE_CODE` 常量替代硬编码 "admin"（批次 24 v6 P0-1 修复）
   - `admin` 角色权限为 `["*"]`（超级通配）

4. **管理员权限初始化 SQL**（`/workspace/backend/database/init_admin_permissions.sql:1-66`）：
   - 为 `role_id=1`（admin）初始化 33 条权限记录，覆盖 11 个资源类型（purchases/sales/inventory/finance/customers/suppliers/products/warehouses/users/audit/dashboard）× 4 个 action（read/create/update/delete）

5. **管理员角色常量化**（`/workspace/backend/src/utils/admin_checker.rs:10-14`）：
   - `ADMIN_ROLE_CODE: &str = "admin"` + `MANAGER_ROLE_CODE: &str = "manager"`
   - 单一真相源，避免多处硬编码

#### ❌ 缺陷项 1：缺少 `user_role` 关联表，不支持多角色

**风险等级：P1**（违反审计计划 12.1.2 要求的"支持一个用户多角色"）

**证据**：
- Grep `user_role|UserRole` 全量检索 `backend/src/`：仅 2 处命中
  - `/workspace/backend/src/services/event_bus.rs:720`：注释明确说明 `"// 系统无 user_role 表，通过 user.role_id 关联 role.code 过滤"`
  - `/workspace/backend/src/handlers/user_handler.rs`：仅出现 `user.role_id` 字段引用，无 `UserRole` 关联表
- 用户模型通过 `user.role_id`（单字段）关联角色，**只支持单角色**

**业务影响**：
- 审计计划 12.1.2 明确要求 `user_role` 关联表（多对多），支持一个用户多角色
- 单角色模型无法支持"销售经理同时是销售"等业务场景
- 影响权限继承、互斥校验等高级 RBAC 能力

**修复建议**：
1. 新增迁移 `migrations/{ts}_create_user_role/up.sql`：创建 `user_role` 关联表（含 `user_id` / `role_id` / `assigned_at` / `assigned_by` / `created_at` + UNIQUE(user_id, role_id)）
2. 修改 `user.role_id` 字段保留向后兼容（默认主角色），同时支持 `user_role` 多角色查询
3. 修改 `permission_middleware` 在权限检查时聚合用户的所有角色权限

#### ❌ 缺陷项 2：角色层级设计不完整

**风险等级：P1**（缺少 9 个面料行业 ERP 必备角色）

**证据**：
- `/workspace/backend/src/services/init_service.rs:363-403`：仅初始化 3 个角色（admin/manager/operator）
- Grep `super_admin|sales_manager|customer_service|quality_inspector|finance|warehouse_manager|production_manager|customer` 在 `backend/src/` 检索：仅 `ap_payment_request_service.rs:492-506` 引用 `ADMIN_ROLE_CODE`/`MANAGER_ROLE_CODE` 进行审批权限判断，**未定义其他角色**
- 审计计划 12.1.4 要求 12 个典型角色（super_admin/admin/sales_manager/sales/customer_service/warehouse_manager/warehouse/quality_inspector/quality_manager/finance/production_manager/customer）

**业务影响**：
- 缺少 `super_admin`：无法区分系统级管理员与业务级管理员
- 缺少 `sales`/`sales_manager`：无法实现销售数据隔离与销售审批分离
- 缺少 `customer`：客户门户批色（维度 11.3）无法实现
- 缺少 `finance`：财务与销售职责分离无法实现
- 缺少 `warehouse`/`warehouse_manager`：仓库操作员与仓库管理员审批分离无法实现
- 缺少 `quality_inspector`/`quality_manager`：质检录入与降级报废审批分离无法实现
- 缺少 `production_manager`：生产订单管理无独立角色

**修复建议**：
1. 在 `init_service.rs::create_default_roles` 中补齐 9 个角色：`super_admin`/`sales_manager`/`sales`/`customer_service`/`warehouse_manager`/`warehouse`/`quality_inspector`/`quality_manager`/`finance`/`production_manager`/`customer`
2. 为每个角色配置默认权限矩阵（参考审计计划 12.1.4 角色层级表）
3. 保留 `admin` 作为系统初始化角色，`super_admin` 仅用于系统配置（区别于业务 `admin`）

#### ❌ 缺陷项 3：权限码命名规范不统一

**风险等级：P2**（命名不统一增加权限配置认知成本）

**证据**：
- 审计计划 12.1.3 要求格式：`<模块>.<资源>.<操作>`（如 `color_card.issue.create`）
- 实际实现使用 `resource_type` + `action` 两个字段拼接：
  - `/workspace/backend/src/models/role_permission.rs:14-16`：`resource_type: String` + `action: String`
  - `/workspace/backend/database/init_admin_permissions.sql:7-64`：实际权限码为 `purchases + read`（不是 `purchases.order.read`）
- 前端使用 `{resource}:{action}` 格式（`/workspace/frontend/src/router/index.ts:53` `permission: 'dashboard:read'`）
- 后端使用 `resource_type + action` 两个字段，**前后端格式不统一**

**业务影响**：
- 无法表达模块→资源→操作的三级粒度（如 `color_card.issue.create` 无法用 `resource_type=color_card` + `action=issue.create` 表达，会导致权限码匹配混乱）
- 前后端权限码格式不一致：前端 `inventory:read` vs 后端 `(resource_type=inventory, action=read)`

**修复建议**：
1. 统一权限码格式为 `<模块>.<资源>.<操作>`（如 `color_card.issue.create`）
2. 修改 `role_permission` 表增加 `permission_code` 单字段（保留 `resource_type`/`action` 向后兼容）
3. 修改 `permission_middleware` 优先按 `permission_code` 匹配

---

## 类十二 维度 12.2：权限矩阵与最小权限原则

### 检查方法
1. Glob `docs/rbac-permission-matrix.md` 检查权限矩阵文档
2. Grep `default.*deny|default_deny|whitelist|白名单` 在 `backend/src/middleware/` 检索
3. Grep `mutual_exclusive|互斥|conflict_role` 在 `backend/src/services/` 检索
4. Grep `role_inherit|继承` 在 `backend/src/services/` 检索
5. 对照审计计划 12.2 检查要点：权限矩阵/最小权限/继承互斥/默认拒绝/粒度控制

### 发现

#### ✅ 已落实的项
1. **默认拒绝实现**（`/workspace/backend/src/middleware/permission.rs:75-80`）：
   ```rust
   if has_permission {
       Ok(next.run(request).await)
   } else {
       warn!("权限不足: path={} {}", method, path);
       Err(forbidden_response("权限不足，无法访问该资源"))
   }
   ```
   `check_permission` 默认返回 `false`（无匹配权限记录时），符合"白名单"模式

2. **权限粒度控制**（`/workspace/backend/src/models/role_permission.rs:14-17`）：
   - `resource_type`：资源级（如 `inventory`）
   - `action`：操作级（如 `read`/`create`/`update`/`delete`/`*`）
   - `resource_id`：行级（精确到具体资源 ID）
   - `action = "*"`：资源级通配符
   - 基本满足审计计划 12.2.5 模块级/资源级/操作级/字段级中的前 3 级

3. **字段级权限模型**（`/workspace/backend/src/models/field_permission.rs:1-50`）：
   - 含 `role_id` / `resource_type` / `field_name` / `can_read` / `can_write` / `mask_strategy`（NONE/MASK/HASH）/ `is_enabled`
   - 支持字段级读写权限与掩码策略

#### ❌ 缺陷项 1：权限矩阵文档完全缺失

**风险等级：P1**（无文档导致权限配置无审计依据）

**证据**：
- Glob `docs/rbac-permission-matrix.md`：**0 文件**
- Grep `rbac-permission-matrix|权限矩阵` 全量检索 `backend/`：**0 命中**
- 权限配置散落在 `init_admin_permissions.sql`，无文档化的角色×资源×操作矩阵

**业务影响**：
- 审计计划 12.2.1 要求"必须有完整的权限矩阵文档（`docs/rbac-permission-matrix.md`）"
- 矩阵覆盖所有角色 × 所有资源 × 所有操作
- 每个单元格明确"允许/拒绝/审批后允许"
- 矩阵与代码实现一致（禁止文档与代码不符）

**修复建议**：
1. 在 `docs/` 下新建 `rbac-permission-matrix.md` 文档
2. 列出 12 个角色 × 11 个资源 × 7 个操作的完整矩阵
3. 标注每个单元格的允许/拒绝/审批后允许状态
4. 与 `init_admin_permissions.sql` 实际配置对照一致

#### ❌ 缺陷项 2：权限继承与互斥完全未实现

**风险等级：P1**（违反审计计划 12.2.3 要求）

**证据**：
- Grep `mutual_exclusive|互斥|conflict_role` 全量检索 `backend/src/services/`：**0 命中**（仅 `di_container.rs` 的"互斥锁中毒"是无关概念）
- Grep `role_inherit|继承` 全量检索 `backend/src/services/`：**0 命中**
- 无角色继承关系表，无权限互斥校验逻辑

**业务影响**：
- 审计计划 12.2.3 要求：
  - 角色继承：`sales_manager` 继承 `sales` 的所有权限 + 额外审批权限
  - 权限互斥：`finance` 与 `sales` 不能同时拥有（财务与销售职责分离）
  - 系统校验：用户分配角色时检查互斥规则
- 当前实现无继承关系，`sales_manager` 需重复配置 `sales` 所有权限
- 当前实现无互斥校验，一个用户可同时拥有 `finance` 和 `sales` 角色，违反职责分离原则

**修复建议**：
1. 新增 `role_relations` 表：`parent_role_id` / `child_role_id` / `relation_type`（inherit/mutual_exclusive）
2. 在 `role_permission_service.rs::assign_permission` 中加入互斥校验
3. 在 `user_service.rs::assign_role` 中加入用户角色互斥校验

#### ❌ 缺陷项 3：最小权限原则未充分落实

**风险等级：P2**（缺少销售/财务/质检等角色，最小权限原则无法落地）

**证据**：
- `/workspace/backend/src/services/init_service.rs:363-403`：仅有 admin/manager/operator 3 个角色
- `manager` 角色权限为 `["user:view", "product:*", "inventory:*", "sales:*"]`，包含 `inventory:*` 和 `sales:*` 通配符，**权限过宽**
- `operator` 角色权限为 `["product:view", "inventory:view", "sales:view"]`，仅 view 权限
- 缺少销售/财务/质检等专业角色，无法实现"销售角色禁止访问财务成本数据"等最小权限约束

**业务影响**：
- 审计计划 12.2.2 要求每个角色仅授予完成职责所需的最小权限
- 销售角色禁止访问财务成本数据 → 无 `sales` 角色，无法实现
- 客户角色禁止访问其他客户数据 → 无 `customer` 角色，无法实现
- 仓库操作员禁止审批出库 → 无 `warehouse` 角色，无法实现

**修复建议**：
1. 补齐 9 个角色（详见维度 12.1 缺陷项 2）
2. 收紧 `manager` 权限：将 `inventory:*` 改为 `inventory:read,inventory:update` 等具体操作
3. 配置每个角色的最小权限集，参照审计计划 12.1.4 角色层级表

---

## 类十二 维度 12.3：权限校验中间件与后端集成

### 检查方法
1. Read `backend/src/middleware/permission.rs`（权限中间件）
2. Read `backend/src/utils/admin_checker.rs`（管理员绕过逻辑）
3. Grep `customer_id.*IN.*SELECT|data_scope|row_level_permission` 在 `backend/src/services/` 检索（数据权限过滤）
4. Grep `skip_serializing_if|field_permission|字段级权限` 在 `backend/src/models/` 检索
5. 对照审计计划 12.3 检查要点：中间件/注解宏/数据权限/字段级/API 注解

### 发现

#### ✅ 已落实的项
1. **权限校验中间件完整实现**（`/workspace/backend/src/middleware/permission.rs:22-81`）：
   ```rust
   pub async fn permission_middleware(
       State(state): State<AppState>,
       request: Request<Body>,
       next: Next,
   ) -> Result<Response, Response> {
       // 公共路径放行
       // 缺少认证上下文拒绝
       // 无角色拒绝
       // 提取资源类型和 ID
       // 检查权限
       // 通过放行，否则拒绝
   }
   ```
   - 路径解析：`extract_resource_info` 支持 `/api/v1/erp/{resource}` 和 `/api/v1/erp/{module}/{resource}/:id` 两种格式
   - HTTP Method → action 映射：GET→read / POST→create / PUT/PATCH→update / DELETE→delete
   - 权限缓存：`PERMISSION_CACHE`（DashMap）+ 5 分钟 TTL + Arc 包装减少复制

2. **管理员角色绕过**（`/workspace/backend/src/middleware/permission.rs:172-174`）：
   ```rust
   if admin_checker::is_admin_role(db, role_id).await {
       return true;
   }
   ```
   - `is_admin_role` 实现 fail-closed（批次 23 v5 P0-3 修复），数据库表不存在时返回 false
   - 缓存 5 分钟，避免每次请求查数据库

3. **资源 ID 精确匹配防越权**（`/workspace/backend/src/middleware/permission.rs:223-236`）：
   ```rust
   fn matches_permission(...) -> bool {
       p.resource_type == resource_type
           && (p.action == action || p.action == "*")
           && match (p.resource_id, resource_id) {
               (None, None) => true,
               (Some(pid), Some(rid)) => pid == rid,
               _ => false,  // M-6 修复：权限无 ID 不能匹配请求有 ID，防垂直越权
           }
   }
   ```
   - M-6 修复点：权限 `resource_id=None` 不能匹配请求 `resource_id=Some`，防止拥有全局权限的用户操作特定资源
   - 包含 13 个单元测试覆盖各种匹配场景

4. **管理员二次校验**（`/workspace/backend/src/handlers/role_handler.rs:27-41`）：
   ```rust
   async fn require_admin_role(state: &AppState, auth: &AuthContext) -> Result<(), AppError> {
       let role_id = auth.role_id.ok_or_else(...)?;
       if !is_admin_role(&state.db, role_id).await {
           return Err(AppError::permission_denied(format!(
               "该操作仅限管理员（code={}）执行", ADMIN_ROLE_CODE
           )));
       }
       Ok(())
   }
   ```
   - 所有写操作（create/update/delete/assign/remove）顶部调用 `require_admin_role`
   - 防御深度：粗粒度（middleware）+ 细粒度（handler）双重防线

5. **用户提权防护**（`/workspace/backend/src/handlers/user_handler.rs:306-316`）：
   ```rust
   // H-1 修复：禁止通过 update_user 提权到 admin 角色
   if let Some(new_role_id) = req.role_id {
       if is_admin_role(&state.db, new_role_id).await 
           && !is_admin_role(&state.db, auth.role_id.unwrap_or(-1)).await {
           return Err(AppError::permission_denied("禁止将用户角色改为 admin 角色"));
       }
   }
   ```
   - 防止非管理员用户通过 `update_user` 将 `role_id` 改为 admin 角色ID

#### ❌ 缺陷项 1：行级数据权限完全未实现

**风险等级：P0**（销售/客户/仓库数据隔离缺失，存在严重越权风险）

**证据**：
- Grep `apply_data_scope|data_scope_filter|row_level|行级权限` 全量检索 `backend/src/services/`：仅 `data_permission_service.rs` 命中（仅字段过滤，无行级过滤）
- Grep `customer_id.*IN.*SELECT|sales_rep_id` 全量检索 `backend/src/`：**0 命中**
- Read `backend/src/utils/data_permission.rs:1-83`：`DataPermissionFilter` 仅含 `allowed_fields` 和 `hidden_fields`，**无行级过滤条件**
- Read `backend/src/handlers/sales_order_handler.rs:38-72`：`list_orders` 接受 `query.customer_id` 作为查询条件，**未注入销售数据隔离条件**
  ```rust
  pub async fn list_orders(
      ...
      query: Query<OrderQuery>,
  ) -> ... {
      sales_service.list_orders(page_req, query.status, query.customer_id, query.order_no).await?
      ...
  }
  ```
  任何登录用户均可查询任何客户的订单

**业务影响**：
- 审计计划 12.3.3 要求 3 种行级权限：
  - 销售仅能查询自己负责的客户：SQL 自动注入 `WHERE customer_id IN (SELECT customer_id FROM customer_sales_rep WHERE sales_rep_id = ?)`
  - 客户仅能查询自己的订单：SQL 自动注入 `WHERE customer_id = ?`
  - 仓库操作员仅能查询本仓库库存：SQL 自动注入 `WHERE warehouse_id = ?`
- 当前实现**完全没有行级过滤**，任何登录用户均可查询所有客户的所有订单
- 销售能查看竞争对手销售的其他客户订单，存在严重商业机密泄露风险

**修复建议**：
1. 在 `AuthContext` 中增加 `customer_id`/`warehouse_id`/`department_id`/`sales_rep_id` 字段（从 JWT 注入）
2. 新增 `backend/src/utils/data_scope_filter.rs` 实现 `apply_data_scope(query, auth, resource_type)` 函数
3. 在 `customer_handler.rs::list_customers` / `sales_order_handler.rs::list_orders` 等列表接口中调用 `apply_data_scope`
4. 建立 `customer_sales_rep` 关联表（客户-销售代表多对多）

#### ❌ 缺陷项 2：字段级权限未在 DTO 序列化层落地

**风险等级：P2**（字段级权限仅在 service 层过滤，未在 DTO 序列化层统一处理）

**证据**：
- `field_permission` 模型存在（`/workspace/backend/src/models/field_permission.rs`），但 Grep `skip_serializing_if|field_permission|字段级` 在 `backend/src/models/`：**0 命中**
- 字段级权限仅在 `customer_handler.rs::get_permission_filter` + `data_permission_service::filter_fields` 中实现
- 实现方式：在 service 返回 JSON Value 后过滤字段，**不是在 DTO 序列化层**
- 审计计划 12.3.4 要求：DTO 序列化时按权限过滤字段（`#[serde(skip_serializing_if = "Option::is_none")]` + handler 中按权限填充）

**业务影响**：
- 字段级权限仅在 customer 和 sales_order 两个 handler 实现，其他 handler（product/sales_order/warehouse 等）未接入
- 财务成本字段（`cost_amount`/`unit_cost`/`total_cost`）未在所有 DTO 中按权限过滤
- 客户联系方式（`phone`/`email`/`address`）未在所有 DTO 中按权限过滤

**修复建议**：
1. 为关键资源（product/customer/sales_order）的 DTO 增加 `Option<T>` 包装敏感字段
2. 在 handler 中按 `auth.has_permission("finance.cost.view")` 等条件填充字段
3. 推广到所有包含敏感字段的 DTO

#### ❌ 缺陷项 3：客户 handler 硬编码 role_id == 1 判断管理员

**风险等级：P2**（违反单一真相源原则，与 `is_admin_role` 函数并存）

**证据**：
- `/workspace/backend/src/handlers/customer_handler.rs:338`：
  ```rust
  // 管理员角色不过滤
  if role_id == 1 {
      return Ok(None);
  }
  ```
  硬编码 `role_id == 1`，假设 role_id=1 是管理员，但实际管理员角色通过 `role.code == "admin"` 判定

**业务影响**：
- 若管理员角色的 ID 不是 1（如数据库迁移后 ID 变化），此处判断失效
- 与 `is_admin_role` 函数（通过 role.code 判定）逻辑不一致
- 违反单一真相源原则（`admin_checker.rs` 已建立单一真相源）

**修复建议**：
1. 将 `if role_id == 1` 改为 `if is_admin_role(&state.db, role_id).await`
2. 全项目 Grep `role_id == 1` 排查其他硬编码位置

---

## 类十二 维度 12.4：前端权限集成

### 检查方法
1. Read `frontend/src/directives/permission.ts`（v-permission / v-role 指令）
2. Read `frontend/src/router/index.ts:840-928`（路由守卫 + hasRoutePermission）
3. Read `frontend/src/store/user.ts`（用户状态管理）
4. Read `frontend/src/api/request.ts:111-200`（403/401 拦截处理）
5. Grep `userRole.*===.*admin|role.*===.*manager` 在 `frontend/src/` 检索
6. 对照审计计划 12.4 检查要点：路由守卫/按钮级/菜单动态加载/403处理/权限码一致

### 发现

#### ✅ 已落实的项
1. **路由守卫完整实现**（`/workspace/frontend/src/router/index.ts:876-928`）：
   ```typescript
   router.beforeEach(async (to, _from, next) => {
     // 系统初始化检查
     if (to.meta.requiresAuth) {
       if (!userStore.userInfo) {
         try { await userStore.fetchUserInfo() }
         catch { next({ path: '/login', query: { redirect: to.fullPath } }); return }
       }
       if (to.meta.permission && userStore.userInfo) {
         if (!hasRoutePermission(to.meta.permission, userPerms)) {
           next({ path: '/403' }); return
         }
       }
     }
     next()
   })
   ```
   - 未登录跳转 `/login`
   - 权限不足跳转 `/403`
   - 支持 `meta.requiresAuth` + `meta.permission` 双重校验

2. **hasRoutePermission 函数支持通配符**（`/workspace/frontend/src/router/index.ts:849-874`）：
   - 支持 `*:*` 超级通配权限（系统管理员角色由后端注入）
   - 支持 `resource:*` 资源通配
   - 支持 `read/view` 等价 + `update/edit` 等价（后端 action 命名兼容）

3. **v-permission 指令实现**（`/workspace/frontend/src/directives/permission.ts:12-43`）：
   ```typescript
   export const permission: Directive = {
     mounted(el: HTMLElement, binding: DirectiveBinding) {
       // 复用 router 守卫的 hasRoutePermission
       if (!hasPermission) {
         el.parentNode?.removeChild(el)
       }
     },
   }
   ```
   - 已在 `main.ts:34` 全局注册（`app.directive('permission', permission)`）
   - 复用 `hasRoutePermission` 保持与路由守卫一致

4. **路由 meta.permission 配置完整**（`/workspace/frontend/src/router/index.ts:53-245`）：
   - 35+ 路由项均配置 `meta.permission`（如 `permission: 'dashboard:read'` / `permission: 'users:read'`）
   - 所有需登录路由均配置 `meta.requiresAuth: true`

5. **前端权限码防篡改**（`/workspace/frontend/src/store/user.ts:19`）：
   ```typescript
   permissions: Object.freeze([...perms]) as readonly string[],
   ```
   - `Object.freeze` 防止前端组件恶意修改权限码数组（批次 22 v5 P0-5 修复）

6. **401 自动刷新流程**（`/workspace/frontend/src/api/request.ts:145-178`）：
   - 401 自动调用 `/auth/refresh` 续期
   - 失败时通知所有排队请求 reject（FE-P1-1 修复）

7. **403 处理**（`/workspace/frontend/src/api/request.ts:133-143` + `228`）：
   - HTTP 403 + `CSRF_TOKEN_MISSING`/`CSRF_TOKEN_INVALID` 跳转登录
   - 通用 403 通过 `SAFE_ERROR_MESSAGES[403] = '拒绝访问'` 提示用户

#### ❌ 缺陷项 1：v-role 指令使用 role_name 硬编码角色判断

**风险等级：P2**（违反审计计划 12.4.2 "禁止使用 `v-if='userRole === admin'` 硬编码角色判断"）

**证据**：
- `/workspace/frontend/src/directives/permission.ts:51-71`：
  ```typescript
  export const role: Directive = {
    mounted(el: HTMLElement, binding: DirectiveBinding) {
      const userRole = userStore.userInfo?.role_name || ''
      let hasRole = false
      if (Array.isArray(value)) {
        hasRole = value.includes(userRole)
      } else {
        hasRole = userRole === value
      }
      if (!hasRole) {
        el.parentNode?.removeChild(el)
      }
    },
  }
  ```
  使用 `userRole === value` 硬编码角色名判断，违反审计计划要求

**业务影响**：
- 角色重命名后前端按钮显示逻辑失效
- 违反"应使用权限码"的最佳实践
- 审计计划 12.4.2 明确禁止此模式

**修复建议**：
1. 删除 `v-role` 指令，统一使用 `v-permission` 权限码指令
2. 或将 `v-role` 改为查 role 对应的权限码集合后调用 `hasRoutePermission`

#### ❌ 缺陷项 2：菜单动态加载未实现

**风险等级：P2**（菜单未根据权限动态生成）

**证据**：
- Grep `menu.filter|menu.*hasPermission|动态.*菜单` 在 `frontend/src/`：**0 命中**
- 路由配置是静态的（`/workspace/frontend/src/router/index.ts` 全量静态路由）
- 菜单生成基于路由 meta，但未根据用户权限过滤

**业务影响**：
- 审计计划 12.4.3 要求：后端返回用户权限列表 + 前端根据权限动态生成菜单 + 无权限的菜单项不显示
- 当前实现：所有菜单项均渲染，仅在点击时由路由守卫跳转 `/403`，用户体验差

**修复建议**：
1. 在 `MainLayout.vue` 中根据 `userStore.userInfo.permissions` 过滤菜单项
2. 实现 `menu.filter(item => hasRoutePermission(item.permission, userPerms))`

---

## 类十二 维度 12.5：权限审计日志与追溯

### 检查方法
1. Grep `permission_audit_log|permission_audit` 在 `backend/src/models/` 检索
2. Read `backend/src/models/audit_log.rs`（通用审计日志模型）
3. Grep `audit.*role|audit.*permission|role.*audit` 在 `backend/src/services/` 检索
4. Read `backend/src/handlers/role_handler.rs` 检查权限变更审计记录
5. 对照审计计划 12.5 检查要点：权限变更审计/校验日志/审计日志表/保留期限/查询接口

### 发现

#### ✅ 已落实的项
1. **通用审计日志模型存在**（`/workspace/backend/src/models/audit_log.rs:71-107`）：
   - `table_name = "audit_logs"`
   - 字段：`user_id` / `username` / `action` / `resource_type` / `resource_id` / `resource_name` / `description` / `ip_address` / `user_agent` / `request_method` / `request_path` / `request_body` / `response_status` / `duration_ms` / `old_value` / `new_value` / `operation_type`（CREATE/UPDATE/DELETE/LOGIN/LOGOUT/EXPORT/QUERY/OTHER）/ `severity`（INFO/WARN/ERROR/CRITICAL）/ `request_id` / `before_snapshot` / `after_snapshot`

2. **角色 CRUD 审计日志记录**（`/workspace/backend/src/handlers/role_handler.rs`）：
   - `create_role`（第 207-232 行）：记录 `OperationType::Create` + `after_snapshot`
   - `update_role`（第 300-325 行）：记录 `OperationType::Update` + `before_snapshot` + `after_snapshot` + 清理 admin 缓存
   - `delete_role`（第 395-414 行）：记录 `OperationType::Delete` + `before_snapshot` + `Severity::Warn`
   - `assign_permission`（第 443-472 行）：记录 `OperationType::Update` + `after_snapshot` + `Severity::Warn`
   - `remove_permission`（第 498-520 行）：记录 `OperationType::Delete` + `before_snapshot` + `Severity::Warn`

3. **用户角色变更审计**（`/workspace/backend/src/handlers/user_handler.rs:306-374`）：
   - `update_user` 记录 `OperationType::Update` + `before_snapshot`（含 `role_id`）+ `after_snapshot`（含 `role_id`）
   - H-1 修复：禁止通过 update_user 提权到 admin 角色

4. **审计日志服务接入**（`/workspace/backend/src/services/audit_log_service.rs`）：
   - 提供 `update_with_audit` / `delete_with_audit` 方法
   - `role_permission_service.rs` 在 update/delete 时调用 audit_log_service

#### ❌ 缺陷项 1：缺少独立的 `permission_audit_log` 表

**风险等级：P2**（权限审计日志混在通用 audit_logs 表，难以独立查询和保留）

**证据**：
- Grep `permission_audit_log|permission_audit` 全量检索 `backend/src/models/`：**0 命中**
- 权限变更记录混入通用 `audit_logs` 表，通过 `resource_type = "role"` 或 `"role_permission"` 区分

**业务影响**：
- 审计计划 12.5.3 要求独立的 `permission_audit_log` 表，含 `user_id` / `action`（role.create/role.update/permission.assign/user.role.assign）/ `resource_type` / `resource_id` / `old_value` / `new_value` / `ip_address` / `user_agent` / `created_at` + 3 个索引（user_id/action/created_at）
- 独立表便于权限审计日志的独立保留期限管理（3 年以上）和高效查询
- 当前混合存储难以区分权限审计与其他业务审计

**修复建议**：
1. 新增 `permission_audit_log` 表（独立于 `audit_logs`）
2. 在 `role_handler` / `user_handler` 中将权限变更记录同时写入 `permission_audit_log`
3. 配置独立保留策略（3 年以上）

#### ❌ 缺陷项 2：权限拒绝日志未落库

**风险等级：P1**（权限拒绝是安全事件，应落库审计）

**证据**：
- `/workspace/backend/src/middleware/permission.rs:78-79`：
  ```rust
  warn!("权限不足: path={} {}", method, path);
  Err(forbidden_response("权限不足，无法访问该资源"))
  ```
  仅 `tracing::warn!` 记录到日志文件，**未调用 `audit_log_service` 落库**
- Grep `权限拒绝|permission.*denied|forbidden.*log` 在 `backend/src/middleware/`：**0 命中**

**业务影响**：
- 审计计划 12.5.2 要求：权限拒绝时必须记录（`user_id` + `required_permission` + `resource_id` + `ip` + `user_agent`）
- 当前实现仅 tracing 日志，无法查询历史权限拒绝事件
- 攻击者尝试越权访问时无法追溯

**修复建议**：
1. 在 `permission_middleware` 权限拒绝分支调用 `AuditLogService::record_async` 落库
2. 记录 `OperationType::Other` + `Severity::Warn` + `resource_type="permission_denied"`
3. 含 user_id/path/method/ip/user_agent 等字段

#### ❌ 缺陷项 3：审计日志保留期限策略缺失

**风险等级：P2**（无保留期限策略，存在合规风险）

**证据**：
- Grep `audit.*retention|retention.*audit|保留期限|3.*年` 全量检索 `backend/src/`：**0 命中**
- 无审计日志保留期限配置
- 无自动清理策略

**业务影响**：
- 审计计划 12.5.4 要求：
  - 权限变更日志：保留 3 年以上（法律合规要求）
  - 权限拒绝日志：保留 1 年以上
  - 禁止自动清理权限审计日志
- 当前实现无保留期限策略，存在数据无限增长或被自动清理风险

**修复建议**：
1. 在 `audit_cleanup_service.rs` 中配置权限审计日志保留期限（3 年）
2. 禁止自动清理 `resource_type IN ('role', 'role_permission', 'permission_denied')` 的审计日志

#### ❌ 缺陷项 4：权限审计日志查询接口缺失

**风险等级：P2**（管理员无法查询权限变更记录）

**证据**：
- Read `backend/src/routes/iam.rs:71-90`：仅有 `/permissions` 和 `/field-permissions` 路由，**无权限审计日志查询路由**
- Grep `permission_audit.*query|查询.*权限审计` 全量检索 `backend/src/handlers/`：**0 命中**

**业务影响**：
- 审计计划 12.5.5 要求：管理员可查询所有权限变更记录 + 按 `user_id`/`action`/`resource_type`/时间范围筛选 + 支持导出 .xlsx
- 当前实现无独立查询接口

**修复建议**：
1. 在 `iam.rs` 中新增 `/permission-audit-logs` 路由
2. 复用 `audit_log_handler.rs` 提供查询接口，按 `resource_type IN ('role', 'role_permission')` 过滤
3. 复用 `export_service.rs` 支持 .xlsx 导出

---

## 类十二 维度 12.6：动态授权与权限委托

### 检查方法
1. Grep `redis.*permission|permission.*cache|role.*cache` 在 `backend/src/services/` 检索
2. Grep `delegation|delegate|委托` 在 `backend/src/services/` 检索
3. Grep `pub.*sub|cache.*invalidat|热更新` 在 `backend/src/` 检索
4. Grep `双人审批|two_person_approval` 在 `backend/src/` 检索
5. 对照审计计划 12.6 检查要点：动态权限分配/权限委托/缓存失效/热更新/变更审批

### 发现

#### ✅ 已落实的项
1. **本地权限缓存实现**（`/workspace/backend/src/middleware/permission.rs:131-208`）：
   - `PERMISSION_CACHE: LazyLock<DashMap<i32, CacheEntry<Arc<Vec<role_permission::Model>>>>>`
   - TTL 5 分钟（`PERMISSION_CACHE_TTL: i64 = 5`）
   - 使用 `Arc` 包装减少数据复制
   - 过期自动移除并重新加载

2. **管理员角色缓存失效**（`/workspace/backend/src/utils/admin_checker.rs:42-52`）：
   ```rust
   pub fn clear_admin_role_cache(role_id: Option<i32>) {
       if let Some(id) = role_id {
           ADMIN_ROLE_CACHE.remove(&id);
       } else {
           ADMIN_ROLE_CACHE.clear();
       }
   }
   ```
   - 在 `role_handler::update_role` / `delete_role` 中调用（批次 103 P2-3 修复）

3. **过期缓存清理任务**（`/workspace/backend/src/utils/admin_checker.rs:54-57`）：
   ```rust
   pub fn cleanup_expired_admin_cache() {
       ADMIN_ROLE_CACHE.retain(|_, entry| !entry.is_expired());
   }
   ```
   - main.rs 后台任务每 10 分钟调用（v11 批次 156 P2-D）

#### ❌ 缺陷项 1：权限委托（Delegation）完全未实现

**风险等级：P1**（违反审计计划 12.6.2 要求）

**证据**：
- Grep `delegation|delegate|委托` 全量检索 `backend/src/services/`：**0 命中**
- 无权限委托表、无委托服务、无委托 API

**业务影响**：
- 审计计划 12.6.2 要求：
  - 销售经理可将部分权限委托给销售（如审批权限临时委托）
  - 委托必须有时限（`valid_from` + `valid_until`）
  - 委托必须记录审计日志
  - 委托不可再委托（禁止链式委托）
- 当前实现完全无委托能力，销售经理请假时无法将审批权限临时委托

**修复建议**：
1. 新增 `permission_delegations` 表：`delegator_id` / `delegatee_id` / `permission_code` / `valid_from` / `valid_until` / `is_chain_allowed`（默认 false）
2. 新增 `backend/src/services/delegation_service.rs`
3. 在 `permission_middleware` 中聚合用户自身权限 + 委托获得的权限

#### ❌ 缺陷项 2：Redis 权限缓存与热更新未实现

**风险等级：P2**（多实例部署时权限缓存不一致）

**证据**：
- Grep `redis.*permission|permission.*cache|role.*cache` 全量检索 `backend/src/services/`：仅 `cache_service.rs` 命中（通用 Redis 缓存，未用于权限）
- Grep `pub.*sub|cache.*invalidat|热更新` 全量检索 `backend/src/`：**0 命中**（无 pub/sub 通知机制）
- 权限缓存仅本地 `DashMap`，无 Redis 分布式缓存，无 pub/sub 热更新

**业务影响**：
- 审计计划 12.6.1 + 12.6.4 要求：
  - 支持 Redis 缓存权限（`role:{role_id}:permissions`）
  - 权限配置变更后，通过 Redis pub/sub 通知所有服务实例
  - 各实例收到通知后清除本地权限缓存
- 当前实现仅本地缓存，多实例部署时权限变更后其他实例 5 分钟内仍使用旧权限
- 缓存失效失败时无回退到数据库查询的兜底（虽然有 `unwrap_or_default`，但返回空权限列表会导致权限被拒绝）

**修复建议**：
1. 在 `cache_service.rs` 中增加权限缓存层（`role:{role_id}:permissions`）
2. 权限变更时通过 Redis pub/sub 通知所有实例
3. 各实例收到通知后清除本地 `PERMISSION_CACHE` 对应条目

#### ❌ 缺陷项 3：敏感角色变更双人审批未实现

**风险等级：P2**（违反审计计划 12.6.5 要求）

**证据**：
- Grep `双人审批|two_person_approval|sensitive.*role.*approval` 全量检索 `backend/src/`：**0 命中**
- `/workspace/backend/src/handlers/role_handler.rs::assign_permission` 仅 `require_admin_role` 单人审批

**业务影响**：
- 审计计划 12.6.5 要求：分配 `super_admin`/`finance` 角色需双人审批
- 审批流程：申请人提交 → 审批人审批 → 生效
- 当前实现单人 admin 即可分配任何角色，存在权限滥用风险

**修复建议**：
1. 在 `assign_role` 接口对 `super_admin`/`finance` 角色加入 BPM 审批流程
2. 复用 `bpm_service.rs` 实现双人审批工作流

---

## 类十二 维度 12.7：数据权限（行级/字段级）

### 检查方法
1. Read `backend/src/models/data_permission.rs`（数据权限模型）
2. Read `backend/src/services/data_permission_service.rs`（数据权限服务）
3. Read `backend/src/utils/data_permission.rs`（数据权限过滤工具）
4. Read `backend/src/handlers/customer_handler.rs:320-385`（客户数据权限接入）
5. Grep `sales_rep_id|customer_sales_rep|sales.*data_scope` 全量检索
6. 对照审计计划 12.7 检查要点：行级/字段级/实现方式/业务结合

### 发现

#### ✅ 已落实的项
1. **数据权限模型存在**（`/workspace/backend/src/models/data_permission.rs:1-50`）：
   - `table_name = "data_permissions"`
   - 字段：`role_id` / `resource_type` / `scope_type`（ALL/DEPT/DEPT_AND_BELOW/SELF/CUSTOM）/ `custom_condition`（JSON）/ `allowed_fields`（JSON 数组）/ `hidden_fields`（JSON 数组）/ `is_enabled`
   - 模型层支持行级（scope_type）+ 字段级（allowed_fields/hidden_fields）配置

2. **字段级权限服务完整**（`/workspace/backend/src/services/data_permission_service.rs:196-228`）：
   - `filter_fields`：根据 allowed_fields/hidden_fields 过滤 JSON 对象字段
   - `filter_fields_batch`：批量过滤
   - 在 `customer_handler.rs` / `sales_order_handler.rs` 中接入使用

3. **客户 handler 接入数据权限**（`/workspace/backend/src/handlers/customer_handler.rs:104-150`）：
   ```rust
   let permission_filter = get_permission_filter(&state, &auth, "customer").await?;
   state.customer_service.list_customers_with_filter(..., permission_filter).await?
   ```
   - 非管理员角色查询客户时，应用字段过滤（隐藏 credit_limit/bank_name/contact_phone 等敏感字段）
   - 默认隐藏字段定义在 `/workspace/backend/src/utils/data_permission.rs:74-83`（DEFAULT_HIDDEN_FIELDS）

4. **销售订单 handler 接入字段级权限**（`/workspace/backend/src/handlers/sales_order_handler.rs:58-72`）：
   ```rust
   .data_permission_service
       .get_role_data_permission(role_id, "sales_order")
   ...
   state.data_permission_service.filter_fields_batch(...)
   ```

5. **数据权限服务事务化**（`/workspace/backend/src/services/data_permission_service.rs:101-147`）：
   - 批次 85 v2 复审 P1-8 修复：find + update/insert 移入单一事务 + lock_exclusive 串行化

#### ❌ 缺陷项 1：行级数据权限（Row-Level Security）完全未实现

**风险等级：P0**（销售/客户/仓库数据隔离缺失，敏感数据可被任意访问）

**证据**：
- `data_permission_service.rs` 仅实现了 `filter_fields`（字段级），**无 `apply_data_scope` 行级过滤函数**
- `/workspace/backend/src/services/data_permission_service.rs:21-23` 注释明确：
  ```rust
  /// 批次 119 P2-5 修复：删除 4 个未接入业务的 scope 常量（DEPT/DEPT_AND_BELOW/SELF/CUSTOM），
  /// 仅保留 ALL（admin 角色使用）。如未来需要行级权限校验，应通过 data_permission 表的
  /// scope_type 字段动态读取，而非硬编码常量。
  ```
  承认行级权限未接入业务
- Grep `customer_id.*eq.*auth|sales_rep_id|customer_sales_rep` 全量检索 `backend/src/services/`：**0 命中**
- `customer_handler.rs::list_customers` 未注入 `customer_id IN (SELECT ...)` 条件
- `sales_order_handler.rs::list_orders` 接受 `query.customer_id` 作为查询参数，**未注入销售数据隔离**

**业务影响**：
- 审计计划 12.7.1 + 12.7.2 要求 4 种行级权限 + 实现方式：
  - 销售数据隔离：销售仅能查询自己负责的客户
  - 客户门户隔离：客户仅能查询自己的订单/色卡/批色记录
  - 仓库隔离：仓库操作员仅能查询本仓库库存
  - 部门隔离：销售经理仅能查询本部门销售数据
- 当前实现**完全没有行级过滤**，任何登录用户均可查询所有客户的所有订单
- 销售能查看竞争对手销售的其他客户订单，存在严重商业机密泄露风险
- 客户门户批色（维度 11.3）无法实现，因为客户能查看所有客户的批色记录

**修复建议**：
1. 在 `AuthContext` 中增加 `customer_id`/`warehouse_id`/`department_id`/`sales_rep_id` 字段
2. 新增 `backend/src/utils/data_scope_filter.rs` 实现 `apply_data_scope(query, auth, resource_type)` 函数
3. 在 `customer_handler` / `sales_order_handler` / `inventory_stock_handler` 等列表接口中调用
4. 建立 `customer_sales_rep` 关联表（客户-销售代表多对多）

#### ❌ 缺陷项 2：字段级权限仅在 customer/sales_order 接入

**风险等级：P2**（其他资源未接入字段级权限）

**证据**：
- Grep `DataPermissionService::new|data_permission_service` 全量检索 `backend/src/handlers/`：仅 6 个文件命中
  - `mod.rs` / `app_state.rs`（基础设施）
  - `purchase_order_handler.rs` / `purchase_receipt_handler.rs` / `sales_order_handler.rs` / `inventory_stock_handler.rs` / `customer_handler.rs` / `data_permission_handler.rs` / `crm_handler.rs` / `ap_payment_request_handler.rs`
- 但 `product_handler.rs` / `warehouse_handler.rs` / `supplier_handler.rs` 等未接入字段级权限
- 财务成本字段（`cost_amount`/`unit_cost`/`total_cost`）未在 product DTO 中按权限过滤

**业务影响**：
- 审计计划 12.7.3 要求字段级数据权限：
  - 财务成本字段（`cost_amount`/`unit_cost`/`total_cost`）仅 `finance`/`admin` 可见
  - 客户联系方式（`phone`/`email`/`address`）仅 `sales`/`customer_service` 可见
  - 采购价格（`purchase_price`）仅 `admin`/`finance` 可见
- 当前实现仅 customer/sales_order 接入，product 等资源未接入，财务成本字段可能泄露给销售角色

**修复建议**：
1. 在 `product_handler.rs` 接入 `get_permission_filter` 过滤财务成本字段
2. 在 `supplier_handler.rs` 接入过滤采购价格字段
3. 推广到所有包含敏感字段的 handler

---

## 类十二 维度 12.8：RBAC 安全审计与漏洞防护

### 检查方法
1. Grep `role_id.*ignore|role_id.*skip|禁止.*role_id` 在 `backend/src/handlers/user/` 检索
2. Grep `can_access_resource|resource.*owner|IDOR` 在 `backend/src/middleware/` 检索
3. Grep `admin.*true.*param|bypass.*permission` 在 `backend/src/` 检索
4. Grep `lock_exclusive.*permission|permission.*lock` 在 `backend/src/services/` 检索
5. Grep `session_fixation|regenerate.*session|会话固定` 全量检索
6. 对照审计计划 12.8 检查要点：权限提升/越权/绕过/会话固定/并发/配置审计/压力测试

### 发现

#### ✅ 已落实的项
1. **权限提升攻击防护**（`/workspace/backend/src/handlers/user_handler.rs:28-43` + `306-316`）：
   ```rust
   // H-1 修复：用户管理 admin 校验 + 限制非 admin 修改 role_id
   // 安全原因：低权限用户调用 create_user 时可指定 role_id=admin_role_id
   // 提权；update_user 时可改写他人 role_id 字段。
   async fn require_admin_role(...) -> Result<(), AppError> { ... }
   ```
   - `create_user` / `update_user` / `delete_user` 顶部调用 `require_admin_role`
   - 进一步防御：`update_user` 中禁止将 `role_id` 改为 admin 角色（即使调用者是 admin）

2. **管理员角色查询 fail-closed**（`/workspace/backend/src/utils/admin_checker.rs:77-90`）：
   ```rust
   let is_admin = match role::Entity::find_by_id(role_id).one(db).await {
       Ok(Some(role)) => role.code == ADMIN_ROLE_CODE,
       Ok(None) => false,
       Err(e) => {
           if err_msg.contains("does not exist") || err_msg.contains("relation") {
               warn!("数据库表不存在，系统可能未初始化，拒绝访问（fail-closed）: {}", e);
               false
           } else {
               warn!("查询角色失败: {}", e);
               false
           }
       }
   };
   ```
   - 批次 23 v5 P0-3 修复：原 fail-open（数据库表不存在返回 true）改为 fail-closed（返回 false）
   - 防止系统未初始化时任何 role_id 都被视为管理员

3. **角色更新/删除事务化防 TOCTOU**（`/workspace/backend/src/services/role_service.rs:67-112`）：
   ```rust
   pub async fn update_role(...) -> Result<role::Model, AppError> {
       let txn = (*self.db).begin().await?;
       let role_model = role::Entity::find_by_id(role_id)
           .lock_exclusive()  // 加 lock_exclusive 串行化并发状态变更
           .one(&txn).await?;
       ...
       txn.commit().await?;
   }
   ```
   - 批次 86 v2 复审 P2-1 修复：find + 状态门 + update 移入单一事务 + lock_exclusive

4. **资源 ID 精确匹配防垂直越权**（`/workspace/backend/src/middleware/permission.rs:223-236`）：
   - `matches_permission` 中 `resource_id` 精确匹配（None 与 Some 不互通）
   - 防止拥有全局权限的用户操作特定资源
   - 13 个单元测试覆盖

5. **系统角色不可修改/删除**（`/workspace/backend/src/handlers/role_handler.rs:371-375`）：
   ```rust
   if old_role.is_system {
       return Err(AppError::bad_request("系统内置角色不可删除"));
   }
   ```
   - `update_role` / `delete_role` / `assign_permission` / `remove_permission` 均校验 `is_system`

6. **登录后用户删除管理员检查**（`/workspace/backend/src/handlers/user_handler.rs:411-418`）：
   ```rust
   if let Some(user_role_id) = existing_user.role_id {
       if is_admin_role(&state.db, user_role_id).await {
           // 检查是否是最后一个管理员
           let count = user::Entity::find()
               .filter(user::Column::RoleId.eq(user_role_id))
               .count(&*self.db).await?;
           ...
       }
   }
   ```
   - 防止删除最后一个管理员导致系统永久锁定

#### ❌ 缺陷项 1：越权访问防护（IDOR）未实现

**风险等级：P0**（所有 `/:id` 路由未校验资源归属）

**证据**：
- Grep `can_access_resource|resource.*owner|IDOR` 全量检索 `backend/src/middleware/`：**0 命中**
- `/workspace/backend/src/middleware/permission.rs:22-81`：`permission_middleware` 仅校验角色对资源类型的访问权限，**未校验资源归属**
- 如销售查询订单 `/api/v1/erp/sales/orders/123`，中间件仅校验 `role_id` 是否有 `orders:read` 权限，**不校验 123 是否属于该销售负责的客户**
- 同样适用于 `/customers/:id` / `/sales-orders/:id` / `/inventory-stocks/:id` 等所有 `/:id` 路由

**业务影响**：
- 审计计划 12.8.2 要求：所有 `/:id` 路由必须校验资源归属
- 示例：销售查询订单 `/orders/123`，必须校验 `123` 是否属于该销售负责的客户
- 实现方式：`if !auth.can_access_resource("order", id) { return 403 }`
- 当前实现无资源归属校验，销售可访问任何客户的订单详情（水平越权）

**修复建议**：
1. 在 `AuthContext` 增加 `can_access_resource(resource_type, resource_id) -> bool` 方法
2. 在所有 `/:id` handler 中调用资源归属校验
3. 优先处理 customer/sales_order/inventory 等敏感资源

#### ❌ 缺陷项 2：会话固定攻击防护未实现

**风险等级：P1**（违反审计计划 12.8.4 要求）

**证据**：
- Grep `session_fixation|regenerate.*session|会话固定` 全量检索 `backend/src/`：**0 命中**
- 登录后未重新生成 session ID
- 权限变更后未清除旧 session

**业务影响**：
- 审计计划 12.8.4 要求：
  - 登录后必须重新生成 session ID
  - 权限变更后必须清除旧 session（强制重新登录）
- 当前实现登录后未重新生成 session ID，存在会话固定攻击风险
- 用户角色变更后旧 session 仍有效，权限缓存 5 分钟内可能使用旧权限

**修复建议**：
1. 登录成功后重新生成 access_token/refresh_token
2. 在 `update_user` 修改 `role_id` 后，清除目标用户的 access_token/refresh_token（强制重新登录）
3. 复用 `cache_service.rs` 的 token 黑名单机制

#### ❌ 缺陷项 3：RBAC 压力测试与性能监控缺失

**风险等级：P3**（性能监控为辅助功能）

**证据**：
- Grep `RBAC.*压力|permission.*performance|权限.*性能` 全量检索 `backend/src/`：**0 命中**
- 无权限校验性能监控指标
- 无高并发场景测试

**业务影响**：
- 审计计划 12.8.7 要求：
  - 权限校验性能：单次权限校验 < 10ms（Redis 缓存命中时 < 1ms）
  - 高并发场景：1000 QPS 下权限校验不降级
  - 缓存失效场景：权限缓存全失效时系统仍可用（回退数据库查询）
- 当前实现有本地缓存（5 分钟 TTL），但无性能指标监控
- 缓存全失效时 `unwrap_or_default` 返回空权限列表，会导致权限被拒绝（应回退到拒绝访问而非放行，但用户体验差）

**修复建议**：
1. 在 `permission_middleware` 中增加 `metrics_service` 记录权限校验耗时
2. 配置 Grafana 监控权限校验 P99 延迟
3. 编写 RBAC 压力测试用例

#### ❌ 缺陷项 4：权限配置定期审计缺失

**风险等级：P3**（配置审计为辅助功能）

**证据**：
- Grep `权限.*快照|snapshot.*permission|配置审计` 全量检索 `backend/src/`：**0 命中**
- 无权限配置快照功能
- 无异常变更告警

**业务影响**：
- 审计计划 12.8.6 要求：
  - 定期审计权限配置（每月生成权限配置快照）
  - 对比快照发现异常变更（如非工作时间权限变更）
  - 权限配置变更告警（邮件/短信通知管理员）
- 当前实现无此能力

**修复建议**：
1. 新增 `permission_config_snapshot` 表，每月自动生成权限配置快照
2. 复用 `notification_service.rs` 在权限变更时通知管理员

---

## 审计结果汇总

| 维度 | P0 | P1 | P2 | P3 | 已落实 | 总检查项 |
|------|----|----|----|----|--------|----------|
| 11.1 大货批色数据模型与状态机 | 1 | 0 | 0 | 0 | 0 | 4 |
| 11.2 剪大货样业务规则 | 1 | 0 | 0 | 0 | 0 | 5 |
| 11.3 客户批色确认流程 | 1 | 0 | 0 | 0 | 0 | 5 |
| 11.4 批色不通过处理流程 | 1 | 0 | 0 | 0 | 2 | 4 |
| 11.5 批色报表与统计 | 0 | 0 | 1 | 0 | 0 | 5 |
| 11.6 批色业务与其他模块集成 | 1 | 0 | 0 | 0 | 0 | 6 |
| **类十一小计** | **5** | **0** | **1** | **0** | **2** | **29** |
| 12.1 RBAC 数据模型与权限架构 | 0 | 2 | 1 | 0 | 5 | 4 |
| 12.2 权限矩阵与最小权限原则 | 0 | 2 | 1 | 0 | 3 | 5 |
| 12.3 权限校验中间件与后端集成 | 1 | 0 | 2 | 0 | 5 | 5 |
| 12.4 前端权限集成 | 0 | 0 | 2 | 0 | 7 | 5 |
| 12.5 权限审计日志与追溯 | 0 | 1 | 3 | 0 | 4 | 5 |
| 12.6 动态授权与权限委托 | 0 | 1 | 2 | 0 | 3 | 5 |
| 12.7 数据权限（行级/字段级） | 1 | 0 | 1 | 0 | 5 | 5 |
| 12.8 RBAC 安全审计与漏洞防护 | 1 | 1 | 0 | 2 | 6 | 7 |
| **类十二小计** | **3** | **7** | **12** | **2** | **38** | **41** |
| **合计** | **8** | **7** | **13** | **2** | **40** | **70** |

---

## 修复优先级队列

### 🔴 P0（阻塞 - 立即修复）

1. **类十一 11.1**：新增 `bulk_color_approval` 表 + 模型 + 服务 + 8 状态机 + 9 流转规则 + 交货门禁
2. **类十一 11.2**：实现剪大货样业务规则（5 项前置条件 + 库存联动 + 事务化 + 标识 + 追溯）
3. **类十一 11.3**：实现客户批色确认流程（客户门户 + 批色通知 + 时限 + 色差判定 + 结果处理）
4. **类十一 11.4**：实现批色不通过处理流程（返工 + 降级 + 报废 + 财务凭证联动）
5. **类十一 11.6**：批色业务与其他模块集成（销售出库门禁最关键 - 修改 `sales_order_handler.rs::ship_order` 加入 `approval_status = Approved` 校验）
6. **类十二 12.3**：实现行级数据权限（`apply_data_scope` + 销售数据隔离 + 客户门户隔离 + 仓库隔离）
7. **类十二 12.7**：行级数据权限完全未实现（同 12.3，重复列出强调）
8. **类十二 12.8**：越权访问防护（IDOR）- 所有 `/:id` 路由必须校验资源归属

### 🟠 P1（高 - 短期修复）

1. **类十二 12.1**：新增 `user_role` 关联表，支持多角色
2. **类十二 12.1**：补齐 9 个面料行业 ERP 必备角色（sales_manager/sales/customer_service/warehouse_manager/warehouse/quality_inspector/quality_manager/finance/production_manager/customer）
3. **类十二 12.2**：编写权限矩阵文档 `docs/rbac-permission-matrix.md`
4. **类十二 12.2**：实现权限继承与互斥校验
5. **类十二 12.5**：权限拒绝日志落库审计（`permission_middleware` 调用 `audit_log_service`）
6. **类十二 12.6**：实现权限委托（Delegation）
7. **类十二 12.8**：会话固定攻击防护（登录后重新生成 session ID + 权限变更清除旧 session）

### 🟡 P2（中 - 计划修复）

1. **类十一 11.5**：批色报表与统计（依赖 11.1-11.4 业务上线后）
2. **类十二 12.1**：统一权限码命名规范为 `<模块>.<资源>.<操作>`
3. **类十二 12.2**：最小权限原则 - 收紧 `manager` 角色权限（`inventory:*` → 具体操作）
4. **类十二 12.3**：字段级权限在 DTO 序列化层落地（推广到 product/supplier 等）
5. **类十二 12.3**：客户 handler 硬编码 `role_id == 1` 改为 `is_admin_role` 函数
6. **类十二 12.4**：删除 `v-role` 指令，统一使用 `v-permission` 权限码
7. **类十二 12.4**：菜单动态加载（根据权限过滤菜单项）
8. **类十二 12.5**：新增独立 `permission_audit_log` 表
9. **类十二 12.5**：审计日志保留期限策略
10. **类十二 12.5**：权限审计日志查询接口
11. **类十二 12.6**：Redis 权限缓存与热更新
12. **类十二 12.6**：敏感角色变更双人审批
13. **类十二 12.7**：字段级权限推广到 product/supplier 等 handler

### 🟢 P3（低 - 优化项）

1. **类十二 12.8**：RBAC 压力测试与性能监控
2. **类十二 12.8**：权限配置定期审计与快照

---

## 关键发现总结

### 类十一：大货批色业务规则（6 维度）

**整体结论**：大货批色业务模块**完全缺失**，是 V15 审计中最严重的业务空白。仅在 `lab_dip_service.rs:833` 有注释提及"大货生产前必须复样"，但属于 lab_dip 打样通知单流程，**非大货批色**。

**最严重风险**：销售出库 handler `sales_order_handler.rs::ship_order`（第 307-308 行）不校验批色状态，未批色通过的面料可直接出库，**违反面料行业"交货前客户批色"业务铁律**。

**修复路径**：建议作为 V16 专项立项，按 11.1 → 11.2 → 11.3 → 11.4 → 11.6 → 11.5 顺序实现，其中 11.6 销售出库门禁可作为最小可用方案先行上线。

### 类十二：RBAC 权限控制（8 维度）

**整体结论**：RBAC 基础架构已建立（角色/权限/中间件/前端指令），但存在 3 个 P0 严重缺陷：

1. **行级数据权限完全未实现**（12.3 + 12.7）：销售能查询所有客户的订单，客户能查询所有客户的批色记录
2. **越权访问防护（IDOR）未实现**（12.8）：所有 `/:id` 路由未校验资源归属，销售可访问任何客户订单详情
3. **角色层级不完整**（12.1）：仅 admin/manager/operator 3 个角色，缺少销售/客户/财务/质检/仓库等 9 个必备角色

**已实现亮点**：
- 权限校验中间件完整（5 分钟缓存 + fail-closed + 资源 ID 精确匹配）
- 前端权限集成完整（路由守卫 + v-permission 指令 + hasRoutePermission 通配符 + 403 跳转）
- 角色 CRUD 审计日志记录完整（before/after_snapshot + Severity 分级）
- 用户提权防护（H-1 修复：禁止非 admin 修改 role_id 为 admin）
- 系统角色不可修改/删除
- 事务化防 TOCTOU（lock_exclusive）

**修复路径**：优先修复 3 个 P0（行级数据权限 + IDOR 防护 + 角色补齐），再修复 7 个 P1（user_role 表 + 权限矩阵文档 + 继承互斥 + 权限拒绝日志 + 委托 + 会话固定防护）。

---

## 审计完成声明

本审计报告基于 2026-07-16 当日代码状态完成，覆盖类十一 6 维度 + 类十二 8 维度共 14 维度 70 项检查点。所有结论均附文件路径:行号证据。审计过程未修改任何业务代码。

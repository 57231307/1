# V15 组织定制物流审计报告（类二十三·批次 19）

- **审计子代理**：V15 审计子代理（类二十三组织定制物流）
- **审计范围**：5 维度（类二十三 23.1～23.5）
- **审计依据**：
  - `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md`（第 6602-6664 行 类二十三）
  - `/workspace/backend/src/models/department.rs`
  - `/workspace/backend/src/services/department_service.rs`
  - `/workspace/backend/src/handlers/department_handler.rs`
  - `/workspace/backend/src/models/data_permission.rs`
  - `/workspace/backend/src/services/data_permission_service.rs`
  - `/workspace/backend/src/models/user.rs`
  - `/workspace/backend/src/models/custom_order.rs`
  - `/workspace/backend/src/utils/process_state_machine.rs`
  - `/workspace/backend/src/services/custom_order_crud_service.rs`
  - `/workspace/backend/src/services/custom_order_state_service.rs`
  - `/workspace/backend/src/services/custom_order_quality_service.rs`
  - `/workspace/backend/src/services/custom_order_process_service.rs`
  - `/workspace/backend/src/services/custom_order_aftersales_service.rs`
  - `/workspace/backend/src/models/after_sales.rs`
  - `/workspace/backend/src/models/lab_dip_request.rs`
  - `/workspace/backend/src/models/logistics_waybill.rs`
  - `/workspace/backend/src/handlers/logistics_handler.rs`
  - `/workspace/backend/src/models/sales_delivery.rs`
  - `/workspace/backend/src/services/so/delivery.rs`
  - `/workspace/backend/src/models/status.rs`
  - `/workspace/backend/src/utils/incoterms.rs`
  - `/workspace/backend/src/models/sales_quotation.rs`
  - `/workspace/backend/src/services/quotation_service.rs`
- **审计方法**：Read 审计计划 + Grep 检索 + Read 关键文件 + 对照审计计划核对
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码

---

## 维度 23.1：组织架构部门管理审计

### 检查方法
1. 读取 `backend/src/models/department.rs`、`backend/src/services/department_service.rs`、`backend/src/handlers/department_handler.rs` 核对部门树形结构、变更审计
2. 读取 `backend/src/models/data_permission.rs`、`backend/src/services/data_permission_service.rs` 核对部门权限关联
3. 读取 `backend/src/models/user.rs` 核对部门与用户关联
4. Grep 检索 `user_department` / `secondary_department` / `merge_department` / `split_department` / `apply_data_permission` / `filter_by_department` 等关键标识

### 发现

#### ✅ 已落实的项

1. **部门树形结构**（部分实现）
   - `backend/src/models/department.rs:15` `pub parent_id: Option<i32>` 支持父部门
   - `backend/src/models/department.rs:24-36` Relation 同时定义 `Parent`（belongs_to）与 `Children`（has_many），具备树形关系
   - `backend/src/services/department_service.rs:214-254` `get_department_tree` 方法递归构建 `DepartmentTreeNode` 树形结构
   - `backend/src/services/department_service.rs:183-190` 删除部门时检查子部门存在性，禁止带子部门删除
   - `backend/src/handlers/department_handler.rs:48-55` 暴露 `get_department_tree` 路由
   - **支持多级部门 ✅**

2. **部门与用户关联**（基础实现）
   - `backend/src/models/user.rs:18` `pub department_id: Option<i32>` 用户归属部门
   - `backend/src/models/user.rs:42-47` Relation `Department` belongs_to，关系存在
   - `backend/src/services/department_service.rs:193-203` 删除部门前检查是否有用户关联，禁止带用户删除

3. **部门变更审计**（部分实现）
   - `backend/src/services/department_service.rs:167-173` `update` 方法调用 `AuditLogService::update_with_audit(self.db.as_ref(), "departments", dept, Some(user_id))`，更新有审计 ✅
   - `backend/src/services/department_service.rs:206-211` `delete` 方法调用 `AuditLogService::delete_with_audit::<DepartmentEntity, _>(&*self.db, "department", id, Some(user_id))`，删除有审计 ✅
   - 批次 94 P2-10 已修复占位符 `Some(0)` 为真实 `user_id`

#### ❌ 缺陷项

##### 缺陷 1：部门与权限关联未实际落地（部门负责人不能看本部门数据）
**风险等级：P1**
**证据**：
- `backend/src/models/data_permission.rs:20` 字段注释明确："数据范围类型：ALL-全部, DEPT-本部门, DEPT_AND_BELOW-本部门及以下, SELF-仅本人, CUSTOM-自定义"
- `backend/src/services/data_permission_service.rs:16-23` 第 18-20 行注释："批次 119 P2-5 修复：删除 4 个未接入业务的 scope 常量（DEPT/DEPT_AND_BELOW/SELF/CUSTOM），仅保留 ALL（admin 角色使用）"
- `backend/src/services/data_permission_service.rs:50-90` `get_role_data_permission` 仅在 admin 角色时返回 `scope_type=ALL`，其他角色只是从表中读出 scope_type 但**没有任何业务代码根据 DEPT/DEPT_AND_BELOW 做行级数据过滤**
- Grep `apply_data_permission|filter_by_department|dept_scope` 全项目无匹配
- Grep `scope_type.*DEPT|DEPT_AND_BELOW` 仅命中 `data_permission_service.rs:18`（注释）和 `data_permission_handler.rs:151,263`（仅作枚举校验/下拉枚举），无业务过滤实现
**业务影响**：审计计划 23.1 第 2 项要求"部门必须关联数据权限（部门负责人能看本部门数据）"。当前 `data_permissions` 表只是空架子，没有任何 service/handler 根据 DEPT/DEPT_AND_BELOW scope_type 做查询过滤。销售订单、采购订单、客户、库存等业务数据全部按用户/角色过滤，部门负责人无法看到本部门下属员工数据，存在数据隔离失效或过度隔离风险。
**修复建议**：
1. 在 `DataPermissionService` 增加 `apply_dept_scope_filter<E>(query, user_id, resource_type)` 工具方法
2. 在主要业务 service（如 sales_order/purchase_order/customer）的 list 查询前调用该方法
3. 为部门负责人角色配置 scope_type=DEPT_AND_BELOW 数据权限规则

##### 缺陷 2：用户不支持一人多部门（主部门+兼职）
**风险等级：P1**
**证据**：
- `backend/src/models/user.rs:18` 仅 `pub department_id: Option<i32>` 一个部门字段
- Grep `user_department|user_departments|secondary_department|主部门|兼职|concurrent_position` 全项目无匹配
- 无 `user_departments` 关联表，无 `is_primary` 标识字段
**业务影响**：审计计划 23.1 第 3 项要求"用户必须归属部门，支持一人多部门（主部门+兼职）"。面料行业实际场景：染色车间主管可能同时兼职品质管理部、跟单员可能跨生产部+销售部。当前模型一人一部门，无法表达兼职关系，导致兼职人员的数据权限只能按主部门算，跨部门协作时数据隔离失效。
**修复建议**：
1. 新增 `user_departments` 关联表：`user_id / department_id / is_primary / start_date / end_date`
2. 修改 `data_permission_service` 在过滤部门数据时查询 `user_departments` 获取用户所有部门（含兼职）
3. UI 端用户编辑界面支持选择多个部门并标注主部门

##### 缺陷 3：部门创建未审计
**风险等级：P2**
**证据**：
- `backend/src/services/department_service.rs:79-120` `create` 方法仅调用 `active_model.insert(&*self.db).await?` 直接插入，**未调用任何审计方法**
- 对比：`update`（第 167 行）调用 `update_with_audit`，`delete`（第 206 行）调用 `delete_with_audit`，二者均有审计
- `backend/src/services/audit_log_service.rs:1-425` AuditLogService 仅提供 `update_with_audit`、`delete_with_audit`、`delete_with_audit_i64`、`record`，**没有 `create_with_audit` 方法**
**业务影响**：审计计划 23.1 第 4 项要求"部门变更（新建/合并/拆分/撤销）必须审计，影响范围可追溯"。新建部门不审计将导致：组织架构调整无追溯记录、新部门创建人不可查、新建时的初始 parent_id/manager_id 等关键字段无变更前后对比。update 和 delete 有审计而 create 没有，审计链路不完整。
**修复建议**：
1. 在 `AuditLogService` 增加 `create_with_audit` 方法（参考 `update_with_audit` 模板）
2. 在 `department_service.create` 中调用该方法记录新建事件
3. 同步排查其他业务模块的 create 方法是否同样缺失审计（custom_order/after_sales/logistics_waybill 等）

##### 缺陷 4：部门负责人（manager_id）功能未实现
**风险等级：P2**
**证据**：
- `backend/src/models/department.rs:16` 字段 `pub manager_id: Option<i32>` 存在
- `backend/src/services/department_service.rs:110` `manager_id: Set(None)` 创建时强制为 None
- `backend/src/handlers/department_handler.rs:22-37` `CreateDepartmentRequest` 和 `UpdateDepartmentRequest` DTO **均无 manager_id 字段**
- `backend/src/services/department_service.rs:122-175` `update` 方法**未处理 manager_id** 字段
- Grep `manager_id` 仅命中 `department.rs:16`（模型字段）和 `department_service.rs:110`（占位赋值）
**业务影响**：审计计划 23.1 第 2 项要求"部门负责人能看本部门数据"。当前 `manager_id` 字段虽存在但创建/更新接口都无法设置，且无业务代码读取该字段做权限判断。部门负责人功能完全空架子，影响"部门数据权限按 manager_id 授权"的链路完整性。
**修复建议**：
1. 在 `CreateDepartmentRequest` 和 `UpdateDepartmentRequest` 增加 `manager_id: Option<i32>` 字段
2. `department_service.create` 和 `update` 中持久化该字段（校验 manager_id 必须是有效 user.id）
3. `data_permission_service` 在 DEPT scope 过滤时优先认 manager_id 为本部门数据查看人

##### 缺陷 5：部门合并/拆分/撤销功能缺失
**风险等级：P2**
**证据**：
- Grep `merge_department|split_department|合并部门|拆分部门|撤销部门` 全项目无匹配
- `backend/src/handlers/department_handler.rs:1-55` 仅暴露标准 CRUD + 树形查询
- `backend/src/services/department_service.rs:1-264` 无 merge/split 方法
**业务影响**：审计计划 23.1 第 4 项要求"部门变更（新建/合并/拆分/撤销）必须审计，影响范围可追溯"。组织架构调整在面料企业常见（如"染整一车间"与"染整二车间"合并为"染整车间"），当前模型无合并/拆分接口，业务方只能手动新建+删除+迁移用户，过程无审计、用户迁移易遗漏、历史归属丢失。
**修复建议**：
1. 增加 `merge_departments(source_id, target_id, user_id)` 方法：迁移用户、子部门、删除源部门，全程落审计
2. 增加 `split_department(source_id, new_dept_list, user_mapping, user_id)` 方法：拆分部门并迁移用户
3. 增加 `deactivate_department(id, user_id)` 方法：软撤销（设置 `is_active=false`），保留历史数据

---

## 维度 23.2：定制订单流程与质量管控审计

### 检查方法
1. 读取 `backend/src/models/custom_order.rs`、`backend/src/utils/process_state_machine.rs` 核对定制订单流程
2. 读取 `backend/src/services/custom_order_crud_service.rs`、`custom_order_state_service.rs`、`custom_order_quality_service.rs`、`custom_order_process_service.rs` 核对状态机和质检
3. 读取 `backend/src/models/lab_dip_request.rs` 核对打样与定制订单关联
4. Grep `custom_order.*lab_dip|lab_dip.*custom_order|approval|审批|approve|customer_confirm|客户确认|custom.*quote` 等关键标识

### 发现

#### ✅ 已落实的项

1. **定制订单状态机**（5 阶段工艺流程）
   - `backend/src/utils/process_state_machine.rs:13-30` `CustomOrderStatus` 枚举定义 8 个状态：Draft / YarnPurchasing / Dyeing / Finishing / Delivery / AfterSales / Completed / Cancelled
   - `backend/src/utils/process_state_machine.rs:91-118` `next_status` 函数实现线性推进：draft → yarn_purchasing → dyeing → finishing → delivery → after_sales → completed
   - `backend/src/utils/process_state_machine.rs:121-140` `can_transition` 函数校验状态转换合法性，禁止跳级（如 draft → dyeing 直接跳过 yarn_purchasing 会失败）
   - `backend/src/utils/process_state_machine.rs:153-229` 含完整单元测试覆盖

2. **定制订单节点级追溯**（部分实现）
   - `backend/src/services/custom_order_state_service.rs:79-122` 推进状态时自动完成当前 in_progress 节点 + 启动下一节点
   - `backend/src/services/custom_order_process_service.rs:193-226` `get_timeline` 返回 `Vec<(process_node, Vec<process_log>)>` 时间线（已修复 N+1：批次 37 批量查询日志）
   - `backend/src/services/custom_order_state_service.rs:184-204` `list_logs` 提供订单全部日志查询
   - `backend/src/services/custom_order_state_service.rs:94-105` 状态推进时落 process_log 日志（含操作人/前后状态/备注）

3. **质量异常上报与解决**（部分实现）
   - `backend/src/services/custom_order_quality_service.rs:50-104` `report_issue` 上报异常，校验 severity（low/medium/high/critical）+ 色差 ΔE（GB/T 26377-2022）+ 色牢度等级（ISO 105 1-5）
   - `backend/src/services/custom_order_quality_service.rs:109-146` `resolve_issue` 解决异常，加 `lock_exclusive` 串行化并发解决（P2-6 修复），通过 `update_with_audit` 记录 operator_id

#### ❌ 缺陷项

##### 缺陷 1：定制订单流程缺失"打样"和"报价"环节
**风险等级：P0**
**证据**：
- 审计计划 23.2 第 1 项要求："定制订单必须走流程：需求确认→打样→报价→生产→质检→交付"
- `backend/src/utils/process_state_machine.rs:13-30` 状态枚举只有：Draft / YarnPurchasing / Dyeing / Finishing / Delivery / AfterSales / Completed / Cancelled
- 状态机首步是 `draft → yarn_purchasing`，**直接跳到纱线采购**，没有"需求确认/打样/报价"阶段
- `backend/src/models/lab_dip_request.rs:21-94` 打样通知单模型存在，含 customer_approved_at/customer_approval_comment/approved_sample_id 字段
- Grep `custom_order.*lab_dip|lab_dip.*custom_order` **无匹配**：打样通知单与定制订单无关联字段（lab_dip_request 表无 custom_order_id）
- `backend/src/models/custom_order.rs:12-37` custom_order 表也无 lab_dip_request_id 字段
- Grep `custom.*quote|quotation.*custom` 仅命中 `quotation_convert_service.rs:91`（quotation 转 sales_order，与定制订单无关）
**业务影响**：面料行业定制业务的核心是"先打样确认→再报价→再大货生产"。当前定制订单直接从草稿进入纱线采购阶段，完全跳过了打样和报价环节。打样通知单（lab_dip_request）作为独立模块存在，与定制订单无关联，导致：
- 客户签字确认的 OK 样（approved_sample_id）无法回流到定制订单作为生产依据
- 报价单（sales_quotation）与定制订单脱钩，金额只能手工填入 total_amount 字段
- 全链路追溯（需求→打样→生产→交付）断裂
**修复建议**：
1. 在 `CustomOrderStatus` 枚举中增加 `RequirementConfirm` / `LabDip` / `Quotation` 三个状态，状态机改为：`draft → requirement_confirm → lab_dip → quotation → yarn_purchasing → dyeing → finishing → delivery → after_sales → completed`
2. 在 `custom_order` 表增加 `lab_dip_request_id` 字段关联打样通知单
3. 在 `custom_order` 表增加 `quotation_id` 字段关联报价单，total_amount 从报价单自动同步
4. 增加状态门：未通过打样确认（approved_sample_id IS NULL）禁止推进到 yarn_purchasing

##### 缺陷 2：定制订单无客户签字确认机制
**风险等级：P1**
**证据**：
- 审计计划 23.2 第 2 项要求："定制订单必须有专属质量标准（客户签字确认），质检按客户标准"
- `backend/src/models/custom_order.rs:12-37` custom_order 表字段中**无 customer_approved_at / customer_approval_comment / signed_by_customer 等客户确认字段**
- Grep `customer_confirm|customer_sign|signed_by_customer|客户确认|签字确认` 仅在 `lab_dip_request.rs:74-78` 命中（lab_dip 有客户确认，custom_order 没有）
- `backend/src/services/custom_order_quality_service.rs:50-104` `report_issue` 使用的色差 ΔE 阈值 3.0 是通用标准（GB/T 26377-2022），**未按客户专属质量标准判定**
- `backend/src/models/quality_standard.rs` 存在通用质量标准表，但 custom_order 表无 `quality_standard_id` 字段关联
**业务影响**：面料行业定制业务中，不同客户对色差、色牢度、缩水率的要求差异巨大（如高端品牌要求 ΔE ≤ 1.5，普通客户接受 ΔE ≤ 3.0）。当前定制订单：
- 客户无法对定制订单整体签字确认（只能在打样阶段确认 OK 样）
- 质检只能按通用 GB/T 26377 标准，无法按客户专属标准
- 出现质量纠纷时无客户确认记录可追溯
**修复建议**：
1. 在 `custom_order` 表增加 `customer_approved_at / customer_approval_comment / quality_standard_id` 字段
2. 在 `custom_order_quality_service.report_issue` 中读取关联 `quality_standard` 的 ΔE/色牢度阈值，替代硬编码的 3.0
3. 增加客户签字确认 API（POST /custom-orders/:id/customer-approval），未确认禁止推进到 yarn_purchasing

##### 缺陷 3：定制订单变更无二级审批
**风险等级：P1**
**证据**：
- 审计计划 23.2 第 3 项要求："定制订单变更必须经客户确认 + 二级审批"
- `backend/src/services/custom_order_crud_service.rs:182-236` `update` 方法仅在 `existing.status == co_status::DRAFT` 时允许更新，**无审批流程**
- `backend/src/services/custom_order_crud_service.rs:239-274` `cancel` 方法直接修改状态为 CANCELLED，**无审批流程**
- `backend/src/models/custom_order.rs:39-77` Relation 中**无 BPM 关联**（无 approval_instance_id 字段）
- Grep `custom_order.*approval|custom_order.*审批` 在 custom_order 相关 service 中无匹配
- 对比：`backend/src/models/sales_quotation.rs:48-52` 报价单有 `approval_instance_id / approved_by / approved_at / rejection_reason` BPM 审批字段
**业务影响**：定制订单一旦进入生产阶段（yarn_purchasing 之后），任何变更（如改数量、改染色方法、取消订单）都可能造成已采购纱线浪费、已染色缸号报废。当前实现：
- 仅 draft 状态可改，进入生产后无法变更（业务上不合理，应有变更流程）
- cancel 直接生效无审批，员工误操作可能直接取消订单造成损失
- 无客户确认环节，变更后客户不知情
**修复建议**：
1. 在 `custom_order` 表增加 `approval_instance_id / approved_by / approved_at / rejection_reason` 字段
2. `update` 和 `cancel` 在非 draft 状态时改为创建 BPM 变更审批流程
3. 增加客户确认环节：变更审批通过后推送客户确认链接，客户确认后才执行变更
4. 增加 `custom_order_change_history` 表记录变更前后字段值（参考 sales_order_change_history 表）

##### 缺陷 4：定制订单全链路追溯不完整
**风险等级：P2**
**证据**：
- 审计计划 23.2 第 4 项要求："定制订单必须支持全链路追溯（需求→打样→生产→交付）"
- `backend/src/services/custom_order_process_service.rs:193-226` `get_timeline` 仅返回 `process_node + process_log`，**不含需求确认/打样/报价/交付环节**
- `backend/src/models/custom_order.rs:39-77` Relation 中有 ProcessNodes / QualityIssues / AfterSalesList，**无 LabDip / Quotation / SalesDelivery 关联**
- `backend/src/services/business_trace_service.rs` 存在通用追溯服务，但 Grep `custom_order.*trace` 在 business_trace_service 中无匹配
**业务影响**：定制订单的工艺节点追溯仅覆盖生产阶段（yarn_purchasing/ dyeing/ finishing/ delivery/ after_sales），无法回溯到需求确认、打样确认、报价审批环节，也不能正向关联到销售发货（sales_delivery）。出现质量纠纷时无法完整还原"客户提需求→打样确认→报价→生产→交付"全链路。
**修复建议**：
1. 在 `business_trace_service` 增加 custom_order 维度的链路聚合方法
2. `get_timeline` 扩展返回结构：包含 lab_dip_request / quotation / process_node / quality_issue / after_sales / sales_delivery 全部关联记录
3. 增加 `GET /custom-orders/:id/full-trace` 接口返回完整链路 JSON

---

## 维度 23.3：售后管理与工单流转审计

### 检查方法
1. 读取 `backend/src/models/after_sales.rs` 核对售后工单模型
2. 读取 `backend/src/services/custom_order_aftersales_service.rs` 核对售后流程和状态机
3. Grep `quality_issue_id|售后.*质量|8D|after_sales.*quality` 核对售后与质量集成

### 发现

#### ✅ 已落实的项

1. **售后工单类型校验**（4 类已实现但与审计计划要求不完全一致）
   - `backend/src/services/custom_order_aftersales_service.rs:75-81` 校验 `issue_type ∈ ["complaint", "repair", "exchange", "refund"]`
   - `backend/src/services/custom_order_aftersales_service.rs:83-88` 退款类型必须有 refund_amount
   - **见下方缺陷 1：类型不匹配**

2. **售后状态机**（部分实现）
   - `backend/src/services/custom_order_aftersales_service.rs:185-195` `is_valid_transition` 状态转换校验：
     - opened → processing / rejected / closed
     - processing → resolved / closed / rejected
     - resolved → closed
     - closed / rejected → 终态
   - `backend/src/services/custom_order_aftersales_service.rs:130-137` 状态变更时自动记录 closed_at
   - **见下方缺陷 2：流程步骤不完整**

#### ❌ 缺陷项

##### 缺陷 1：售后工单类型与审计计划要求的 4 类不匹配
**风险等级：P2**
**证据**：
- 审计计划 23.3 第 1 项要求："必须支持退货/换货/维修/投诉 4 类售后工单"
- `backend/src/services/custom_order_aftersales_service.rs:76` 实际支持：`["complaint", "repair", "exchange", "refund"]`（客诉/维修/换货/退款）
- **缺失"退货"（return_goods）类型**，多出"退款"（refund）类型
- 退货与退款是不同业务：退货涉及物流收货、库存回库，退款涉及财务出账，两者常常并存但不应混淆
- 当前实现将"退货"业务强行合并到"退款"或"换货"，无法表达"客户退货但不退款"等场景
**业务影响**：面料行业售后常见场景：客户收到布后发现色差，要求退货（物流回货）+ 退款（财务出账）+ 重新生产换货（新订单）。当前 4 类无法表达"退货"独立类型，导致：
- 退货物流单据无法归类
- 退货库存回库流程无法触发
- 售后类型统计失真
**修复建议**：
1. 在 `after_sales.issue_type` 增加 `return_goods`（退货）类型，校验列表改为 5 类：`["complaint", "repair", "exchange", "return_goods", "refund"]`
2. 或保留 4 类但改为：`["complaint", "repair", "exchange", "return_goods"]`（退货含退款场景，退款独立用 finance 模块处理）
3. 与审计计划对齐：至少包含"退货/换货/维修/投诉"4 类

##### 缺陷 2：售后流程闭环步骤不完整
**风险等级：P1**
**证据**：
- 审计计划 23.3 第 2 项要求："售后必须走流程：申请→受理→处理→确认→评价→关闭"（6 步）
- `backend/src/services/custom_order_aftersales_service.rs:185-195` 实际状态机只有 5 步：`opened → processing → resolved → closed / rejected`（申请→处理→解决→关闭/拒绝）
- **缺失"受理"（accepted）步骤**：客户申请后直接进入 processing，未体现客服受理环节
- **缺失"评价"（evaluated）步骤**：resolved 后直接 closed，无客户评价环节
- `backend/src/models/after_sales.rs:12-26` 表结构中**无 evaluation_score / evaluation_comment / evaluated_at 字段**
- `backend/src/services/custom_order_aftersales_service.rs:30-36` `UpdateAfterSalesDto` 也无评价字段
**业务影响**：售后流程不完整导致：
- 无法区分"客户已申请但客服未受理"和"客服正在处理"两种状态，工单分配效率低
- 客户无法对售后处理结果评价，企业无法度量售后满意度
- 售后 KPI（如平均受理时长、客户满意度）无法统计
**修复建议**：
1. 状态机改为 6 步：`opened → accepted → processing → resolved → evaluated → closed`
2. 在 `after_sales` 表增加 `accepted_at / accepted_by / evaluation_score（1-5） / evaluation_comment / evaluated_at` 字段
3. `UpdateAfterSalesDto` 增加对应字段
4. 增加客户评价 API（POST /after-sales/:id/evaluation）

##### 缺陷 3：售后原因分析与 TOP 5 月报缺失
**风险等级：P1**
**证据**：
- 审计计划 23.3 第 3 项要求："售后必须记录原因（质量/物流/客户偏好），月度 TOP 5 原因分析"
- `backend/src/models/after_sales.rs:12-26` 表结构中**无 reason / reason_category 字段**，仅有 description（描述）
- `backend/src/services/custom_order_aftersales_service.rs:19-28` `CreateAfterSalesDto` 也无 reason 字段
- Grep `after_sales.*reason|after_sales.*top|after_sales.*monthly|售后.*月报|售后.*top` 无匹配
- 无 `after_sales_monthly_report` 视图或服务
**业务影响**：售后原因分析是售后管理的核心 KPI。当前实现：
- 售后工单只能用 description 自由文本描述，无法结构化分类统计
- 无法按月统计 TOP 5 售后原因（如"色差超差"/"缸号混铺"/"物流破损"）
- 企业无法识别主要售后问题源头，无法针对性改进
**修复建议**：
1. 在 `after_sales` 表增加 `reason_category`（枚举：quality/logistics/customer_preference/other）和 `reason_detail`（结构化子类）字段
2. 增加 `after_sales_monthly_report` 服务：按月聚合 reason_category + reason_detail，输出 TOP 5
3. 增加 `GET /after-sales/monthly-report?year=2026&month=7` 接口

##### 缺陷 4：售后与质量集成缺失（无 quality_issue_id 关联，无 8D 流程）
**风险等级：P0**
**证据**：
- 审计计划 23.3 第 4 项要求："质量问题引发的售后必须关联 quality_issue，走 8D 流程"
- `backend/src/models/after_sales.rs:28-42` Relation 中**无 QualityIssue 关联**
- `backend/src/models/after_sales.rs:12-26` 表结构中**无 quality_issue_id 字段**
- `backend/src/models/quality_issue.rs` 存在独立的质量异常表（在 custom_order_quality_service 中上报/解决），**与售后模块完全隔离**
- Grep `8D|eight_d|d0|d1|d2|d3|d4|d5|d6|d7|d8` 在 after_sales 相关代码中无匹配
- 8D 流程（D0 准备/D1 团队/D2 描述问题/D3 临时措施/D4 根本原因/D5 永久措施/D6 实施验证/D7 预防措施/D8 团队表彰）无任何实现
**业务影响**：面料行业质量异常的售后处理是核心业务场景。当前实现：
- 客户投诉色差问题，售后工单无法关联到生产过程的质量异常记录（quality_issue）
- 无法追溯质量异常是否已通过 8D 流程闭环
- 售后处理与质量改进脱钩，相同质量问题可能反复发生
- 审计计划类二十一 21.4 要求的 8D 流程在售后侧完全缺失
**修复建议**：
1. 在 `after_sales` 表增加 `quality_issue_id` 字段关联 `quality_issue` 表
2. 售后创建时若 reason_category=quality，强制选择关联的 quality_issue_id
3. 新建 `quality_8d_report` 表：`quality_issue_id / d0_date / d1_team / d2_description / ... / d8_date / status`
4. 增加 8D 流程服务 `EightDService`，提供 `start_8d / update_step / close_8d` 方法
5. 售后关闭前校验关联的 quality_issue 是否已完成 8D 流程

---

## 维度 23.4：物流运单跟踪与运费核算审计

### 检查方法
1. 读取 `backend/src/models/logistics_waybill.rs` 核对运单模型字段
2. 读取 `backend/src/handlers/logistics_handler.rs` 核对运单业务逻辑
3. 读取 `backend/src/models/status.rs`（logistics_waybill 子模块）核对状态机
4. 读取 `backend/src/services/so/delivery.rs` 核对销售发货与应收确认
5. Grep `logistics_track|tracking_history|waybill_track|express_api|快递|ar_invoice|应收|receivable|sign_receipt|电子签收` 等关键标识

### 发现

#### ✅ 已落实的项

1. **运单基本管理**（部分实现）
   - `backend/src/models/logistics_waybill.rs:10-25` 字段含：order_id（关联 sales_order）/ logistics_company / tracking_number / driver_name / driver_phone / freight_fee / status / expected_arrival / actual_arrival / notes
   - `backend/src/handlers/logistics_handler.rs:31-76` `create_waybill` 创建运单并同步更新销售订单状态为 SHIPPED
   - `backend/src/handlers/logistics_handler.rs:94-115` `update_waybill_status` 更新运单状态，DELIVERED 时自动设置 actual_arrival
   - `backend/src/handlers/logistics_handler.rs:129-156` `delete_waybill` 删除时检查状态（IN_TRANSIT/DELIVERED 不可删除），并通过 `delete_with_audit` 落审计
   - `backend/src/models/status.rs:324-330` 状态常量化：IN_TRANSIT / DELIVERED（批次 232 v13 P1-1 修复）

2. **运单有 tracking_number 字段**（基础实现）
   - `backend/src/models/logistics_waybill.rs:15` `pub tracking_number: String` 字段存在
   - **见下方缺陷 2：仅存储单号，无跟踪历史**

3. **销售发货 → 应收确认集成**（在 sales_delivery 侧实现，与 logistics_waybill 解耦）
   - `backend/src/services/so/delivery.rs:378-400` `ship_order` 全额发货时调用 `ar_service.create_receivable` 生成 AR 应收账款
   - `backend/src/services/so/delivery.rs:405-495` 自动生成收入确认凭证（借应收账款 / 贷主营业务收入+销项税）
   - **见下方缺陷 4：logistics_waybill 签收与应收确认脱钩**

#### ❌ 缺陷项

##### 缺陷 1：运单未关联采购订单，不支持多订单合并发货
**风险等级：P1**
**证据**：
- 审计计划 23.4 第 1 项要求："运单必须关联销售订单/采购订单，支持多订单合并发货"
- `backend/src/models/logistics_waybill.rs:13` `pub order_id: i32` 仅关联 sales_order
- `backend/src/models/logistics_waybill.rs:27-35` Relation 只有 `SalesOrder`，**无 PurchaseOrder 关联**
- `backend/src/handlers/logistics_handler.rs:38-41` 创建运单时仅校验 sales_order 存在
- 无 `logistics_waybill_order` 关联表支持多订单合并
**业务影响**：
- 采购入库的物流单据无法管理（如供应商发货 → 公司仓库的运输单）
- 多订单合并发货场景无法表达（如同一客户多个小订单合并发一车）
- 物流费用无法按多订单分摊
**修复建议**：
1. 在 `logistics_waybill` 表增加 `order_type` 字段（sales_order/purchase_order/transfer_order）
2. 或新建 `logistics_waybill_orders` 关联表：`waybill_id / order_type / order_id / quantity / amount`
3. `create_waybill` 支持传入订单列表，自动创建关联记录

##### 缺陷 2：物流跟踪无历史记录，未对接快递 API
**风险等级：P1**
**证据**：
- 审计计划 23.4 第 2 项要求："必须支持物流跟踪（对接快递 API 或手工录入），状态实时更新"
- `backend/src/models/logistics_waybill.rs:15` 仅 `tracking_number: String` 字段存储单号
- **无 `logistics_tracking_event` 表**存储跟踪历史（如"已揽收/运输中/到达转运中心/派送中/已签收"）
- `backend/src/handlers/logistics_handler.rs:94-115` `update_waybill_status` 只更新整体 status，无历史时间线
- Grep `logistics_track|tracking_history|waybill_track|express_api|快递` **全项目无匹配**
- 无快递 API 对接代码（如顺丰/京东/中通开放 API）
**业务影响**：
- 客户无法查询物流轨迹，只能看到"运输中/已送达"两个状态
- 出现物流异常（如滞留、丢件）时无法追溯历史节点
- 客服需要手工查询快递公司官网，效率低下
**修复建议**：
1. 新建 `logistics_tracking_event` 表：`waybill_id / event_time / location / description / event_type`
2. 增加 `update_tracking` API 支持手工录入跟踪事件
3. 集成主流快递 API（如快递鸟/快递 100），自动同步跟踪事件
4. 增加 `GET /logistics/waybills/:id/tracking` 返回完整轨迹

##### 缺陷 3：运费核算缺少按重量/体积/距离的核算逻辑
**风险等级：P1**
**证据**：
- 审计计划 23.4 第 3 项要求："运费必须按重量/体积/距离核算，支持客户承担/公司承担分摊"
- `backend/src/models/logistics_waybill.rs:18` `pub freight_fee: Option<Decimal>` 仅一个总运费字段
- **无 weight / volume / distance / freight_rate / freight_bearer 字段**
- `backend/src/handlers/logistics_handler.rs:44-46` 创建运单时 `freight_fee` 直接从请求 DTO 传入，无核算逻辑
- Grep `freight.*calc|freight_rate|freight_bearer|运费.*核算|运费.*承担` **全项目无匹配**
**业务影响**：
- 运费只能手工填入，无法根据重量/体积/距离自动核算
- 无法区分客户承担运费和公司承担运费，财务无法正确归集销售费用
- 多订单合并发货时运费无法分摊
- 物流成本分析无数据基础
**修复建议**：
1. 在 `logistics_waybill` 表增加 `total_weight / total_volume / distance_km / freight_rate / freight_bearer（customer/company）` 字段
2. 增加 `calculate_freight` 方法：根据 weight × rate 或 volume × rate 或 distance × rate 自动核算
3. `freight_bearer=customer` 时将运费计入销售订单 shipping_cost，由客户支付
4. `freight_bearer=company` 时将运费计入销售费用科目

##### 缺陷 4：物流签收无电子签收单，签收未触发应收确认
**风险等级：P0**
**证据**：
- 审计计划 23.4 第 4 项要求："必须支持电子签收（上传签收单），签收后自动触发应收确认"
- `backend/src/models/logistics_waybill.rs:10-25` 表结构中**无 sign_receipt_url / signed_by / signed_at / sign_photo 字段**
- `backend/src/handlers/logistics_handler.rs:94-115` `update_waybill_status` 中 DELIVERED 仅设置 `actual_arrival = Some(Utc::now())`，**无签收单上传逻辑**
- Grep `sign_receipt|电子签收|签收单|signed_by` **全项目无匹配**
- 应收确认逻辑在 `backend/src/services/so/delivery.rs:378-400` 的 `ship_order` 中触发（基于销售发货），**与 logistics_waybill 签收完全脱钩**
- 即客户实际签收与否不影响 AR 应收确认时点，存在"未签收已确认应收"的风险
**业务影响**：
- 客户签收无电子凭证，出现物流纠纷时无法举证
- 签收时点不触发应收确认，导致应收账款时点不准确（按发货确认而非签收确认，违反收入确认原则）
- 客户拒收/部分签收场景无法处理
**修复建议**：
1. 在 `logistics_waybill` 表增加 `sign_receipt_url / signed_by / signed_at / sign_photo_url / sign_remark` 字段
2. 增加 `confirm_sign` API：上传签收单图片 + 签收人信息
3. 签收事件触发应收确认：调用 `ar_service.create_receivable`（替代当前在 ship_order 中的应收确认逻辑）
4. 签收异常（拒收/部分签收）触发售后工单自动创建

##### 缺陷 5：logistics_waybill 与 sales_delivery 双系统并存且未集成
**风险等级：P2**
**证据**：
- `backend/src/models/logistics_waybill.rs` 物流运单表（11 字段）
- `backend/src/models/sales_delivery.rs:13-58` 销售交货表（10 字段，独立于 logistics_waybill）
- 两套表均关联 sales_order，但字段不互通：
  - sales_delivery 有 delivery_no / customer_id / warehouse_id / total_quantity / total_amount，logistics_waybill 无
  - logistics_waybill 有 logistics_company / tracking_number / driver_name / freight_fee，sales_delivery 无
- `backend/src/services/so/delivery.rs:109-514` `ship_order` 完整发货流程操作 sales_delivery，**不创建 logistics_waybill**
- `backend/src/handlers/logistics_handler.rs:31-76` `create_waybill` 创建 logistics_waybill，**不操作 sales_delivery**
**业务影响**：
- 业务方需要在两个系统分别录入发货信息，数据冗余
- 发货统计报表口径不一致（按 sales_delivery 还是 logistics_waybill 统计）
- 库存扣减只在 sales_delivery 侧（reduce_inventory），logistics_waybill 创建不扣库存，存在库存账实不符风险
**修复建议**：
1. 合并两套表为统一 `sales_delivery` 表，增加 logistics_company / tracking_number / driver_name / freight_fee 等物流字段
2. 或保留两表但建立关联：`logistics_waybill.sales_delivery_id` 字段
3. `ship_order` 创建 sales_delivery 后自动创建关联的 logistics_waybill

---

## 维度 23.5：国际贸易术语 incoterms 完整性审计

### 检查方法
1. 读取 `backend/src/utils/incoterms.rs` 核对术语枚举完整性
2. 读取 `backend/src/models/sales_quotation.rs` 核对术语与价格集成
3. 读取 `backend/src/services/quotation_service.rs`（第 500-545 行）核对术语校验逻辑
4. Grep `incoterm.*report|incoterm.*monthly|incoterm.*statistics|贸易术语.*月报` 核对术语报表

### 发现

#### ✅ 已落实的项

1. **Incoterms 工具模块存在**（部分实现）
   - `backend/src/utils/incoterms.rs:1-91` 提供 `Incoterms2020` 枚举与解析工具
   - `backend/src/utils/incoterms.rs:33-42` `from_code` 支持大小写不敏感解析
   - `backend/src/utils/incoterms.rs:55-67` 提供 `includes_insurance / includes_freight / requires_duty_paid` 业务属性判断
   - `backend/src/utils/incoterms.rs:71-79` `description` 提供中文业务描述
   - `backend/src/utils/incoterms.rs:93-140` 含完整单元测试
   - `backend/src/services/quotation_service.rs:535-544` `validate_price_terms` 接入工具校验，错误信息派生合法代码列表

2. **报价单存储 Incoterms 字段**（基础实现）
   - `backend/src/models/sales_quotation.rs:27-29` 字段：`price_terms: String` / `incoterms_version: Option<String>` / `incoterm_location: Option<String>`
   - `backend/src/services/quotation_service.rs:96-97` 创建报价单时持久化 incoterms 字段
   - `backend/src/services/quotation_service.rs:289-293` 更新报价单时持久化 incoterms 字段
   - `backend/src/services/quotation_service.rs:519-526` 创建报价单时校验 incoterm 合法性并记录业务元数据到日志（含 includes_insurance / includes_freight / requires_duty_paid）

#### ❌ 缺陷项

##### 缺陷 1：Incoterms 2020 仅支持 5 种，缺失 6 种
**风险等级：P0**
**证据**：
- 审计计划 23.5 第 1 项要求："必须支持 11 种国际贸易术语（EXW/FCA/CPT/CIP/DAP/DPU/DDP/FAS/FOB/CFR/CIF）"
- `backend/src/utils/incoterms.rs:18-29` `Incoterms2020` 枚举仅 5 个变体：Fob / Cif / Exw / Ddp / Dap
- `backend/src/utils/incoterms.rs:82-90` `all()` 方法返回 5 个术语
- `backend/src/utils/incoterms.rs:34-41` `from_code` 仅识别 5 个代码
- **缺失 6 种术语**：FCA（Free Carrier）/ CPT（Carriage Paid To）/ CIP（Carriage and Insurance Paid To）/ DPU（Delivered at Place Unloaded）/ FAS（Free Alongside Ship）/ CFR（Cost and Freight）
**业务影响**：Incoterms 2020 是国际贸易核心标准，11 种术语覆盖不同贸易场景：
- FCA 是最常见的集装箱贸易术语（替代 FOB 用于集装箱运输）
- CPT/CIP 是国际快递/空运常用术语
- CFR 是海运常用术语（与 CIF 区别在于不含保险）
- DPU 是 DAP 的变种（卖方负责卸货）
- 当前只支持 5 种，导致：
  - 集装箱贸易报价单无法使用 FCA，被迫用 FOB（FOB 不适用集装箱，存在风险转移点争议）
  - 国际空运报价无法使用 CPT/CIP
  - 报价单的术语选择不符合实际贸易场景，可能引发国际贸易纠纷
**修复建议**：
1. 在 `Incoterms2020` 枚举增加 6 个变体：Fca / Cpt / Cip / Dpu / Fas / Cfr
2. `from_code` / `code` / `all` 同步增加 6 种术语支持
3. 调整 `includes_insurance / includes_freight / requires_duty_paid` 的判断逻辑：
   - includes_insurance: CIF / CIP / DDP
   - includes_freight: 除 EXW / FCA / FAS 外都包含
   - requires_duty_paid: 仅 DDP
4. 增加 `risk_transfer_point` 方法返回风险转移点描述

##### 缺陷 2：术语与价格构成未集成
**风险等级：P1**
**证据**：
- 审计计划 23.5 第 2 项要求："不同术语必须关联不同价格构成（如 FOB 不含运费/保费，CIF 含运费/保费）"
- `backend/src/models/sales_quotation.rs:41-43` 字段 `subtotal / tax_amount / total_amount` 仅基础金额，**无 freight_cost / insurance_cost / duty_cost 字段拆分**
- `backend/src/services/quotation_service.rs` 全文检索 `calculate_amount` 等金额计算方法，**未读取 incoterm 调整运费/保费**
- `backend/src/utils/incoterms.rs:55-67` `includes_insurance / includes_freight / requires_duty_paid` 方法存在但**仅用于日志记录**（`quotation_service.rs:519-526` 只 tracing::info），未参与金额计算
- 报价单 subtotal 和 total_amount 直接由 items 累加，不区分术语
**业务影响**：
- FOB 报价应只含货物成本（不含运费保费），但当前 subtotal 计算与 CIF 完全相同
- CIF 报价应 = 货物成本 + 运费 + 保费，但当前实现无运费/保费字段
- DDP 报价应 = 货物成本 + 运费 + 保费 + 关税，但当前无关税字段
- 报价金额不反映术语差异，国际贸易中可能引发严重商务纠纷（如客户按 FOB 付款但收到含运费的 CIF 报价金额）
**修复建议**：
1. 在 `sales_quotation` 表增加 `freight_cost / insurance_cost / duty_cost / other_cost` 字段
2. 增加 `calculate_amount_by_incoterm` 方法：根据 incoterm 自动计算各成本项
   - EXW: 仅货物成本
   - FOB/FAS: 货物成本 + 装船前费用
   - CFR/CPT: 货物成本 + 运费
   - CIF/CIP: 货物成本 + 运费 + 保费
   - DAP/DPU: 货物成本 + 运费 + 保费 + 目的地费用
   - DDP: 货物成本 + 运费 + 保费 + 关税
3. `total_amount = subtotal + freight_cost + insurance_cost + duty_cost + other_cost + tax_amount`
4. 报价单 PDF 导出时显示价格构成明细

##### 缺陷 3：术语风险转移点/责任划分无结构化字段
**风险等级：P2**
**证据**：
- 审计计划 23.5 第 3 项要求："必须明确风险转移点/费用承担方/出口进口清关责任"
- `backend/src/utils/incoterms.rs:71-79` `description` 方法返回中文业务描述（如 "装运港船上交货（卖方承担装船前费用和风险）"），是**自然语言描述**
- **无结构化字段**：无 `risk_transfer_point` / `cost_bearer` / `export_clearance_party` / `import_clearance_party` 等字段
- `includes_insurance / includes_freight / requires_duty_paid` 仅布尔判断，无法区分"卖方承担运费到目的港"vs"卖方承担运费到目的地"等细节
**业务影响**：
- 报价单/合同中无法自动生成结构化的责任划分条款
- 业务人员需要凭经验理解术语含义，存在理解偏差风险
- 出现贸易纠纷时无系统化的责任划分依据
- 国际贸易合规审查无数据基础
**修复建议**：
1. 在 `Incoterms2020` 增加 `risk_transfer_point() -> &str` 方法：返回风险转移点（如 FOB 返回"装运港船上"）
2. 增加 `cost_bearer() -> CostBearer` 方法：返回结构化的费用承担方（卖方/买方/共担）
3. 增加 `export_clearance_party() -> Party` / `import_clearance_party() -> Party` 方法
4. 报价单生成时输出结构化责任划分表

##### 缺陷 4：术语使用月报缺失
**风险等级：P1**
**证据**：
- 审计计划 23.5 第 4 项要求："必须有术语使用月报（按术语统计出口量/金额），支持合规审查"
- Grep `incoterm.*report|incoterm.*monthly|incoterm.*statistics|贸易术语.*月报|术语.*月报` **全项目无业务代码匹配**（仅命中审计计划文件本身和 CHANGELOG）
- `backend/src/services/report/` 模块下无 incoterms 相关报表
- `backend/src/services/finance_report_service.rs` 无术语维度统计
- `backend/src/services/bi_analysis_service.rs` 无术语维度分析
**业务影响**：
- 企业无法按月统计各术语的出口量和金额
- 国际贸易合规审查无报表支撑（如海关合规、税务合规需要按术语统计）
- 无法识别术语使用趋势（如 FOB 占比上升可能反映贸易条款变化）
- 管理层无法基于术语维度做贸易策略决策
**修复建议**：
1. 新建 `incoterm_monthly_report` 视图：按月份 + incoterm 聚合 sales_quotation 的 total_amount 和订单数
2. 在 `finance_report_service` 或 `sales_analysis_service` 增加 `incoterm_monthly_report(year, month)` 方法
3. 增加 `GET /reports/incoterm-monthly?year=2026&month=7` 接口
4. 前端增加术语使用月报看板（柱状图 + 饼图 + 趋势图）

---

## 审计结果汇总

| 维度 | P0 | P1 | P2 | P3 | 已落实 | 总检查项 |
|------|----|----|----|----|--------|----------|
| 23.1 组织架构部门管理 | 0 | 2 | 3 | 0 | 3 | 8 |
| 23.2 定制订单流程与质量管控 | 1 | 2 | 1 | 0 | 3 | 7 |
| 23.3 售后管理与工单流转 | 1 | 2 | 1 | 0 | 2 | 6 |
| 23.4 物流运单跟踪与运费核算 | 1 | 3 | 1 | 0 | 3 | 8 |
| 23.5 国际贸易术语 incoterms 完整性 | 1 | 2 | 1 | 0 | 2 | 6 |
| **合计** | **4** | **11** | **7** | **0** | **13** | **35** |

## 修复优先级队列

### P0（阻塞，4 项）

1. **23.2 缺陷 1**：定制订单流程缺失"打样"和"报价"环节
   - 文件：`backend/src/utils/process_state_machine.rs`、`backend/src/models/custom_order.rs`
   - 影响：面料行业定制业务核心流程缺失，打样和报价与定制订单脱钩
   - 修复：状态机增加 RequirementConfirm/LabDip/Quotation 阶段，custom_order 表增加 lab_dip_request_id/quotation_id 字段

2. **23.3 缺陷 4**：售后与质量集成缺失（无 quality_issue_id 关联，无 8D 流程）
   - 文件：`backend/src/models/after_sales.rs`、`backend/src/services/custom_order_aftersales_service.rs`
   - 影响：售后与质量异常隔离，无 8D 闭环，质量改进断链
   - 修复：after_sales 表增加 quality_issue_id，新建 quality_8d_report 表与 EightDService

3. **23.4 缺陷 4**：物流签收无电子签收单，签收未触发应收确认
   - 文件：`backend/src/models/logistics_waybill.rs`、`backend/src/handlers/logistics_handler.rs`
   - 影响：签收无凭证，应收确认时点错误（按发货而非签收）
   - 修复：logistics_waybill 增加 sign_receipt_url/signed_by/signed_at 字段，签收事件触发 AR 应收确认

4. **23.5 缺陷 1**：Incoterms 2020 仅支持 5 种，缺失 6 种（FCA/CPT/CIP/DPU/FAS/CFR）
   - 文件：`backend/src/utils/incoterms.rs`
   - 影响：集装箱/空运/海运常用术语无法使用，国际贸易纠纷风险
   - 修复：Incoterms2020 枚举增加 6 个变体，同步 from_code/code/all/includes_*/requires_*

### P1（高，11 项）

1. **23.1 缺陷 1**：部门与权限关联未实际落地（部门负责人不能看本部门数据）
   - 文件：`backend/src/services/data_permission_service.rs`
   - 修复：增加 `apply_dept_scope_filter` 工具方法，业务 service 查询前调用

2. **23.1 缺陷 2**：用户不支持一人多部门（主部门+兼职）
   - 文件：`backend/src/models/user.rs`
   - 修复：新建 `user_departments` 关联表，data_permission_service 查询用户所有部门

3. **23.2 缺陷 2**：定制订单无客户签字确认机制
   - 文件：`backend/src/models/custom_order.rs`、`backend/src/services/custom_order_quality_service.rs`
   - 修复：custom_order 表增加 customer_approved_at/quality_standard_id 字段，质检按客户专属标准

4. **23.2 缺陷 3**：定制订单变更无二级审批
   - 文件：`backend/src/services/custom_order_crud_service.rs`
   - 修复：custom_order 表增加 approval_instance_id，update/cancel 改走 BPM 流程

5. **23.3 缺陷 2**：售后流程闭环步骤不完整（缺受理和评价环节）
   - 文件：`backend/src/services/custom_order_aftersales_service.rs`、`backend/src/models/after_sales.rs`
   - 修复：状态机改为 6 步 opened→accepted→processing→resolved→evaluated→closed

6. **23.3 缺陷 3**：售后原因分析与 TOP 5 月报缺失
   - 文件：`backend/src/models/after_sales.rs`
   - 修复：after_sales 表增加 reason_category/reason_detail，新建 after_sales_monthly_report 服务

7. **23.4 缺陷 1**：运单未关联采购订单，不支持多订单合并发货
   - 文件：`backend/src/models/logistics_waybill.rs`
   - 修复：增加 order_type 字段或新建 logistics_waybill_orders 关联表

8. **23.4 缺陷 2**：物流跟踪无历史记录，未对接快递 API
   - 文件：`backend/src/models/logistics_waybill.rs`、`backend/src/handlers/logistics_handler.rs`
   - 修复：新建 logistics_tracking_event 表，集成快递 API 自动同步轨迹

9. **23.4 缺陷 3**：运费核算缺少按重量/体积/距离的核算逻辑
   - 文件：`backend/src/models/logistics_waybill.rs`、`backend/src/handlers/logistics_handler.rs`
   - 修复：增加 weight/volume/distance/freight_bearer 字段，实现 calculate_freight 方法

10. **23.5 缺陷 2**：术语与价格构成未集成
    - 文件：`backend/src/models/sales_quotation.rs`、`backend/src/services/quotation_service.rs`
    - 修复：sales_quotation 表增加 freight_cost/insurance_cost/duty_cost 字段，按 incoterm 自动核算

11. **23.5 缺陷 4**：术语使用月报缺失
    - 文件：`backend/src/services/finance_report_service.rs`
    - 修复：新建 incoterm_monthly_report 视图，增加 GET /reports/incoterm-monthly 接口

### P2（中，7 项）

1. **23.1 缺陷 3**：部门创建未审计
   - 文件：`backend/src/services/department_service.rs`、`backend/src/services/audit_log_service.rs`
   - 修复：AuditLogService 增加 create_with_audit 方法，department_service.create 调用

2. **23.1 缺陷 4**：部门负责人（manager_id）功能未实现
   - 文件：`backend/src/handlers/department_handler.rs`、`backend/src/services/department_service.rs`
   - 修复：CreateDepartmentRequest/UpdateDepartmentRequest 增加 manager_id 字段

3. **23.1 缺陷 5**：部门合并/拆分/撤销功能缺失
   - 文件：`backend/src/services/department_service.rs`
   - 修复：增加 merge_departments/split_department/deactivate_department 方法

4. **23.2 缺陷 4**：定制订单全链路追溯不完整
   - 文件：`backend/src/services/custom_order_process_service.rs`、`backend/src/services/business_trace_service.rs`
   - 修复：get_timeline 扩展返回 lab_dip/quotation/delivery 关联记录

5. **23.3 缺陷 1**：售后工单类型与审计计划要求的 4 类不匹配
   - 文件：`backend/src/services/custom_order_aftersales_service.rs`
   - 修复：issue_type 增加 return_goods 类型，对齐审计计划要求

6. **23.4 缺陷 5**：logistics_waybill 与 sales_delivery 双系统并存且未集成
   - 文件：`backend/src/models/logistics_waybill.rs`、`backend/src/models/sales_delivery.rs`
   - 修复：合并两表或建立 sales_delivery_id 关联字段

7. **23.5 缺陷 3**：术语风险转移点/责任划分无结构化字段
   - 文件：`backend/src/utils/incoterms.rs`
   - 修复：增加 risk_transfer_point/cost_bearer/export_clearance_party/import_clearance_party 方法

### P3（低，0 项）

无 P3 级别缺陷。

---

## 审计总结

### 已落实情况

类二十三组织定制物流审计专项 5 维度共检查 35 项，已落实 13 项（37%），存在 22 项缺陷（63%）。已落实项主要集中在：
- 基础数据模型字段存在（部门 parent_id、tracking_number、freight_fee、incoterms_version 等）
- 状态机基础校验（部门状态机、定制订单状态机、售后状态机、运单状态机）
- 单元测试覆盖（incoterms 完整测试、process_state_machine 完整测试）

### 主要风险

1. **P0 级缺陷 4 项，均为业务核心功能缺失或重大合规风险**：
   - 定制订单核心流程缺失打样报价环节
   - 售后与质量异常无 8D 闭环
   - 物流签收无电子凭证且未触发应收确认（违反收入确认原则）
   - Incoterms 仅支持 5 种（缺 6 种常用术语）

2. **P1 级缺陷 11 项，影响业务完整性和数据准确性**：
   - 数据权限未实际落地（部门负责人无法看本部门数据）
   - 一人多部门不支持
   - 定制订单无客户签字确认和变更审批
   - 售后流程缺受理和评价环节
   - 售后原因无结构化分类和月报
   - 物流运单缺多订单合并、跟踪历史、运费核算
   - Incoterms 与价格构成未集成
   - Incoterms 月报缺失

3. **P2 级缺陷 7 项，影响业务便利性和数据完整性**：
   - 部门创建未审计
   - 部门负责人字段未接入
   - 部门合并/拆分/撤销功能缺失
   - 定制订单全链路追溯不完整
   - 售后类型与审计计划要求不完全匹配
   - logistics_waybill 与 sales_delivery 双系统未集成
   - Incoterms 责任划分无结构化字段

### 建议优先级

建议按 P0 → P1 → P2 顺序修复，P0 缺陷应在下一个迭代立即修复（涉及合规和核心业务），P1 缺陷应在 2 个迭代内修复，P2 缺陷可在 3 个迭代内修复。修复时应同步更新测试覆盖，确保新功能有单元测试和集成测试。

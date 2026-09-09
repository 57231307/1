# 需求文档：RLS 表「本部门数据」（Dept）语义定义

Feature Name: dept-data-scope-semantics
Updated: 2026-09-08
Status: DRAFT（待确认 3 个决策点）

## 引言

系统数据范围模型分三级（all/dept/self）。RLS（行级安全）策略在 5 张表
（customers、suppliers、sales_orders、crm_lead、crm_opportunity）上实现了
self 级隔离，dept 级语义缺失导致两类问题：

1. RLS 策略与角色 dept 用户（约 10 个角色：sales_manager、finance_manager 等）
   的可见范围冲突——RLS 激活后 dept 用户被锁出团队数据。
2. 应用层 `build_data_scope_condition` 的 dept 分支把 owner/created_by 列误传为
   department 列，dept 用户实际执行 `created_by = department_id`（既有 bug，
   几乎恒空集）。

本需求定义「本部门数据」在 RLS 5 张表上的精确语义，作为 RLS 策略扩展与应用层
修复的统一依据。

## 术语表

- **数据行**：5 张 RLS 表（customers、suppliers、sales_orders、crm_lead、crm_opportunity）中的记录。
- **数据归属人**：数据行上的责任用户。customers/crm_lead/crm_opportunity 取 `owner_id`；suppliers/sales_orders 取 `created_by`。
- **数据部门**：数据归属人在 users 表上的 department_id。数据部门的归属随数据归属人转移而变化。
- **可见部门集合**：当前用户可视为「本部门」的部门 ID 集合 = 用户主部门 + 全部兼职部门（user_departments）+ 上述部门的全部子部门（departments.parent_id 递归）。
- **公海行**：customers 中 `owner_id = 0` 的行；crm_lead/crm_opportunity 中 `lead_status/opportunity_status = 'pool'` 的行。
- **无部门用户**：users.department_id 为 NULL 且 user_departments 无关联记录的用户。

## 现状事实（决策依据）

| # | 事实 | 出处 |
|---|------|------|
| F1 | 5 张 RLS 表均无 department_id 列 | migration/src/domain/system/m0001_initial_schema.rs |
| F2 | AuthContext.department_id 仅含 users 表单值，user_departments 多部门表已建但未接入 auth | src/middleware/auth.rs:313 |
| F3 | `get_user_dept_scope_ids`（含兼职+子树）已实现但零调用方 | src/services/data_permission_service.rs:306 |
| F4 | 初始部门树扁平（parent_id 全 NULL），子部门为部署期业务配置 | src/services/init_service_ops/dept_user.rs:15 |
| F5 | 应用层 dept 分支在 5 张表上列语义错误（dept_column 误传 owner 列） | src/services/so/order_query.rs:122 等 5 处 |
| F6 | crm_lead/crm_opportunity 回收公海只改状态不清理 owner_id，RLS 策略缺公海分支 | src/services/crm/recycle_executor.rs:127 |
| F7 | 角色分布：all 6 个 / dept 约 10 个 / self 约 17 个 | src/services/init_service_ops/role.rs |
| F8 | 数据归属人转移写点约 8-10 处（assign/lead/pool/import/update） | src/services/crm/assign.rs:225,371,568 等 |

## 需求

### R1 dept 用户的行级可见范围

**User Story**: 作为 dept 角色（部门经理），我要查看本部门成员的数据行，以便履行团队审批与业务管理职责。

#### 验收标准

1. WHEN dept 用户查询 5 张 RLS 表中任一表，系统 SHALL 返回「数据部门 ∈ 可见部门集合」的数据行与本用户自身的数据行。
2. WHEN dept 用户查询任一 RLS 表，系统 SHALL 同时返回公海行（公海行对所有已认证用户可见）。
3. WHEN 数据行的数据归属人发生转移，系统 SHALL 使该行的可见部门集合按新归属人重新计算。
4. IF 数据归属人（owner_id/created_by）为空或 0，系统 SHALL 视该行为公海行或历史数据并放行（suppliers/sales_orders 的 created_by NULL 语义与 customers owner_id=0 语义保持现状）。

### R2 dept 语义的口径定义

**User Story**: 作为系统管理员，我要「本部门数据」有唯一确定的口径，以便 RLS 策略与应用层过滤结果一致。

#### 验收标准

1. 系统 SHALL 以「数据归属人所在部门」作为数据部门（口径随归属人动态变化）。
2. 系统 SHALL 将可见部门集合定义为：主部门 + 全部兼职部门 + 上述部门的全部子部门。
3. WHEN 用户仅存在于 users.department_id（历史数据路径），系统 SHALL 将该单值纳入可见部门集合。
4. 系统 SHALL 保证 RLS 策略与应用层 DataScope 过滤对同一用户、同一表返回一致的行集合。

### R3 无部门用户的降级行为

**User Story**: 作为未分配部门的 dept 用户，我要获得确定的数据可见范围，以便系统行为可预期。

#### 验收标准

1. IF 用户为无部门用户且角色 data_scope 为 dept，系统 SHALL 按 self 语义（仅本人数据 + 公海行）放行。
2. 系统 SHALL 在日志中记录「dept 退化为 self」事件（每会话一次，避免日志风暴）。

### R4 RLS 上下文携带部门信息

**User Story**: 作为系统，我要在连接级会话上下文中携带用户的可见部门集合，以便 PG RLS 策略在行级判断部门归属。

#### 验收标准

1. WHEN 已认证请求进入 RLS 中间件且用户 data_scope 为 dept，系统 SHALL 解析可见部门集合并写入请求级上下文。
2. WHEN 连接池钩子在业务查询的同一连接上设置 GUC，系统 SHALL 同时设置 app.user_id 与部门集合上下文。
3. IF 部门集合解析失败，系统 SHALL 降级为仅设置 app.user_id（self 语义），并记录 warn 日志。

### R5 性能与安全约束

#### 验收标准

1. 系统 SHALL 为部门判断谓词涉及的列提供索引（数据部门列或 users 关联列）。
2. WHEN 可见部门集合解析完成，系统 SHALL 缓存结果（TTL ≤ 5 分钟或部门变更时主动失效）。
3. 系统 SHALL 保证 admin/all 角色的可见范围与现状一致（跳过 RLS）。
4. 系统 SHALL 保证 RLS 与应用层过滤叠加后结果集取交集语义成立（两者口径一致时交集=各自结果）。

## 待确认决策点（阻塞项）

- **D1 数据部门口径**：推荐「归属人部门动态计算」；备选「创建时快照部门」（转移后部门不变）。
- **D2 部门范围口径**：推荐「主+兼职+子部门」；备选「仅主部门单值」（最简，兼容现状）。
- **D3 实现载体**：推荐「表冗余数据部门列 + 回填迁移 + 写点维护」（策略纯本表谓词、可索引、性能最优）；备选「策略内 EXISTS 反查 users」（零列改动、每行点查）。

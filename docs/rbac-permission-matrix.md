# RBAC 权限矩阵文档（V15 P1 12.2）

> **文档版本**：V15
> **创建时间**：2026-07-27
> **维护说明**：本文档与 `backend/database/init_admin_permissions.sql` 保持同步，权限码变更时必须同步更新本文档

## 1. 概述

本文档定义面料行业 ERP 系统的完整权限矩阵，覆盖所有角色 × 所有资源 × 所有操作。

### 权限码格式

```
{resource_type}:{action}
```

示例：`color_card_issue:create`、`inventory:read`、`users:delete`

### 权限判定规则

1. **admin 角色**：拥有 `*:*` 通配权限，可访问所有资源
2. **继承关系**：`sales_manager` 继承 `sales` 的所有权限（见第 4 节）
3. **互斥关系**：互斥角色不可同时持有（见第 5 节）
4. **数据范围**：行级数据隔离通过 `data_scope` 字段控制（见第 6 节）

## 2. 角色列表

| 角色编码 | 角色名称 | 数据范围 | 系统角色 | 说明 |
|---------|---------|---------|---------|------|
| `admin` | 管理员 | ALL | 是 | 系统管理员，拥有全部权限 |
| `manager` | 部门经理 | dept | 是 | 部门级管理，管理本部门数据 |
| `operator` | 操作员 | self | 是 | 基础操作员，仅本人数据 |
| `sales_manager` | 销售经理 | dept | 否 | 管理销售团队，继承 sales 权限 |
| `sales` | 销售代表 | self | 否 | 管理自己负责的客户 |
| `customer_service` | 客户服务 | self | 否 | 客户服务与售后 |
| `warehouse_manager` | 仓库经理 | dept | 否 | 仓库管理，可查看成本 |
| `warehouse` | 仓库员 | self | 否 | 仓库日常操作 |
| `quality_inspector` | 质检员 | self | 否 | 质量检验 |
| `quality_manager` | 质量经理 | dept | 否 | 质量管理 |
| `finance` | 财务 | all | 否 | 财务管理，可查看成本 |
| `production_manager` | 生产经理 | dept | 否 | 生产计划与管理 |
| `customer` | 客户 | self | 否 | 客户门户，仅查看自己的数据 |

## 3. 权限矩阵

### 3.1 色卡发放管理（color_card_issue）

| 角色 | create | return | lost | damaged | cancel | read |
|------|--------|--------|------|---------|--------|------|
| admin | ✅ 允许 | ✅ 允许 | ✅ 允许 | ✅ 允许 | ✅ 允许 | ✅ 允许 |
| sales_manager | ✅ 允许 | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ✅ 允许 |
| sales | ✅ 允许 | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ✅ 允许 |
| customer_service | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ✅ 允许 |
| warehouse_manager | ✅ 允许 | ✅ 允许 | ✅ 允许 | ✅ 允许 | ✅ 允许 | ✅ 允许 |
| warehouse | ✅ 允许 | ✅ 允许 | ✅ 允许 | ✅ 允许 | ❌ 拒绝 | ✅ 允许 |
| quality_inspector | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ✅ 允许 |
| quality_manager | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ✅ 允许 |
| finance | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ✅ 允许 |
| production_manager | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ✅ 允许 |
| customer | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ❌ 拒绝 | ✅ 仅自己 |

**权限码说明**：
- `color_card_issue:create`：发放色卡
- `color_card_issue:return`：归还色卡
- `color_card_issue:lost`：登记遗失
- `color_card_issue:damaged`：标记损坏
- `color_card_issue:cancel`：取消发放（仅仓库经理/admin）
- `color_card_issue:read`：查看发放记录

### 3.2 采购管理（purchases）

| 角色 | read | create | update | delete |
|------|------|--------|--------|--------|
| admin | ✅ | ✅ | ✅ | ✅ |
| manager | ✅ | ✅ | ✅ | ❌ |
| sales | ❌ | ❌ | ❌ | ❌ |
| warehouse_manager | ✅ | ❌ | ❌ | ❌ |
| finance | ✅ | ❌ | ❌ | ❌ |

### 3.3 销售管理（sales）

| 角色 | read | create | update | delete |
|------|------|--------|--------|--------|
| admin | ✅ | ✅ | ✅ | ✅ |
| sales_manager | ✅ | ✅ | ✅ | ❌ |
| sales | ✅ | ✅ | ✅（仅自己） | ❌ |
| customer_service | ✅ | ❌ | ❌ | ❌ |
| finance | ✅ | ❌ | ❌ | ❌ |

### 3.4 库存管理（inventory）

| 角色 | read | create | update | delete |
|------|------|--------|--------|--------|
| admin | ✅ | ✅ | ✅ | ✅ |
| warehouse_manager | ✅ | ✅ | ✅ | ✅ |
| warehouse | ✅ | ✅ | ✅ | ❌ |
| production_manager | ✅ | ❌ | ❌ | ❌ |

### 3.5 财务管理（finance）

| 角色 | read | create | update | delete |
|------|------|--------|--------|--------|
| admin | ✅ | ✅ | ✅ | ✅ |
| finance | ✅ | ✅ | ✅ | ❌ |
| manager | ✅ | ❌ | ❌ | ❌ |

### 3.6 客户管理（customers）

| 角色 | read | create | update | delete |
|------|------|--------|--------|--------|
| admin | ✅ | ✅ | ✅ | ✅ |
| sales_manager | ✅ | ✅ | ✅ | ❌ |
| sales | ✅（仅自己负责） | ✅ | ✅（仅自己负责） | ❌ |
| customer_service | ✅ | ❌ | ✅（售后） | ❌ |

### 3.7 用户管理（users）

| 角色 | read | create | update | delete |
|------|------|--------|--------|--------|
| admin | ✅ | ✅ | ✅ | ✅ |
| manager | ✅（本部门） | ❌ | ❌ | ❌ |

### 3.8 审计日志（audit）

| 角色 | read |
|------|------|
| admin | ✅ |
| 其他 | ❌ |

### 3.9 仪表板（dashboard）

| 角色 | read |
|------|------|
| 所有角色 | ✅ |

## 4. 角色继承关系

通过 `role_relations` 表（`relation_type=inherit`）配置，父角色自动继承子角色的所有权限。

| 父角色 | 子角色 | 说明 |
|--------|--------|------|
| sales_manager | sales | 销售经理继承销售代表权限 |
| purchase_manager | purchase_clerk | 采购经理继承采购员权限 |
| inventory_manager | warehouse_keeper | 库存经理继承仓库管理员权限 |
| qc_manager | quality_inspector | 质量管理经理继承质检员权限 |
| finance_manager | accountant | 财务经理继承会计权限 |
| hr_manager | hr_specialist | 人事经理继承人事专员权限 |
| crm_manager | crm_rep | CRM经理继承CRM专员权限 |

**继承规则**：
- 继承是递归的（A 继承 B，B 继承 C，则 A 拥有 B+C 的权限）
- 继承不传递数据范围（sales_manager 的 data_scope=dept 不会变为 self）
- 继承关系不可形成环（系统通过 BFS 遍历 + visited 集合防环）

## 5. 角色互斥关系

通过 `role_relations` 表（`relation_type=mutual_exclusive`）配置，互斥角色不可同时分配给同一用户。

| 角色 A | 角色 B | 互斥原因 |
|--------|--------|---------|
| sales_rep | accountant | 防止销售操控账务 |
| sales_rep | cashier | 防止销售收款舞弊 |
| purchase_clerk | accountant | 防止采购舞弊 |
| warehouse_keeper | accountant | 防止库存账务舞弊 |

**互斥校验**：
- 在 `user_service.assign_role` 中调用 `RoleRelationService::check_mutual_exclusive`
- 在 `role_permission_service.assign_permission` 中校验角色互斥
- 互斥校验失败返回 `AppError::business("角色互斥冲突：...")`

> **注意**：与 `role_conflicts` 表（SoD 互斥）互补：
> - `role_conflicts` 仅用于财务三权分立等 SoD 互斥
> - `role_relations` 用于更通用的业务角色互斥

## 6. 数据范围（Data Scope）

通过 `roles.data_scope` 字段控制行级数据隔离：

| 数据范围 | 说明 | 适用角色 |
|---------|------|---------|
| `ALL` | 全部数据，无过滤 | admin |
| `dept` | 本部门数据 | manager / warehouse_manager / sales_manager |
| `self` | 仅本人数据 | sales / warehouse / operator |

**数据范围应用**（`DataPermissionService::apply_data_scope`）：
- **销售角色**：仅查询 `customer.owner_id = user_id` 的客户发放记录
- **客户门户**：仅查询 `customer_id = ?` 的自己的发放记录
- **其他角色**：仅查询 `issued_by = user_id` 的自己发放的记录
- **admin 角色**：不加过滤（全部数据）

**成本数据敏感过滤**（`DataPermissionService::can_view_cost_data`）：
- 仅 admin / finance / warehouse_manager 角色可查看成本字段
- 其他角色查询时应隐藏 `cost_amount` / `unit_cost` / `total_cost` 等字段

## 7. 权限委托（Delegation）

通过 `permission_delegations` 表支持时限化临时权限委托。

### 委托规则
1. 委托人与被委托人不可为同一人
2. `valid_until` 必须晚于 `valid_from`
3. 委托时长上限：90 天（防止长期委托变相授权）
4. `is_chain_allowed` 默认 false（禁止链式委托）
5. 委托必须记录审计日志
6. 过期委托由定时任务自动标记为 expired

### 委托状态机
```
pending → active → expired（自动过期）
                 → revoked（手动撤销）
```

### 权限聚合
权限中间件在权限校验时聚合：
```
用户最终权限 = 用户自身权限 + 委托获得的权限（在有效期内）
```

## 8. 审计日志要求

以下敏感操作必须记录审计日志：

| 操作类型 | 操作 | 审计字段 |
|---------|------|---------|
| 权限分配 | `assign_permission` | operator_id / role_id / resource_type / action / old_value / new_value |
| 权限删除 | `remove_permission` | operator_id / role_id / resource_type / action / old_value |
| 角色变更 | `update_user`（role_id 变更） | operator_id / target_user_id / old_role_id / new_role_id |
| 权限委托 | `create_delegation` | delegator_id / delegatee_id / permission_code / valid_until |
| 委托撤销 | `revoke_delegation` | delegation_id / operator_id / revoke_reason |
| 权限拒绝 | HTTP 403 响应 | user_id / required_permission / path / method |
| 色卡发放 | issue/return/lost/damaged/cancel | operator_id / issue_id / color_card_id / before/after_snapshot |

## 9. 会话安全

### 会话固定攻击防护（V15 P1 12.8）

用户角色变更后立即：
1. 吊销该用户所有历史 JWT（`revoke_user_jtis`）
2. 清除旧 CSRF Token（`clear_old_csrf_token_for_user`）
3. 失效角色权限缓存（`invalidate_permission_cache`）

强制用户重新登录，防止旧 session 使用过期权限。

## 10. 变更历史

| 日期 | 变更内容 | 变更人 |
|------|---------|--------|
| 2026-07-27 | 初始版本，V15 P1 12.2 创建权限矩阵文档 | P1 修复代理 |

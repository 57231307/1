# V15 修复阶段已修复任务实际状态复查报告

> **复查时间**：2026-07-17
> **复查范围**：V15 修复阶段已合并的 30 个 P0 任务（Batch 433-472 / PR #611~#648）
> **复查方法**：仅读取代码，不做任何修改（用户指令："只记录不做任何修改"）
> **复查员**：小玲（GLM-5.2）+ 3 个 search 子代理并行
> **特批指令**：本次直接合并（用户指令："复查结果推送ci,特批不等ci时间"）

---

## 一、复查汇总

### 1.1 总体统计

| 状态 | 项数 | 占比 |
|------|------|------|
| ✅ 一致（归档描述与实际代码完全匹配） | **19** | 76.0% |
| ⚠️ 部分一致（核心功能已实现，但归档描述与代码细节有偏差） | **6** | 24.0% |
| ❌ 不一致（功能完全不存在） | **0** | 0.0% |
| **合计** | **25** | 100% |

> 说明：25 项中 P0-F03/F04/F05/F08/F09 合并为 1 项复查（Batch 471 同 PR），实际覆盖 30 个 P0 任务。

### 1.2 关键发现等级

| 等级 | 数量 | 说明 |
|------|------|------|
| 🔴 高（功能性缺失，需修复） | **1** | P0-S14 migration 047 缺失 |
| 🟡 中（归档描述与代码方案不符，功能已达成） | **2** | P0-F03 borrow_record model 已删 / P0-S02 调用层级差异 |
| 🟢 低（数量/位置类差异，不影响功能） | **3** | P0-S04 角色数 37≠31 / P0-S20 函数位置 / P0-S27 方法数 5≠7 |

---

## 二、详细复查结果

### 2.1 P0-S 系列权限任务（11 项，Subagent 1 复查）

| P0 任务 | 归档状态 | 实际状态 | 一致性 |
|---------|---------|---------|--------|
| P0-S01 行级数据权限 | 已完成 | ✅ 全部存在 | ✅ 一致 |
| P0-S02 IDOR 防护 | 已完成 | ⚠️ 部分缺失（handler 层未直接调用 check_resource_owner，改由 service 层调用） | ⚠️ 部分一致 |
| P0-S03 超级权限注入修复 | 已完成 | ✅ 全部存在 | ✅ 一致 |
| P0-S04 14 类角色补齐 | 已完成 | ⚠️ 部分缺失（实际 37 类角色，归档描述为 31 类） | ⚠️ 部分一致 |
| P0-S05 SoD 互斥 | 已完成 | ✅ 全部存在 | ✅ 一致 |
| P0-S06 权限变更审计 | 已完成 | ✅ 全部存在 | ✅ 一致 |
| P0-S07 权限缓存失效 | 已完成 | ✅ 全部存在 | ✅ 一致 |
| P0-S09 AuthContext 补齐 | 已完成 | ✅ 全部存在 | ✅ 一致 |
| P0-S10 method_to_action | 已完成 | ✅ 全部存在 | ✅ 一致 |
| P0-S11 导出审计日志 | 已完成 | ✅ 全部存在 | ✅ 一致 |
| P0-S14 敏感数据导出二级审批 | 已完成 | ⚠️ 部分缺失（**migration 047 不存在**，m0047 实际为 webhooks 相关） | ⚠️ 部分一致 |

### 2.2 P0-S18/20-28 系列任务（10 项，Subagent 2 复查）

| P0 任务 | 归档状态 | 实际状态 | 一致性 |
|---------|---------|---------|--------|
| P0-S18 dye_recipe_master 角色 | 已完成 | ✅ 全部存在 | ✅ 一致 |
| P0-S20 权限资源缺口 | 已完成 | ⚠️ 部分缺失（extract_action_from_path 在 permission.rs 而非 init_service.rs） | ⚠️ 部分一致 |
| P0-S21 模块前缀白名单 | 已完成 | ✅ 全部存在（39 个前缀，超过 28 个） | ✅ 一致 |
| P0-S22 权限矩阵 | 已完成 | ✅ 全部存在（36 角色 × 60+ 资源） | ✅ 一致 |
| P0-S23 用户角色互斥校验 | 已完成 | ✅ 全部存在 | ✅ 一致 |
| P0-S24 前后端权限边界一致性 | 已完成 | ✅ 全部存在 | ✅ 一致 |
| P0-S25 RLS 策略启用 | 已完成 | ✅ 全部存在（5 张表） | ✅ 一致 |
| P0-S26 AI 端点权限码 | 已完成 | ✅ 全部存在（8 个 AI 资源） | ✅ 一致 |
| P0-S27 AI 推理数据范围过滤 | 已完成 | ⚠️ 部分缺失（查询方法 5 个而非 7 个，但调用点 8 ≥ 7） | ⚠️ 部分一致 |
| P0-S28 前端 v-permission 覆盖率 | 已完成 | ✅ 全部存在 | ✅ 一致 |

### 2.3 P0-F 系列面料任务（4 大项，Subagent 3 复查）

| P0 任务 | 归档状态 | 实际状态 | 一致性 |
|---------|---------|---------|--------|
| P0-F01 dye_batch dye_lot_no 字段 | 已完成 | ✅ 全部存在（migration 048 + model + handler + service） | ✅ 一致 |
| P0-F02 关键业务约束 UNIQUE | 已完成 | ✅ 全部存在（migration 049 含 3 个联合唯一索引） | ✅ 一致 |
| P0-F03/F04/F05/F08/F09 色卡发放模式重构-后端 | 已完成 | ⚠️ 部分缺失（color_card_borrow_record.rs 已删除，归档说应保留） | ⚠️ 部分一致 |
| P0-F07 色卡发放前端重写 | 已完成 | ✅ 全部存在（10 文件，2 新增 + 2 删除 + 6 修改） | ✅ 一致 |

---

## 三、关键差异详细说明

### 3.1 🔴 P0-S14 migration 047 缺失（高优先级，功能性缺失）

**归档描述**：新增 migration 047（export_approval_request 表 29 字段 + 6 索引）

**实际状态**：
- ✅ `backend/src/services/export_approval_service.rs` 存在
- ✅ `backend/src/models/export_approval_request.rs` 存在
- ✅ `backend/src/handlers/export_approval_handler.rs` 存在 7 端点
- ❌ **migration 047 完全不存在**：实际 `m0047` 为 `add_last_payload_to_webhooks`（与 export_approval 无关）
- ❌ migration 列表（m0001~m0054）中无任何 export_approval 相关 migration
- ❌ 全代码库无 `export_approval_requests` 表的 CREATE TABLE 语句

**影响**：数据库表无法通过 migration 自动创建。若线上环境已手动建表，功能不受影响；若新环境部署，则会因表不存在导致运行时错误。

**建议**：补充 migration 创建 `export_approval_requests` 表（29 字段 + 6 索引）。

### 3.2 🟡 P0-F03 borrow_record model 已删除（中优先级，归档描述偏差）

**归档描述**：保留旧表 color_card_borrow_records 不重命名为 _legacy，保留 Rust model 保护 migration m0029 链路

**实际状态**：
- ✅ migration 050 创建 color_card_issues 表正常存在
- ✅ color_card_issue.rs model + service + handler 全部存在
- ✅ routes/color_card.rs 已删除 /borrow/* 路由
- ✅ color_card_response_dto.rs 含普通 `//` 占位注释
- ❌ **`backend/src/models/color_card_borrow_record.rs` 文件不存在**（已彻底删除）
- ✅ `backend/src/models/mod.rs:295` 注释明确"V15 P0-F03：删除 color_card_borrow_dto（borrow 模式已废弃）"

**影响**：实际无影响。m0029 通过 `include_str!` 加载 SQL 文件 `backend/migrations/20260628000001_drop_tenant_columns/up.sql` 第 168 行执行 `ALTER TABLE "color_card_borrow_records" DROP COLUMN IF EXISTS "tenant_id";`，纯 SQL 执行不依赖 Rust model 文件。m0044 同理通过 include_str! 加载 `20260617000008_create_color_card_borrow_records/up.sql` 创建旧表。

**结论**：实际项目选择了"彻底删除 Rust 端 borrow model"方案，比归档描述的"保留 model 保护链路"方案更彻底。两种方案都能让 SQL 迁移正常执行，但归档描述与代码不完全匹配。

### 3.3 🟡 P0-S02 调用层级差异（中优先级，归档描述偏差）

**归档描述**：在 get/update/delete handler 的 update/delete 调用前显式调用 service.get_xxx_by_id(id, Some(&data_scope_ctx)) 复用 check_resource_owner 做归属校验

**实际状态**：
- ✅ sales_order_handler.rs / purchase_order_handler.rs / crm_handler.rs / inventory_adjustment_handler.rs 等 IDOR 防护已生效
- ⚠️ 但 handler 层未直接调用 `check_resource_owner`（grep `backend/src/handlers` 无匹配）
- ⚠️ 实际调用层级：handler 通过 `get_order_detail(id, Some(&data_scope_ctx))` 间接防护；service 内部调用 `check_resource_owner`
- ✅ sales_order_handler.rs L222/L245 注释明确"V15 P0-S02：IDOR 防护"

**影响**：功能已实际生效，只是调用层级与检查清单字面要求不同。

### 3.4 🟢 P0-S04 角色数量偏差（低优先级，数量更多但描述不符）

**归档描述**：补齐 31 类业务角色

**实际状态**：`init_service.rs` 的 `create_default_role_permissions` 实际配置 37 类角色（3 基础 + 34 业务）

**影响**：无功能影响，数量更多覆盖更全，但归档描述的"31 类"与代码实际"37 类"不符。

### 3.5 🟢 P0-S20 函数位置差异（低优先级，架构更合理）

**归档描述**：`init_service.rs` 含 `extract_action_from_path` 函数

**实际状态**：
- ✅ `PERMISSION_RESOURCES`（100+ 资源）+ `PERMISSION_ACTIONS`（11 个）在 init_service.rs
- ⚠️ `extract_action_from_path` 函数在 `backend/src/middleware/permission.rs:148`

**影响**：无功能影响。从架构角度看，该函数属于权限中间件的路径解析工具，放在 `middleware/permission.rs` 更合理。

### 3.6 🟢 P0-S27 查询方法数差异（低优先级，调用点数足够）

**归档描述**：7 个查询方法注入 apply_data_scope 过滤

**实际状态**：
- ✅ `ai_extend_service.rs` 共 5 个查询方法调用 `apply_data_scope`（list_process_optimizations / list_process_optimizations_by_color / list_quality_predictions / list_quality_predictions_by_product / ai_summary）
- ✅ 调用点数 8 ≥ 7（ai_summary 含 4 个调用点）
- ✅ 另外 2 个单条记录查询方法（get_process_optimization / get_quality_prediction）使用 `check_resource_owner` 进行归属校验（架构上更合适）
- ✅ 4 个写操作调用 `check_resource_owner` 完全符合
- ✅ 15 个端点使用 `auth: AuthContext`（无 `_auth` 残留），超过期望的 11 个

**影响**：无功能影响。查询方法数 5 而非 7，但调用点数 8 ≥ 7 且其他子项完全满足。

---

## 四、关键文件路径清单

### 4.1 P0-S 系列权限相关（Subagent 1 + 2 复查）

后端：
- `/workspace/backend/src/utils/data_scope.rs`（P0-S01）
- `/workspace/backend/src/handlers/auth_handler.rs`（P0-S03）
- `/workspace/backend/src/services/init_service.rs`（P0-S04/S18/S20/S22/S23/S26）
- `/workspace/backend/src/services/user_service.rs`（P0-S05/S06/S07/S23）
- `/workspace/backend/src/models/role_conflict.rs`（P0-S05）
- `/workspace/backend/src/models/permission_change_audit.rs`（P0-S06）
- `/workspace/backend/src/middleware/permission.rs`（P0-S07/S10/S20）
- `/workspace/backend/src/handlers/dye_recipe_handler.rs` + `dye_batch_handler.rs` + `print_handler.rs`（P0-S09）
- `/workspace/backend/src/handlers/sales_order_handler.rs` + `purchase_order_handler.rs` + `crm_handler.rs`（P0-S11/S02）
- `/workspace/backend/src/services/export_approval_service.rs`（P0-S14，⚠️ migration 缺失）
- `/workspace/backend/src/models/export_approval_request.rs`（P0-S14）
- `/workspace/backend/src/utils/path_utils.rs`（P0-S21）
- `/workspace/backend/src/services/ai_extend_service.rs`（P0-S27）
- `/workspace/backend/src/handlers/ai_extend_handler.rs`（P0-S27）
- `/workspace/backend/src/routes/analytics.rs`（P0-S26）
- `/workspace/backend/database/rls.sql`（P0-S25）

前端：
- `/workspace/frontend/src/constants/permissions.ts`（P0-S24/S28）
- `/workspace/frontend/src/views/system/tabs/UserTab.vue`（P0-S24）
- `/workspace/frontend/src/views/warehouse/index.vue`（P0-S24）
- `/workspace/frontend/src/views/customer/index.vue` + `supplier/index.vue`（P0-S28）

### 4.2 P0-F 系列面料相关（Subagent 3 复查）

后端：
- `/workspace/database/migration/048_add_dye_lot_no_to_dye_batch.sql`（P0-F01）
- `/workspace/database/migration/049_add_fabric_unique_constraints.sql`（P0-F02）
- `/workspace/database/migration/050_color_card_issue_refactor.sql`（P0-F03）
- `/workspace/backend/src/models/dye_batch.rs`（P0-F01）
- `/workspace/backend/src/handlers/dye_batch_handler.rs`（P0-F01）
- `/workspace/backend/src/services/dye_batch_cost_bridge_service.rs`（P0-F01）
- `/workspace/backend/src/models/color_card_issue.rs`（P0-F04）
- `/workspace/backend/src/services/color_card_issue_service.rs`（P0-F05/F06/F09）
- `/workspace/backend/src/handlers/color_card/issue.rs`（P0-F08）
- `/workspace/backend/src/routes/color_card.rs`（P0-F08）
- `/workspace/backend/src/models/color_card_response_dto.rs`（P0-F03 占位注释）
- `/workspace/backend/src/models/mod.rs:295`（P0-F03 删除说明注释）

前端：
- `/workspace/frontend/src/api/color-card.ts`（P0-F07）
- `/workspace/frontend/src/views/color-cards/issues.vue`（P0-F07）
- `/workspace/frontend/src/views/color-cards/list.vue` + `detail.vue`（P0-F07）
- `/workspace/frontend/src/components/IssueRecordTimeline.vue`（P0-F07）
- `/workspace/frontend/src/router/index.ts`（P0-F07）
- `/workspace/frontend/src/components/Layout/MainLayout.vue`（P0-F07）
- `/workspace/frontend/src/locales/zh-CN.ts` + `en-US.ts`（P0-F07）
- `/workspace/frontend/e2e/color-card.spec.ts`（P0-F07）

---

## 五、结论与建议

### 5.1 总体结论

V15 修复阶段 30 个 P0 任务**全部已在代码中落地实现**，无完全不一致项。19 项完全一致，6 项部分一致（核心功能已达成，仅归档描述与代码细节有偏差），0 项不一致。

### 5.2 修复优先级建议

| 优先级 | 任务 | 建议 |
|--------|------|------|
| 🔴 高 | P0-S14 migration 047 缺失 | 补充 migration 创建 `export_approval_requests` 表（29 字段 + 6 索引），确保新环境部署可用 |
| 🟡 中 | 归档描述与代码偏差 | 后续归档时校正描述（P0-F03 已删除 borrow_record model / P0-S02 调用层级 / P0-S04 角色数 37 / P0-S20 函数位置 / P0-S27 方法数 5） |
| 🟢 低 | 无需修复 | 数量/位置类差异不影响功能 |

### 5.3 复查限制

- 本次复查仅检查代码存在性，未运行功能测试
- 未验证数据库实际表结构（仅检查 migration 文件存在性）
- 未验证运行时行为（如权限校验是否真实生效）
- 未覆盖 P1/P2/P3 任务（用户指令仅复查已修复 P0）

---

## 六、复查元数据

- **复查执行时间**：2026-07-17
- **复查员**：小玲（GLM-5.2）
- **并行子代理**：3 个 search subagent
- **复查方法**：只读取代码，不修改
- **特批指令**：本次直接合并，不等 CI 时间
- **报告归档**：`.monkeycode/docs/audits/v15-fix-reaudit-2026-07-17.md`
- **关联文档**：
  - 审计计划：[v15-review-plan-2026-07-15.md](file:///workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md)
  - 审计汇总：[v15-summary-2026-07-16.md](file:///workspace/.monkeycode/docs/audits/v15/v15-summary-2026-07-16.md)
  - 已完成任务归档：[doto-su.md](file:///workspace/.monkeycode/doto-su.md)
  - 一句话总结：[CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)

# A.2.2 TODO/FIXME 标记扫描报告

- 扫描范围：`backend/src/` 全目录
- 扫描命令：`rg -n "TODO|FIXME" backend/src/`
- 扫描时间：2026-08-22
- 执行者：代码审计代理（只读扫描，未修改任何代码文件）

## 一、统计

| 指标 | 数量 |
|------|------|
| grep 匹配总数 | 12 |
| 真实 TODO 标记 | 10 |
| FIXME 标记 | 0 |
| 历史说明性注释（含"TODO"字样但非待办） | 2 |

说明：`purchase_return_service.rs:180` 与 `:483` 两处为"批次 103 P0-4 修复：删除过时 TODO 注释"的历史说明，本身并非待办标记，已完成的清理记录。在下表标注为"历史说明"。

## 二、标记明细表

| 文件 | 行号 | 类型 | 内容摘要 | 是否有对应 issue/计划 |
|------|------|------|----------|----------------------|
| handlers/dashboard_handler.rs | 262 | TODO | `cache_hit_rate = 0.0`，缓存命中率占位，需从 metrics 服务获取真实值 | 否（无 issue 编号，无具体计划） |
| handlers/dye_recipe_handler.rs | 185 | TODO | 引入"待审核"中间态，submit 改为 DRAFT → PENDING_APPROVAL 状态转换 | 是（标注"批次 423B"，属批次计划） |
| handlers/slow_query_handler.rs | 373 | TODO | 从 AppState.settings 读取慢查询阈值配置，需将 settings 存入 AppState | 否（无 issue 编号，有简短计划） |
| services/stock_alert.rs | 57 | TODO(tech-debt) | EXPIRING_THRESHOLD_DAYS 后续改为从配置读取，支持按产品类别差异化 | 是（tech-debt 标签，有计划） |
| services/stock_alert.rs | 61 | TODO(tech-debt) | SLOW_MOVING_THRESHOLD_DAYS 后续改为从配置读取，支持按产品类别差异化 | 是（tech-debt 标签，有计划） |
| services/supplier_service.rs | 1148 | TODO | `item_count: 0` 占位，需查询明细数量 | 否（无 issue 编号，无具体计划） |
| services/data_permission_service.rs | 287 | TODO | user 表新增 customer_id 字段后补充查询逻辑 | 否（无 issue 编号，有依赖条件） |
| utils/di_container.rs | 9 | TODO | 迁移 parking_lot::Mutex 消除 Mutex 中毒问题 | 否（无 issue 编号，有具体计划） |
| services/purchase_return_service.rs | 180 | 历史说明 | "批次 103 P0-4 修复：删除过时 TODO 注释（submit_return 已在批次 59b 透传 user_id）" | 不适用（已完成的清理记录） |
| services/purchase_return_service.rs | 483 | 历史说明 | "批次 103 P0-4 修复：删除过时 TODO 注释（reject_return 已在批次 59b 透传 user_id）" | 不适用（已完成的清理记录） |
| utils/cache.rs | 255 | TODO(tech-debt) | CSRF Token 默认 TTL 已从 7200s 缩短为 1800s；tech-debt 待进一步评估 | 是（关联"Wave 3 安全漏洞 #7"） |
| middleware/auth.rs | 48 | TODO(后续改造) | 接入 Redis L1(进程内)+L2(分布式)，禁用/删除时主动失效 user_active 缓存 | 是（标注"后续改造"，有详细计划） |

## 三、建议

### 3.1 建议转为 Issue（8 项）

以下标记缺乏 issue 跟踪，建议在 Issue 管理系统中创建对应任务，避免遗忘：

1. **dashboard_handler.rs:262** — 缓存命中率占位返回 0.0，影响监控数据准确性，应转 issue 并标注优先级。
2. **dye_recipe_handler.rs:185** — 染色配方审批流缺少"待审核"中间态，属业务流程缺陷，应转 issue 跟踪"批次 423B"。
3. **slow_query_handler.rs:373** — 慢查询阈值硬编码，应转 issue 跟踪 settings 注入 AppState 的改造。
4. **stock_alert.rs:57 与 :61** — 两处阈值常量配置化改造，建议合并为单个 issue（同属 tech-debt）。
5. **supplier_service.rs:1148** — `item_count` 恒为 0，影响采购订单明细展示，应转 issue。
6. **data_permission_service.rs:287** — 客户门户角色权限待实现，应转 issue 并标注为"依赖 schema 变更"。
7. **di_container.rs:9** — Mutex 中毒风险消除，应转 issue 跟踪 parking_lot 迁移。
8. **auth.rs:48** — 用户禁用缓存跨实例一致性改造，应转 issue 跟踪 Redis L1+L2 改造。

### 3.2 已有跟踪、无需新建 Issue（2 项）

1. **cache.rs:255** — 已关联"Wave 3 安全漏洞 #7"，且 TTL 已缩短至 1800s，建议在对应 issue 中追加"tech-debt 进一步评估"备注即可。
2. **stock_alert.rs:57/:61 的 tech-debt 标签** — 若已有 tech-debt 看板，可并入既有条目；若无，则归入 3.1 建议合并的 issue。

### 3.3 建议删除/改写（2 项）

1. **purchase_return_service.rs:180 与 :483** — 这两处是"删除过时 TODO 注释"的**历史说明**，本身并非待办标记。批次 103 P0-4 修复已完成，说明性注释价值有限，建议删除或改写为不含"TODO"字样的普通注释，避免后续扫描误报。

### 3.4 总体建议

- 项目当前 TODO 标记均为技术债务或功能占位，无 FIXME（紧急修复）标记，技术债风险可控。
- 建议建立统一的 TODO 标记规范，要求新增 TODO 时附带 issue 编号（如 `// TODO(#123):`），便于自动化追踪。
- 建议在 CI 中加入 `rg "TODO|FIXME"` 计数检查，标记数量增长时触发告警。

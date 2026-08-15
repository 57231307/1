# 技术债务跟踪清单

> 基于 2026-08-15 代码扫描，从 `backend/src/` 提取的 TODO/FIXME 标记。

---

## 待处理（10 处）

| 编号 | 文件:行号 | 类型 | 内容 | 优先级 | 关联 |
|------|-----------|------|------|--------|------|
| 1 | `utils/di_container.rs:9` | TODO | 迁移 parking_lot::Mutex 消除中毒问题 | P2 | 2.6 |
| 2 | `utils/cache.rs:255` | TODO(tech-debt) | CSRF Token 默认 TTL 从 7200s 缩短为 1800s | P2 | 2.8 |
| 3 | `handlers/dashboard_handler.rs:258` | TODO | cache_hit_rate 从 metrics 服务获取（当前硬编码 0.0） | P1 | 3.4 |
| 4 | `handlers/dye_recipe_handler.rs:182` | TODO(批次 423B) | 引入"待审核"中间态，submit 改为 DRAFT → PENDING_APPROVAL | P1 | 5.3 |
| 5 | `handlers/slow_query_handler.rs:367` | TODO | 从 AppState.settings 读取配置 | P2 | 3.13 |
| 6 | `middleware/auth.rs:48` | TODO(后续改造) | 接入 Redis L1(进程内)+L2(分布式) 并在禁用/删除时主动失效 | P1 | 4.4 |
| 7 | `services/stock_alert.rs:57` | TODO(tech-debt) | 即将过期阈值天数改为从配置读取，支持按产品类别差异化 | P2 | 2.8 |
| 8 | `services/stock_alert.rs:61` | TODO(tech-debt) | 滞销阈值天数改为从配置读取，支持按产品类别差异化 | P2 | 2.8 |
| 9 | `services/data_permission_service.rs:287` | TODO | user 表新增 customer_id 字段后补充查询逻辑 | P1 | 4.6 |
| 10 | `services/supplier_service.rs:1148` | TODO | 查询明细数量（当前硬编码 0） | P1 | 3.4 |

---

## 已完成（可移除，2 处）

| 编号 | 文件:行号 | 原因 |
|------|-----------|------|
| 11 | `handlers/ar_reconciliation_handler.rs:194` | 已用 `#[allow(dead_code)]` 标注，TODO 注释多余 |
| 12 | `services/warehouse_service.rs:108` | 注释本身说明"实现原 TODO 占位"，已完成 |

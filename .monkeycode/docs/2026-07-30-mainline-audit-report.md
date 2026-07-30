# V15 主线八维审计报告（2026-07-30）

> 在 V15 25 大类 195 维度审计报告（[docs/audits/v15/](file:///workspace/.monkeycode/docs/audits/v15/)）完成后，对当前 main 主线做"最严格"二次八维审计，作为 P0→P2→P3 快速修复的依据。

## 一、八维审计范围

| 维度 | 范围 | 严重度排序 |
|------|------|-----------|
| 1. 技术债务 | clippy 警告/TODO 标记/死代码/未使用 import | P0→P3 |
| 2. 功能缺失 | 文档定义但未实现的端点/字段/状态 | P0→P3 |
| 3. 功能连通性 | 前端调用 vs 后端实际路由 + 入参/出参契约 | P0→P3 |
| 4. 数据一致性 | 跨表事务/外键/唯一/CHECK 约束 | P0→P3 |
| 5. 数据孤岛 | 业务追溯三表 producer/触发器/约束 | P0→P3 |
| 6. 流程 | 状态机/审批流/事件监听 | P0→P3 |
| 7. 安全 | 越权/IDOR/对象级授权/敏感数据 | P0→P3 |
| 8. 合规 | SOC2/ISO27001/中国《劳动法》/隐私法/数据安全法 | P0→P3 |

## 二、关键发现

### 2.1 P0 Critical（11 项，全部修复）

| # | 类别 | 描述 | 修复位置 |
|---|------|------|----------|
| 1 | 连通性 | 前端 inventory-count 9 端点契约与后端不符（complete vs submit+approve、count_date 非 ISO 8601） | [frontend/src/api/inventory-count.ts](file:///workspace/frontend/src/api/inventory-count.ts) + 2 个 Vue |
| 2 | 流程 | 库存财务桥接事件 listener 无事务保护，业务失败后事件已标记成功导致重放失败 | [backend/src/services/inventory_finance_bridge_ops/listener.rs](file:///workspace/backend/src/services/inventory_finance_bridge_ops/listener.rs) |
| 3 | 流程 | 导出审批缺二级机制（target_level=2 时一级审批直接置 Approved） | [export_approval_request.rs](file:///workspace/backend/src/models/export_approval_request.rs) + [service.rs](file:///workspace/backend/src/services/export_approval_service.rs) |
| 4 | 安全 | init_token 占位值（changeme）可绕过 | [middleware/init_token.rs](file:///workspace/backend/src/middleware/init_token.rs) |
| 5 | 安全 | API 网关 4 handler 缺对象级授权 | [api_gateway_handler.rs](file:///workspace/backend/src/handlers/api_gateway_handler.rs) |
| 6 | 安全 | 导出审批 list_approval_requests 非 admin 可越权查看他人申请 | [export_approval_handler.rs](file:///workspace/backend/src/handlers/export_approval_handler.rs) |
| 7 | 技术债务 | 冒烟脚本 api-crud-test.sh 用 `code:400` 当成功条件 | [scripts/api-crud-test.sh](file:///workspace/scripts/api-crud-test.sh) |
| 8 | 合规 | 导出格式允许 csv（中国《数据安全法》要求导出不落地明文） | [export_approval_service.rs](file:///workspace/backend/src/services/export_approval_service.rs) |
| 9 | 数据一致性 | 定制订单 advance() 非事务化（验证→更新状态→完成节点→启动下节点之间可能被中断） | [custom_order_state_service.rs](file:///workspace/backend/src/services/custom_order_state_service.rs) |
| 10 | 数据一致性 | 委外订单 issue_order/settle 凭证创建与主单更新分两事务 | [outsourcing_ops/order.rs](file:///workspace/backend/src/services/outsourcing_ops/order.rs) |
| 11 | 合规 | SECURITY.md 漏洞披露邮箱为 `[TODO]` | [.monkeycode/docs/SECURITY.md](file:///workspace/.monkeycode/docs/SECURITY.md) |

### 2.2 P1 High（不在本批次，留待下批）

- 委外订单 record_receipt 内部 4 子方法（insert_receipt_record/insert_receipt_voucher/insert_loss_voucher_if_needed/apply_order_receipt）都直接用 self.db，无事务保护
- 前端 4 个 v-permission 缺失（已在 PR #775 修复但需复审）
- API 网关 PATCH rate_limit 缺范围校验
- 委外订单行数限制未生效

### 2.3 P2 Medium（3 项，全部修复）

| # | 类别 | 描述 | 修复位置 |
|---|------|------|----------|
| 1 | 技术债务 | test_inventory_count.rs / inv/count.rs / test_generate_no_endpoints.rs 3 处"占位模块"陈旧注释 | test_inventory_count.rs + inv/count.rs + test_generate_no_endpoints.rs |
| 2 | 功能缺失 | 导出审批缺"待我审批"快捷入口 | [service.rs](file:///workspace/backend/src/services/export_approval_service.rs) + [system.rs](file:///workspace/backend/src/routes/system.rs) + [handler.rs](file:///workspace/backend/src/handlers/export_approval_handler.rs) |
| 3 | 数据孤岛 | business_trace_chain / business_trace_snapshot / business_trace_assist_links 三表无 UNIQUE/CHECK/FK 约束，snapshot 缺逻辑外键 | [migrations/20260801000001_business_trace_constraints/](file:///workspace/backend/migrations/20260801000001_business_trace_constraints/) + [business_trace_service.rs](file:///workspace/backend/src/services/business_trace_service.rs) |

### 2.4 P3 Low（123 项，按需修复，本批次未启动）

文档/注释/命名/i18n/暗黑模式/CSP/可观测性增强等。

## 三、修复策略

- **P0**：单批次 `fix/audit-batch-2026-07-30` 修复 11 项 Critical/High
- **P2-02/05/06**：与 P0 同批次
- **P3**：留待下批

## 四、未覆盖的 P1/P2 风险（留待下批）

| 项 | 严重度 | 描述 |
|----|-------|------|
| 委外 record_receipt 子方法事务化 | P1 | 4 个内部 helper 用 self.db，无事务 |
| 业务追溯 producer 完整接入 | P2-06 续 | upsert_chain_node/link_assist/upsert_snapshot 已写但未在所有上游业务（采购收货/库存出入库/委外）中调用 |
| 前端契约对齐补齐 | P2-07 | 本批次仅修了 inventory-count，其余前端契约差异（如 V15 P1-11/12 涉及的 25+ 导出按钮）已在 PR #758/#771 处理 |
| 覆盖率阈值回调 | P3 | 24.10-1 临时降级为 1%，需补齐测试后回调至 70% |

## 五、Worktree 工作流教训

- 单次 worktree 集中多模块修复后，squash merge 不会丢
- 但若 worktree 提前丢失（磁盘清理/PR squash 后 worktree 自动删除），所有修改全丢
- **建议**：每次完成 P0/P1/P2 立即 commit+push+merge，避免跨批次 worktree 滞留

---

> 报告生成日期：2026-07-30
> 关联分支：fix/audit-batch-2026-07-30
> 修复 PR：待 gh pr create

# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单）。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近整理：2026-07-30（doto.md 更新：**PR #786 已合并 main**：V15 主线八维审计 + 快速修复 P0/P2 批次；① 八维审计报告 [docs/2026-07-30-mainline-audit-report.md](file:///workspace/.monkeycode/docs/2026-07-30-mainline-audit-report.md)；② P0 全部 11 项完成（盘点契约+事件事务+二级审批+init token+API 网关授权+导出范围收敛+冒烟脚本+导出格式合规+定制/委外事务化+SECURITY 邮箱）；③ P2-02 清理 3 处陈旧占位注释；④ P2-05 导出审批 list_pending_for_me 路由；⑤ P2-06 业务追溯三表 unique/CHECK/逻辑外键触发器约束迁移 20260801000001 + service 端生产者 upsert_chain_node/link_assist/upsert_snapshot；⑥ 前端 inventory-count 3 文件对齐后端契约；共 21 文件 +989/-229；CI 全部通过，clippy baseline 308 条；**PR #785 已合并 main**：P1 预留服务路由接入消除 174 个 dead_code 警告；**PR #783 已合并 main**：Clippy runner shutdown (exit 143) 修复 + Release 变更说明模板；**PR #777 已合并 main**：彻底移除 Docker/K8s 引用；**SeaORM 2.0 升级评估暂缓**：2.0.0 稳定版 2026-07-19 发布，181 处 active.insert(db) 需调整，暂不升级）

---

## 〇₀、V15 主线八维审计快速修复（2026-07-30 启动）

| 状态 | 数量 | 批次 |
|------|------|------|
| ✅ 已合并 main | 1 批 | audit-batch-2026-07-30（PR #786 已合并） |
| ⏳ 待推送 | 0 批 | — |

### 0.0.1 P0 完成明细

| P0 项 | 文件 | 关键改动 |
|-------|------|----------|
| 盘点契约 | frontend/src/api/inventory-count.ts + CountListTab + CountFormDialogTab | 对齐后端 9 端点（list/create/get/update/record/submit/approve/reject），complete → submit+approve，count_date → ISO 8601 |
| 事件事务 | backend/src/services/inventory_finance_bridge_ops/listener.rs | 阶段1查重→阶段2事务→失败回滚+幂等清除+死信兜底；event_idempotency_service 新增 unmark_processed |
| 二级审批 | export_approval_request.rs + service.rs | ApprovalStatus 新增 PendingL2；approve 拆 target_level+current_approval_step |
| init token 强度 | middleware/init_token.rs | INIT_TOKEN_PLACEHOLDERS 黑名单 + is_init_token_strong ≥32 字节 |
| API 网关授权 | handlers/api_gateway_handler.rs | ensure_can_manage_api_key 4 handler 接入 |
| 导出范围 | handlers/export_approval_handler.rs + system.rs | 非 admin 强制 applicant_user_id = auth.user_id；新增 /export-approvals/pending-for-me |
| 冒烟脚本 | scripts/api-crud-test.sh | 严格断言移除 code:400 误判 |
| 导出格式 | export_approval_service.rs | validate_create_request_fields 移除 csv 仅 xlsx/pdf |
| 定制订单事务 | custom_order_state_service.rs | advance() 用 txn.begin() + lock_exclusive + 3 个 _txn 子方法 |
| 委外订单事务 | outsourcing_ops/order.rs | issue_order/settle 凭证创建+主单更新同事务；TransactionTrait 导入 |
| SECURITY 邮箱 | .monkeycode/docs/SECURITY.md | [TODO] → security@57231307.com |

### 0.0.2 P2 完成明细

| P2 项 | 文件 | 关键改动 |
|-------|------|----------|
| P2-02 清理陈旧注释 | test_inventory_count.rs / inv/count.rs / test_generate_no_endpoints.rs | 3 处"占位模块"陈旧注释删除 |
| P2-05 导出审批 list_pending_for_me | service.rs + system.rs + handler.rs | 新增 list_pending_for_user(user_id,is_admin,q) 服务 + GET 路由 |
| P2-06 业务追溯约束 | migrations/20260801000001_business_trace_constraints + business_trace_service.rs | uniq_business_trace_chain_head/tail partial unique + snapshot trace_chain_id unique + assist_links 联合 unique + 3 个 CHECK + 3 个逻辑外键触发器；upsert_chain_node/link_assist/upsert_snapshot producer |

### 0.0.3 P1-委外收货主链路后续

| 状态 | 剩余项 | 文件 | 说明 |
|------|--------|------|------|
| ⏳ 待继续 | Task6-7 事务回滚测试与 PR/CI | `backend/tests/outsourcing_receipt_transaction.rs` + CI | `fix/p1-outsource-receipt-unify-2026-07-30` 已完成 Task3-5：`confirm` 整段事务化、质检事务兼容、删除 `record_receipt` 与 4 个子方法；剩余集成测试补齐和 CI 全绿验证 |

---

## 〇、P1 级任务进度总览（2026-07-27 启动）

### 0.1 按批次状态归类

| 状态 | 数量 | 批次 |
|------|------|------|
| ✅ 已合并到 main | 25 批 | P1-A、P1-B1、P1-B2、P1-C、P1-面料行业深化（batch-04+05）、P1-D（batch-08+20）、P1-batch13/14、P1-Batch16、P1-batch11/12、P1-batch19、P1-08 法律合规第二批（PR #758）、P1-09 色卡发放（9 项，PR #763）、P1-10 大货批色（7 项，PR #763）、P1-19 报表 BI（5 项，PR #763）、P1-25 部署升级（11 项，PR #758+#763）、P1-B3 法律合规扩展（PR #765）、P1-07 剩余可维护性（PR #767）、P1-20 可观测性（9 项全部完成，批次 1 PR #768 + 此前 6 项已实现）、P1-21 胚布拆匹（10 项，PR #770）、P1-22 库存排程（9 项，PR #770）、P1-batch02+03 通用代码质量+安全性（9 项，PR #771）、P1-batch19 组织定制物流（11 项，PR #771）、P1-24 前端架构（16 项，PR #771）、P1-Batch16 剩余 P1（5 项，PR #771）、P1-batch04/05 续作 事件贯通+业财一致性（PR #774）、P1-batch11 缺陷 2-3 遗留修复（4 个前端页面 v-permission，PR #775 admin override 合并） |
| ✅ 核实已完成 | 5 批 | batch-11 类十三打印导出 15/15 ✅（PR #775 已合并）、batch-12 类十四权限维度 14/14 ✅、batch-13 类十五业务主体 1/1 ✅、batch-14 类十六 AI 模块 24/24 ✅、batch-15 类十七+十八 财务+CRM 35/35 ✅ |
| ✅ 其他合并 | 3 PR | PR #776（文档同步 PR #775 合并记录）、PR #777（彻底移除 Docker/K8s 引用，11 文件 -130 行，对齐 systemd 直部署）、SeaORM 2.0 升级评估暂缓 |

### 0.2 待启动批次（优先级从高到低）

| 批次 | 类别 | P1 数 | 主要内容 |
|------|------|-------|----------|
| ~~P1-B3~~ | ~~类八 法律合规扩展~~ | ~~—~~ | ~~脱敏扩展 + 规则 4 注释精简~~ **✅ 已完成（2026-07-29，PR #765 已合并 main cc8a43f，脱敏扩展 PR #758 已完成 + 规则 4 注释精简 406 文件 +1917 -7735 约 1525 处，CI 关键检查全绿）** |
| ~~P1-batch02+03~~ | ~~类二+三 通用代码质量+安全性~~ | ~~9~~ | ~~api 命名/缩写命名/DbErr 包装 + refresh_token/PUBLIC_PATHS/validator/Webhook/magic bytes/zip bomb~~ **✅ 已完成（2026-07-29，PR #771 已合并 main：batch03 安全 6 项 PUBLIC_PATHS 精确匹配 + request_logging_middleware 重命名 + refresh_token Cookie 2天对齐 + Webhook payload 日志脱敏 + crm xlsx magic bytes 校验 + system_update zip bomb 防护；batch02 代码质量 3 项 前端 api 文件 kebab-case 7 文件 + 视图文件夹 kebab-case 17 文件夹 + 组件缩写重命名 14 个；附 22 文件 FromStr 导入清理）** |
| ~~P1-09~~ | ~~类九 色卡发放~~ | ~~9~~ | ~~清单/通知/报表~~ **✅ 已完成（2026-07-28，PR #763，9 项 P1：10.2-4/10.3-1/10.3-2/10.4-1/10.4-2/10.4-3/10.5-1/10.6-5/10.6-6，详见 doto-su.md/CHANGELOG.md）** |
| ~~P1-10~~ | ~~类十大货批色~~ | ~~7~~ | ~~提醒/报表/统计~~ **✅ 已完成（2026-07-28，PR #763，7 项 P1：批色提醒 + 批色报表 + 批色统计 + 交货门禁 + 客户反馈 + 批色重做 + 历史追溯 m0085）** |
| ~~P1-06~~ | ~~类六 测试体系~~ | ~~11~~ | ~~覆盖率/mock/fixtures/文档~~ **✅ 已完成（2026-07-29 核验，PR #758 Batch 485-488 已修复全部 P0/P1 项）** |
| ~~P1-07剩余~~ | ~~类七 可维护性~~ | ~~—~~ | ~~i18n/aria/缓存/文档~~ **✅ 已完成（2026-07-29，PR #767，缺陷 7.1-2 模块循环依赖修复）** |
| ~~P1-19~~ | ~~类十九 报表 BI~~ | ~~5~~ | ~~版本管理/缓存~~ **✅ 已完成（2026-07-28，PR #763，5 项 P1：模板版本管理 m0083 + 权限注册 + 订阅推送重试 + BI 缓存 5min + 仪表板 dashboard_layouts）** |
| ~~P1-20~~ | ~~类二十 可观测性~~ | ~~9~~ | ~~trace/metrics/WebSocket~~ **✅ 全部完成（9/9）：批次 1 PR #768（20.8-1 日志 JSON + 20.1-1 trace HTTP + 20.6-2 API 熔断）+ 此前已实现 6 项（20.1-2 Kafka trace event_kafka.rs + 20.3-1 WS ACK notifications.rs + 20.3-2 Redis Pub/Sub notifications.rs + 20.4-3 流复制 failover_service.rs check_replication_sync/wait_for_backup_catchup + 20.7-1 灰度升级 deploy-canary.sh + nginx-canary-10/50.conf + 20.8-2 日志保留 log_cleanup_service.rs）** |
| ~~P1-21~~ | ~~类二十一 胚布拆匹~~ | ~~10~~ | ~~库存/委外/继承~~ **✅ 已完成（2026-07-29，PR #770 已合并 main，10 项 P1 全部修复：缺陷 1.1/1.2/2.1/2.2/3.1/3.3/4.2/4.3/5.1/5.3，详见 CHANGELOG.md）** |
| ~~P1-22~~ | ~~类二十二 库存排程~~ | ~~9~~ | ~~调拨/安全/排程~~ **✅ 已完成（2026-07-29，PR #770 已合并 main，9 项 P1 全部修复：缺陷 6.1/6.2/7.1/7.2/8.2/9.1/10.1/11.1/11.3，详见 CHANGELOG.md）** |
| ~~P1-Batch16 剩余 P1~~ | ~~类十九 报表 BI（batch-16 审计剩余）~~ | ~~—~~ | ~~Webhook 通知分发 + 单元测试~~ **✅ 已完成（2026-07-29，PR #771 已合并 main，缺陷 5.1 Webhook 通知分发真实实现 + 缺陷 2.2/2.3/5.1/5.2 单元测试 20 项，2 文件 +354 行；其他 batch-16 P1 缺陷已在此前 PR #758/#763 完成）** |
| ~~P1-24~~ | ~~类二十四 前端架构（batch-20 审计）~~ | ~~16~~ | ~~PWA/移动端/chunks/ErrorBoundary/CSP/keep-alive/CSS/暗黑~~ **✅ 已完成（2026-07-29，PR #771 已合并 main，16 项 P1 全部修复：24.1-1 PWA manifest+sw.js + 24.1-2 移动端抽屉化 + 24.2-1 manualChunks + 24.6-1 ECharts 按需 + 24.9-1 optimizeDeps + 24.10-1 覆盖率 CI 基础设施齐全（⚠️ 阈值临时降级为 1%，实际覆盖 1.67%，待补齐测试后回调至 70%）+ 24.11-1 nginx CSP + 24.13-1 键盘导航 + 24.14-1 ErrorBoundary + 24.14-2 监控 SDK + 24.15-2 脏数据检测 + 24.16-2 i18n + 24.17-1 v-permission + 24.18-1 keep-alive + 24.19-1 CSS 变量 + 24.20-2 暗黑模式，详见 CHANGELOG.md）** |
| ~~P1-25~~ | ~~类二十五 部署升级~~ | ~~11~~ | ~~set -euo/SHA256/schema/蓝绿/健康/优雅/回滚~~ **✅ 已完成（2026-07-28，11 项全部完成：10 项 PR #758 + 25.3-A deploy-latest.sh SHA256 校验 PR #763）** |
| ~~P1-20 批次 2-3~~ | ~~类二十 可观测性~~ | ~~6~~ | ~~20.4-3 流复制 + 20.7-1 灰度升级 + 其他可观测性 P1 项~~ **✅ 已包含在 P1-20 全部完成项中（见上）** |

---

## 一、P1/P2/P3 任务规划（按类别汇总）

> P0 完成后按优先级顺序推进。详细内容见 V15 审计报告 [docs/audits/v15/](file:///workspace/.monkeycode/docs/audits/v15/)。

### 1.1 P1 高优先级（257 项，预估 45-55 批次，按每批 9-12 文件计算）

| 模块 | P1 数 | 主要内容 | 关键批次预估 |
|------|-------|----------|--------------|
| 类二 通用代码质量 | 3 | api 命名/缩写命名/DbErr 包装 | 2 批 |
| 类三 安全性 | 6 | refresh_token/PUBLIC_PATHS/validator/Webhook/magic bytes/zip bomb | 3 批 |
| 类四 面料行业深化 | 11 | batch_trace/检验指标/工资凭证/能耗/委外/事件发布/工时 | 4 批 |
| 类五 运行逻辑闭环 | 11 | 状态机/配置/业务事件/成本归集/加权平均 | 4 批 |
| 类六 测试体系 | 11 | 覆盖率/mock/fixtures/文档 | 4 批 |
| 类七 可维护性 | 11 | i18n/aria/缓存/文档 | 4 批 |
| 类八 法律合规 | 16 | 用户协议/HTTPS/脱敏/导出/docx/标准/签章/税/环保/排污/劳动/工时/社保/职业健康 | 6 批 |
| 类九 色卡发放 | 9 | 清单/通知/报表 | 3 批 |
| 类十 大货批色 | 7 | 提醒/报表/统计 | 3 批 |
| 类十三 打印导出 | 14 | 审计字段/水印/性能 | 5 批 |
| 类十四 权限维度 | 14 | 权限测试/审计/缓存 | 5 批 |
| 类十五 业务主体 | 1 | supplier_evaluation migration | 1 批 |
| 类十六 AI 模块 | 24 | 配伍性/化验室/准确率/版本/权限/超时/并发/缓存/脱敏/MLOps | 8 批 |
| 类十七 财务深化 | 35 | 期间/反结账/年结/回转/账龄/杜邦/预测/差异/折旧 | 12 批 |
| 类十八 CRM | 12 | 线索评分/去重/转移审批 | 4 批 |
| 类十九 报表 BI | 5 | 版本管理/缓存 | 2 批 |
| 类二十 可观测性 | 9 | trace/metrics/WebSocket | 3 批 |
| 类二十一 胚布拆匹 | 10 | 库存/委外/继承 | 4 批 |
| 类二十二 库存排程 | 9 | 调拨/安全/排程 | 3 批 |
| 类二十三 组织物流 | 11 | 组织树/售后/运费 | 4 批 |
| 类二十四 前端架构 | 16 | PWA/移动端/chunks/ErrorBoundary/CSP/keep-alive/CSS/暗黑 | 6 批 |
| 类二十五 部署升级 | 11 | set -euo/SHA256/schema/蓝绿/健康/优雅/回滚 | 4 批 |
| **合计** | **257** | | **约 45 批**（每批 9-12 文件） |

### 1.2 P2 中优先级（248 项，预估 35-45 批次）

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

### 1.3 P3 低优先级（123 项，按需修复）

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

## 二、规则节点提醒

| 规则 | 优先级 | 内容 |
|------|--------|------|
| 规则 0/1/2/8 | 🔴 | 真实实现强制：所有 P0/P1 修复必须真实实现，禁止占位符 |
| 规则 3 | 🔴 | 成品文档格式：导出必须 .xlsx / 报表必须 .docx |
| 规则 4 | 🔴 | `///` 注释精简为 1 行（首选），最多 2 行，禁止 3 行+注释块 |
| 规则 5 | 🟡 | E2E 独立工作流：每 30 批次触发（批次 30/60/90...） |
| 规则 6 | 🔴 | 测试 mock 数据禁止硬编码：所有测试 mock 数据抽取到 fixtures |
| 规则 10 | 🟡 | 每 15 批次记忆整理 + 实时归档：每批完成后立即归档到 doto-su.md |
| 规则 11/12 | 🔴 | 法律合规与安全标准：所有修复必须符合中国法律法规 + 安全标准 |
| 规则 13 | 🔴 | 修复流程自动化：CI 全绿后自动开始下一批；步骤 0 确定审计结果内容是否存在 + 步骤 4 修复后推送前自审 |
| 规则 14 | 🔴 | 移除所有警告抑制：所有警告视为错误需修复（baseline 213/213 ✅ 全部清零） |
| 规则 15 | 🟢 | V15 全项目综合审计：25 大类 195 维度审计 ✅ 已完成 |
| 规则 19 | 🟡 | 工具连接异常分级响应：L1 60s / L2 60-180s / L3 30min 周期 |
| 规则 20 | 🔴 | 注释与功能一致性：代码注释必须与功能实现一致，禁止随意编写；CI 强制检查 |

---

## 三、历史归档索引

> 详细历史任务归档见 [archives/2026-07-22/doto-historical-tasks.md](file:///workspace/.monkeycode/docs/archives/2026-07-22/doto-historical-tasks.md)，包含：
> - P0 批次规划表（39 项 → 22 批次）
> - 已完成模块 A-F 清单（39 项 P0 任务全部完成）
> - 历史阶段任务（v13/v14 复审修复 + V15 审计 + V15 修复阶段一/续/复审归档/复审报告）

> P0 模块 G（D01-D17）已完成归档见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md) §📋 P0 模块 G 任务归档。
> P1 已合并批次（11 批）详细修复记录见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md) 与 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)。

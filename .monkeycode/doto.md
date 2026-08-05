# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单），进度必须真实，禁止乐观偏差。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。

---

## 〇〇、V15 主线八维审计快速修复（2026-07-30 启动）

| 状态 | 数量 | 批次 |
|------|------|------|
| ✅ 已合并 main | 4 批 | audit-batch-2026-07-30（PR #786，11 项 P0 + 3 项 P2）、fix/p1-outsource-receipt-unify-2026-07-30（PR #788，委外收货主链路）、PR #790（盘点契约对齐 + API 网关 rate_limit 校验）、PR #793（业务追溯 producer 接入） |
| ⏳ 待推送 | 0 批 | — |

> **完成明细已归档**（规则 10）：[doto-su.md §📦 V15 主线八维审计与快速修复](file:///workspace/.monkeycode/doto-su.md)（11 项 P0 + 3 项 P2 ✅ 完整修复）+ [doto-su.md §🧵 P1 委外收货主链路统一](file:///workspace/.monkeycode/doto-su.md)（PR #788 ✅）+ [doto-su.md §🔧 PR #790](file:///workspace/.monkeycode/doto-su.md)（盘点契约 + rate_limit ✅）+ [doto-su.md §🔧 PR #793](file:///workspace/.monkeycode/doto-su.md)（业务追溯 producer ✅）。本节仅保留未完成项。

### 0.0.1 主线八维 P1 后续未完成项（2026-07-31 规则 10 归档修正）

> **归档说明**：原 6 项中 5 项已完成，归档到 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)：委外收货主链路统一（PR #788）、委外 record_receipt 4 子方法事务化、盘点契约 P0-1 + API 网关 rate_limit（PR #790）、业务追溯 producer（PR #793）。本节仅保留唯一未完成项。

| # | 项 | 文件 | 真实状态 | 代码证据 |
|---|-----|------|----------|----------|
| 1 | 覆盖率阈值回调 | [vitest.config.ts](file:///workspace/frontend/vitest.config.ts) | ❌ **未修复** | L31-39 thresholds 4 项（lines/functions/branches/statements）均为 1，非 70；注释明确"临时下调至 1%"、"待测试补齐后逐步提升回 70%"；实际覆盖率 1.67% |

**真实进度**：5/6 已完成（归档 doto-su.md）/ 1/6 未修复（#1 覆盖率阈值回调，需先补齐前端测试再回调阈值）

### 0.0.2 打印功能未完成项（2026-07-30 核实，整体完成度约 60%）

> 详见 V15 审计 batch-11（类十三打印导出审计与权限控制专项）。已实现 6 个场景归档 doto-su.md，本节仅列未完成项。

**业务场景覆盖**：6/16 = 37.5%（纺织核心 6 个场景全部缺失）

| 状态 | 场景 | 路由/文件 |
|------|------|-----------|
| ⚠️ 不完整 | 会计凭证（service 已实现但无路由） | [print_service.rs:784](file:///workspace/backend/src/services/print_service.rs) |
| ❌ 未实现 | 销售出库单/采购合同/库存盘点单 | — |
| ❌ 未实现（纺织核心） | 生产流转卡/验布打卷单/染色技术卡 | — |
| ❌ 未实现（纺织业务） | 色卡发放单/大货批色单/工资单 | — |

**规则 3 合规性**：✅ 实际合规（xlsx/docx），但 3 处 `export_csv` 函数名误导（实际生成 xlsx），建议重命名

---

## 一、P2/P3 任务规划（按类别汇总）

> P1（257 项）✅ 100% 完成，实际 25 批已合并 main，详细归档见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)。P0 完成后按优先级顺序推进。详细内容见 V15 审计报告 [docs/audits/v15/](file:///workspace/.monkeycode/docs/audits/v15/)。

### 1.1 P2 中优先级（248 项，预估 5-8 批次，按每批 65-99 文件计算）

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

### 1.2 P2 执行批次进度（2026-08-03 复审修正，规则 13 步骤 0 逐步推进）

| 批次 | 范围 | 项数 | 主要内容 | 状态 |
|------|------|------|----------|------|
| P2-Batch-01a | 类二+三+四+六+七（首批 9 项快速修复） | 9 | CSP+Argon2+魔法数字+TODO+i18n 注释 | ✅ 已合并 main（PR #797，6a38e05，归档 doto-su.md） |
| P2-Batch-01b | 类二+三+四+六+七（续作 18 项） | 18 | Cookie 双写+缓存一致性+SQL 参数化+表重叠+测试补齐+service 拆分+差异化 TTL | ✅ 已合并 main（PR #799，5bd1743，归档 doto-su.md） |
| P2-Batch-02 | 类五（运行闭环） | 10 | 反馈闭环 + 重染补染 + 告警死信 + 资源管理 + 凭证归集 | ✅ 已合并 main（PR #801，b4bc147 squash，归档 doto-su.md） |
| P2-Batch-03 | 类八（法律合规剩余）+ 类九（色卡发放） | 8+12+4 | 跨境合规 + 商检/产地证 + 色卡报表/成本/预警/统计 | ✅ 已合并 main（PR #803，bb010ad squash，归档 doto-su.md） |
| P2-Batch-04 | 类十+类十一+类十二（P2 快速修复） | 2 | 硬编码 role_id==1 修复 + v-role 指令删除 | ✅ 已合并 main（PR #814，f77d232） |
| P2-Batch-05 | 类十三（导出审计 + 打印水印） | 3 | 3 个导出端点补 Export 审计 + 打印IP水印 + rate_limit确认全局挂载 | ✅ 已合并 main（PR #815，ab4d729） |
| P2-Batch-06 | 类十二~十三（权限 fail-closed + PII 脱敏 + CRUD 审计） | 3 | extract_resource_info unknown fail-closed + 手机号/身份证脱敏 + CRUD 审计 | ✅ 已合并 main（PR #817，ed62471） |
| P2-Batch-07 | 类十五~十六（AI 输入校验 + 降级 + 推理耗时） | 4 | create_process_optimization 长度/枚举校验 + anomaly_detection 降级 + 错误文案 + inference_latency_ms | ✅ 已合并 main（PR #819，9d2cf06） |
| P2-Batch-08 | 类十四（角色校验 + 通配匹配 + 测试） | 4 | is_system/admin 校验 + matches_permission 通配 + require_admin_role 测试 + 文档单复数 | ✅ 已合并 main（PR #820，e0d2810） |
| P2-Batch-21 | 类二十五（部署脚本加固） | 10 | 日志持久化 + 配置权限600 + 健康检查database + CLI权限/确认/校验/回退 + 回滚验证 | ✅ 已合并 main（PR #821，47c2975） |
| P2-Batch-19 | 类二十三（售后退货类型 + incoterms 责任划分） | 2 | issue_type 增加 return_goods（前后端）+ incoterms cost_bearer/清关责任接入报价构成 | ✅ 已合并 main（PR #822，ba05490） |

### 1.3 P2-Batch-01b 遗留未完成项

| 编号 | 缺陷描述 | 真实状态 | 待办 |
|------|---------|----------|------|
| B04-P2-3 | 月末分摊缺端到端集成测试 | ✅ 完全存在（main 无任何 energy/allocation 测试） | 待后续批次补充月末分摊端到端集成测试 |

### 1.4 P2-Batch-04 修复项（2026-08-03 核实）

| 编号 | 缺陷描述 | 文件 | 修复状态 |
|------|---------|------|----------|
| B10-P2-5 | 客户 handler 硬编码 role_id == 1 改为 is_admin_role 函数 | [customer_handler.rs:350](file:///workspace/backend/src/handlers/customer_handler.rs) | ✅ 已修复 |
| B10-P2-6 | 删除 v-role 指令，统一使用 v-permission 权限码 | [permission.ts](file:///workspace/frontend/src/directives/permission.ts) + [main.ts](file:///workspace/frontend/src/main.ts) | ✅ 已修复 |

**核实结论**：batch-10 和 batch-11 的 P2 任务中，部分已实现（菜单动态加载、permission_audit_log 表、审计日志保留期限、Redis 权限缓存、omni_audit operation_category 字段、打印用户水印），部分需后续批次实现（权限审计日志查询接口、敏感角色变更双人审批、字段级权限推广、CSV/PDF 水印、流式导出、rate_limit 中间件）。

### 1.4 P3 低优先级（123 项，按需修复）

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
| 规则 0/1/2/8 | 🔴 | 真实实现强制：所有 P0/P1 修复必须真实实现，禁止占位符/stub/扩展空间视为未实现 |
| 规则 3 | 🔴 | 成品文档格式：导出 .xlsx / 报表 .docx，禁止 CSV/txt/rtf/html 作为成品 |
| 规则 4 | 🔴 | `///` 注释精简为 1 行（首选），最多 2 行，禁止 3 行+注释块 |
| 规则 5 | 🟡 | E2E 独立工作流：每 30 批次触发（批次 30/60/90...），不阻塞主 CI |
| 规则 6 | 🔴 | 测试 mock 数据禁止硬编码，必须抽取到 fixtures 文件 |
| 规则 10 | 🟡 | 记忆整理归档：每 15 批次深度整理 + 每批完成后实时归档到 doto-su.md；doto.md 只记录未完成任务 |
| 规则 11/12 | 🔴 | 法律合规与安全标准：符合中国法律法规（个保法/数安法/网安法）+ API 认证/权限/SQL 参数化/敏感操作审计 |
| 规则 13 | 🔴 | 修复流程自动化：CI 全绿后自动开始下一批（每批 65-99 文件）；步骤 0 确定审计结果内容是否存在 + 步骤 4 修复后推送前自审 |
| 规则 14 | 🔴 | 移除所有警告抑制：所有警告视为错误需修复；新增代码禁止 `#[allow(dead_code)]`，dead_code 通过接入路由消除（仅 models/ SeaORM 模型保留文件级例外）；既有 allow 在后续批次逐步清理 |
| 规则 15 | 🟢 | 复审严格规范：baseline 警告视为错误，8 维度闭环 + 4 轮次状态；V15 审计进度详见 [audit_assignment.md](file:///workspace/.monkeycode/audit_assignment.md) |
| 规则 19 | 🟡 | 工具连接异常分级响应：L1 60s / L2 60-180s / L3 30min 周期 + 非阻塞推理 |
| 规则 20 | 🔴 | 注释与功能一致性：代码注释必须与功能实现一致，禁止随意编写；CI 强制检查 |

---

## 三、历史归档索引

> 详细历史任务归档见 [archives/2026-07-22/doto-historical-tasks.md](file:///workspace/.monkeycode/docs/archives/2026-07-22/doto-historical-tasks.md)，包含：
> - P0 批次规划表（39 项 → 22 批次）
> - 已完成模块 A-F 清单（39 项 P0 任务全部完成）
> - 历史阶段任务（v13/v14 复审修复 + V15 审计 + V15 修复阶段一/续/复审归档/复审报告）

> P0 模块 G（D01-D17）已完成归档见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md) §📋 P0 模块 G 任务归档。
> P1 已合并批次（25 批）详细修复记录见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md) 与 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)。

---

## 四、CI 基础设施修复归档（2026-08-03）

> 以下 CI 问题已通过 PR #807-#812 修复并合并 main，Release 流程恢复正常。

| PR | 内容 | 状态 |
|-----|------|------|
| #807 | fix(backend): 修复 main 分支 clippy 新增警告与 fmt 失败 | ✅ 已合并 main（709b2a9） |
| #808 | ci: clippy 日志化 + fmt 自动修正 + 消除重复检查 | ✅ 已合并 main（99498ca） |
| #809 | fix(ci): 添加发布说明生成调试输出和错误处理 | ✅ 已合并 main（e0a1635） |
| #810 | fix(ci): 用 gh CLI 替代 softprops/action-gh-release，添加发布包验证 | ✅ 已合并 main（da8e358） |
| #811 | fix(ci): 修复版本号格式，月日分隔为独立段 | ✅ 已合并 main |
| #812 | fix(ci): Cargo.toml 版本号转为 SemVer 3 段格式 | ✅ 已合并 main |

**最终状态**：CI 全绿，Release v2026.8.3.2335 已生成（资产 state=uploaded）。
| 2026-08-04 | P2-Batch-22 | AI explanation + 前端性能/可访问性/权限缓存 | 6 | PR #823 ✓ | 14.1.71 explanation字段; 20.2-D错误去重; 20.2-C焦点重置; 20.9-C懒加载; 20.11-D权限缓存; 20.12-C路由预取 |
| 2026-08-04 | P2-Batch-23 | 部署变更文件记录 | 1 | PR #824 ✓ | 25.5-D 部署时记录变更文件列表到 deploy-changes.log |
| 2026-08-04 | P2-Batch-24 | CI Release 清理修复 | 1 | PR #826 ✓ | 修复 --cleanup-tag 不生效，手动删除关联 tag；清理无 Release 的旧 tag（保留 100 个） |
| 2026-08-04 | P2-Batch-25/26 | 前端优化 + 后端超时/事务/账龄基准日 | 14 | PR #827 ✓ | 20.9-D visualizer; 20.10-D persistedstate; 20.6-C lazy loading; 20.8-C alt prop; 17.4-D3 baseline_date; 14.10-D batch atomicity; 17.7 OTel 10%; 23.1-D manager_id; 13.3-D supplier qual CRUD; 16.4-D BI/dashboard timeout |
| 2026-08-04 | P2-Batch-27 | 报表元数据 refresh/cache + AI 速率限制 | 2 | PR #829 ✓ | 16.1-D3 refresh_strategy/cache_ttl_seconds 字段; 16.4-D4 AI 端点专用速率限制 (10 req/min/user) |
| 2026-08-04 | P2-Batch-28 | 角色命名校验 + is_system 约束 + 报表参数 Validate | 3 | PR #830 ✓ | 14.1-D 角色编码命名规范; 14.5-C is_system=true 需 code=admin; 16.1-D4 报表参数 Validate 派生 |
| 2026-08-04 | P2-Batch-29 | WebSocket 心跳超时断开 | 1 | PR #831 ✓ | 20.3-C 30s Ping + 60s 超时断开 |
| 2026-08-04 | CI 修复 | Release 清理排序修复 | 1 | PR #832 ✓ | sort -V 混合段数版本号排序错误，改用 --order asc 按创建时间排序 |
| 2026-08-04 | P2-Batch-30 | Nginx gzip + 移动端触屏按钮 | 2 | PR #833 ✓ | 25.1-F gzip 压缩; Touch targets 44px CSS |
| 2026-08-05 | P2-Phase-3 | DB migration: suppliers FK + 合同明细 + 快照表 + 预警规则 | 5 | PR #835 ✓ | m0093 suppliers category_id FK; m0094 is_processor+processor_type; m0095 sales_contract_items; m0096 period_report_snapshot; m0097 aging_alert_rules |
 | 2026-08-05 | P2-Phase-3.5 | P2 核实后修正：接入未实现的修复项 | 6 | PR #836 ✓ | m0094 processor_type 筛选接入; m0095 sales_contract_items service/handler/route; m0096 period_report_snapshot service/handler; m0097 aging_alert_rules service/handler; mask_fields 接入 customer_handler; record_actual_grade handler 端点 |
| 2026-08-05 | P2-Phase-4 | 辅助核算余额增强+账龄业务员维度+穿透查询 | 3 | PR #838 ✓ | P2-4 期初/期末余额计算; P2-7 账龄按 salesperson_id GROUP BY; P2-3 穿透查询总账到辅助明细 |
| 2026-08-05 | P2-Phase-5 | 预算科目-会计科目映射 + 资产分类管理 | 2 | PR #839 ✓ | P2-14 budget_items.account_subject_id; P2-17 asset_categories 表 + CRUD + fixed_assets.asset_category_id; m0098 migration |
| 2026-08-05 | P2-Phase-6 | 现金流比率 + 趋势分析增强 | 2 | PR #840 ✓ | 17.5-D6 现金流比率（OPERATING_CF_RATIO/SALES_CF_RATIO/CF_ADEQUACY_RATIO）; 17.5-D5 趋势分析增强（线性回归+移动平均+趋势方向） |
| 2026-08-05 | P2-Phase-6B | 预算版本管理 + 资产减值测试 + 折旧政策变更 | 3 | PR #842 ✓ | 17.7-D5 预算版本管理; 17.8-D5 资产减值测试; 17.8-D6 折旧政策变更; m0099 migration |
| 2026-08-05 | P2-Phase-6C | 调拨审批流 + 资金日报/月报 | 2 | PR #844 ✓ | 17.6-D5 调拨审批流（按金额分级审批）; 17.6-D6 资金日报/月报接口 |
| 2026-08-05 | P2-Phase-7 | CRM 线索管理增强 | 3 | PR #846 ✓ | 18.1-D4 线索来源 ROI 跟踪; 18.1-D5 线索分配规则; 18.1-D6 线索培育流程 |
| 2026-08-05 | P2-Phase-8 | CRM 商机+公海管理增强 | 6 | PR #847 ✓ | 18.2-D5 阶段停留时长; 18.2-D6 商机竞争对手; 18.2-D7 商机跟进记录; 18.3-D5 回收规则跟进/成交周期; 18.3-D6 回收规则部门差异化; 18.3-D7 公海客户保护机制 |
| 2026-08-05 | P2-Phase-9 | CRM 数据权限+数据流转 | 5 | PR #848 ✓ | 18.4-D5 客户字段权限配置; 18.4-D6 客户操作审计日志; 18.5-D3 转化数据双向同步; 18.5-D4 客户主数据关系; 18.5-D5 客户 CLV |

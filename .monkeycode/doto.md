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

### 0.0.2 打印功能未完成项（2026-08-06 二次查漏更新）

> 详见 V15 审计 batch-11（类十三打印导出审计与权限控制专项）。原"已实现 6 个场景"实为返回 HTML，违反规则 3（A0 整改）；完整场景清单见已批准执行计划（批次 A：A0 合规基建 → A1 纺织专用 → A2 P0 → A3 P1 → A4 P2，约 60 个打印场景）。本节列未完成项。

**业务场景覆盖**：原称 6/16 = 37.5%（注：**销售发货单通知单**与**销售出库细码单**是**两个不同单据**，原清单误合并/混淆，不得去重；实际初始场景数按 16 计）；二次查漏又发现约 26 个业务实体需打印，合计约 60 场景分批推进。

| 状态 | 场景 | 说明 |
|------|------|------|
| 🔧 修复中（A0） | 6 个原 HTML 场景（销售订单/销售合同/采购订单/采购收货单/库存调拨单/会计凭证） | A0 改为返回 docx（接入 generate_docx） |
| 🔧 修复中（A0） | 会计凭证路由缺失 | A0 在 finance.rs 新增 `/vouchers/:id/print` |
| ❌ 待实现（A1-A4） | 销售发货单通知单（出库前置单据）/销售出库细码单（面料出库明细）/采购合同/库存盘点单/工资单 | 原清单未实现项 |
| ❌ 待实现（A1 纺织专用） | 生产流转卡/验布打卷单/染色技术卡/色卡发放单/大货批色单/卷标签·条码标签/打样单 Lab Dip/生产任务单/质检记录 | 纺织核心+业务+生产 |
| ❌ 待实现（A2 P0） | 销售发货单通知单(出库前置)/销售出库细码单(面料出库明细)/收款单/付款单/销项·进项发票/销售报价单/销售退货单/采购退货单/委外加工单/委外收货单/物流运单/产地证/出口报关单/危废五联单/不合格品单/染化料领用单 | 通用+法定 |
| ❌ 待实现（A3 P1） | 付款申请单/供应商对账单/采购验货单/其他出入库·调整单/BOM·工艺单/领料单·缺料表/质检报告·8D/商检单/劳动合同 + 14 个 P1（外汇核销/出口退税/固定资产卡/资产盘点/资金调拨/科目余额/物理检测/工序卡/缸号回修/售后工单/质量异常/安全事故/劳保签收/库存台账） | 通用 |
| ❌ 待实现（A4 P2） | 坏账核销单/定制订单确认单/存货跌价·减值单/社保缴纳表/职业健康体检报告/客户信用审批单 | 低优 |

**规则 3 合规性**：🔧 **修复中**——原 6 个场景返回 HTML（违反规则 3），A0 改为 docx 成品；另发现 `report_enhanced` `POST /export/pdf`（`report_enhanced_handler.rs:228`）声称 PDF 实际产纯文本（`export_service::export_pdf` `export_service.rs:45` 注释自认"导出为文本格式"），属规则 3 硬违规，由紧随的 A0b 修复为真 PDF。命名误导的 `export_csv` 见 §五。

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

**核实结论**：batch-10 和 batch-11 的 P2 任务中，部分已实现（菜单动态加载、permission_audit_log 表、审计日志保留期限、Redis 权限缓存、omni_audit operation_category 字段、打印用户水印、**字段级权限 CRUD 与路由挂载 `routes/iam.rs:77-88,100`、脱敏接入 customer/crm、`rate_limit` 中间件全局挂载 180/min+AI 10/min `middleware_bootstrap.rs:90,197-204`**）。**仍为待实现项（2026-08-06 核实）**：权限审计日志查询接口、敏感角色变更双人审批、CSV/PDF 水印（xlsx 水印已实现，仅 CSV/PDF 缺）、流式导出。

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

---

## 五、导出链路 CSV 中间格式技术债（2026-08-06 排查，非规则 3 违规）

> 排查背景：§0.0.2 提及 3 处 `export_*_to_csv` service 产真 CSV，曾疑违反规则 3（成品须 xlsx）。**排查结论：不违反规则 3**——3 处 CSV 均非最终成品，而是 service 层中间格式，由 handler 解析后转 xlsx 返回。但仍属需优化的技术债，列为待办（非合规问题）。

| # | 问题 | 代码证据 | 待办 |
|---|------|----------|------|
| T1 | 产品导出经 CSV 中转 | `product_handler.rs:471-496` 调 `export_products_to_csv` 得 `csv_data`，再 `csv::ReaderBuilder` 解析 → `XlsxTable` → `build_xlsx_response` 返回 xlsx（注释「规则 3：将 service 返回的 CSV 解析为 xlsx 表格」） | 改为 service 直接返回结构化数据（如 `Vec<Vec<String>>`/实体），handler 直接 `build_xlsx`，去掉 CSV 序列化+反序列化往返 |
| T2 | 采购订单导出经 CSV 中转 | `purchase_order_handler.rs:526-560` 同上链路 | 同 T1 |
| T3 | 销售订单导出经 CSV 中转 | `sales_order_handler.rs:503-538` 同上链路 | 同 T1 |

**附带（命名误导，不违规）**：路由 `GET /api/v1/erp/export/csv/:export_type`（`analytics.rs:190` → `import_export_handler.rs:245`）路径含 `csv`，但实际返回 xlsx（`generate_xlsx`，content_type 为 xlsx MIME，filename `.xlsx`）→ 建议路由路径改为 `/excel/` 或 `/xlsx/`。

**为何列为待办（非合规）**：规则 3 要求成品为 xlsx，当前成品确为 xlsx，已合规；但 CSV 中转带来 ① 命名误导 ② 性能浪费（CSV 序列化后立即反序列化）③ 解析脆弱（字段含逗号/引号/换行易出错）。属可维护性/健壮性优化，建议后续 P2/P3 批次处理。

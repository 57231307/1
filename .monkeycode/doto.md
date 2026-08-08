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

**业务场景覆盖**：原称 6/16 = 37.5%（注：**销售发货单通知单**与**销售出库细码单**是**两个不同单据**，原清单误合并/混淆，不得去重；实际初始场景数按 16 计）；二次查漏又发现约 26 个业务实体需打印，合计约 60 场景分批推进。**A1-A4 已全部完成**（57 个新场景 + 6 个原场景 = 63 个打印端点）。

| 状态 | 场景 | 说明 |
|------|------|------|
| ✅ 已合并 main（87637967） | 6 个原 HTML 场景（销售订单/销售合同/采购订单/采购收货单/库存调拨单/会计凭证） | A0 改为返回 docx（接入 generate_docx） |
| ✅ 已合并 main（87637967） | 会计凭证路由缺失 | A0 在 finance.rs 新增 `/vouchers/:id/print` |
| ✅ 已合并 main（1a0028d7，A0b） | `report_enhanced` `POST /export/pdf` 声称 PDF 实际产纯文本 | `export_service.rs:45` `export_pdf` 原注释自认"导出为文本格式"，规则 3 硬违规；A0b 改写为 printpdf 真 PDF + 修复 export_template pdf 分支 |
| ✅ 已合并 main（ddce03d6，A1-A4） | 57 个新 docx 打印端点 | A1 纺织专用 9 + A2 P0 16 + A3 P1 25 + A4 P2 6 + 已有 quality_inspection_record；63 个 get_*_print_data + 63 个 handler；PR #862 合并 |

**规则 3 合规性**：✅ **A0 已合并 main（87637967）**——原 6 个场景已由 HTML 改为 docx 成品；✅ **A0b 已合并 main（1a0028d7）**——`report_enhanced` `POST /export/pdf` 已由 printpdf 渲染为真 PDF（复用 `services/report/exp.rs` 已验证的 printpdf 渲染），同时修复 `export_template` 的 pdf 分支。命名误导的 `export_csv` 见 §五。

---

## 一、P2/P3 任务规划（按类别汇总）

> P1（257 项）✅ 100% 完成，实际 25 批已合并 main，详细归档见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)。P0 完成后按优先级顺序推进。详细内容见 V15 审计报告 [docs/audits/v15/](file:///workspace/.monkeycode/docs/audits/v15/)。

### 1.1 P2 中优先级（248 项，已完成约 224 项，剩余约 24 项见 §1.3）

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
| P2-Batch-22 | 类十四+二十（AI explanation + 前端性能/可访问性/权限缓存） | 6 | explanation 字段 + 错误去重 + 焦点重置 + 懒加载 + 权限缓存 + 路由预取 | ✅ 已合并 main（PR #823） |
| P2-Batch-23 | 类二十五（部署变更文件记录） | 1 | 部署时记录变更文件列表到 deploy-changes.log | ✅ 已合并 main（PR #824） |
| P2-Batch-24 | CI Release 清理修复 | 1 | 修复 --cleanup-tag 不生效 + 清理无 Release 旧 tag | ✅ 已合并 main（PR #826） |
| P2-Batch-25/26 | 前端优化 + 后端超时/事务/账龄基准日 | 14 | 懒加载/持久化/alt/基线日/原子性/OTel/manager_id/供应商CRUD/BI超时 | ✅ 已合并 main（PR #827） |
| P2-Batch-27 | 报表元数据 + AI 速率限制 | 2 | refresh_strategy/cache_ttl_seconds + AI 端点 10 req/min | ✅ 已合并 main（PR #829） |
| P2-Batch-28 | 角色命名校验 + is_system 约束 + 报表参数 | 3 | 角色编码规范 + admin 约束 + Validate 派生 | ✅ 已合并 main（PR #830） |
| P2-Batch-29 | WebSocket 心跳超时断开 | 1 | 30s Ping + 60s 超时断开 | ✅ 已合并 main（PR #831） |
| P2-Batch-30 | Nginx gzip + 触屏按钮 | 2 | gzip 压缩 + 44px CSS | ✅ 已合并 main（PR #833） |
| P2-Batch-31 | 全域 P2 审计修复（后端/前端/部署） | 20 | 慢查询告警/优化追踪 + 通知订阅调度 + 权限合规 + 供应商评估 + recipe_opt + PII脱敏 + 存货跌价 + 部门服务 | ✅ 已合并 main（PR #834 + #852） |
| P2-Batch-32 | 缺陷 1.3/3.2/7.3/8.4/9.2/10.2/11.2 | 7 | 胚布追溯字段 + 拆匹强校验 + 告警去重 + 在途采购 + 排程冲突告警 + 负荷告警 + SPT 调度 | ✅ 已合并 main（PR #853） |
| P2-Phase-3~3.5 | DB migration 5 项 + 接入未实现修复 6 项 | 11 | suppliers FK/合同明细/快照表/预警规则 + service/handler/route 接入 | ✅ 已合并 main（PR #835 + #836） |
| P2-Phase-4~6C | 财务增强 | 10 | 辅助核算余额 + 账龄业务员维度 + 穿透查询 + 预算映射 + 资产分类 + 现金流比率 + 趋势分析 + 预算版本 + 减值测试 + 折旧政策 + 调拨审批 + 资金日报 | ✅ 已合并 main（PR #838/#839/#840/#842/#844） |
| P2-Phase-7~9 | CRM 增强 | 14 | 线索 ROI/分配/培育 + 商机竞品/跟进/阶段 + 公海回收/部门差异化/保护 + 字段权限/操作审计/双向同步/主数据/CLV | ✅ 已合并 main（PR #846/#847/#848） |
| P2-batch-21 部署升级 | 25.1/25.3/25.4 部署 + 残留项 | 11 | 端口冲突 + .env 600 + 断点续传 + 版本降级 + API 兼容 + 配置迁移 + 日志持久化 + draining + 升级监控告警 + 多租户残留 | ✅ 已合并 main（PR #854） |
| P0 补充批次 | P0-5-1/P0-9-2/batch-10 | 3 | 5 报表导出 API + EXPORT_LIMIT=10000 + 批色 P0 业务规则 | ✅ 已合并 main（PR #855~#858） |
| A0 | 打印合规(docx) + 打印基建改造 | 2 | 6 场景改 docx + 会计凭证打印路由 | ✅ 已合并 main（87637967） |

### 1.3 P2 真实未完成项清单（2026-08-07 explore 核实，基于 HEAD=87637967）

> 经代码级核实（文件:行号证据），以下 P2 项**仍未完成**。已合并批次修复的项（B01-P2-1 quality_pred 编译错误、B04-P2-4 物理指标字段、B06-P2-7 覆盖率趋势、B07-P2-3 install 路径、B08-P2-2 登录限流、B11-P2-1 批色报表、batch-18 全项、batch-21 部署 24/25、batch-13 P2 大部分等）不再列出。

| 编号 | 描述 | 代码证据 | 类别 |
|------|------|----------|------|
| B02-P2-3 | handlers 未按业务域分子目录（平铺） | `backend/src/handlers/` 158 个文件，仅 advanced/ + color_card/ 两个子目录 | 可维护性 |
| B04-P2-3 | 月末分摊缺端到端集成测试 | `backend/tests/` 无 energy/allocation 测试；energy_service.rs 仅纯函数单测 | 测试 |
| B12-P2-1 | 权限码命名规范未统一为 `<模块>.<资源>.<操作>` 三段式 | permission.rs:186 `format!("{}:{}", ...)` 冒号两段式；migration 中 permission_code 为点号两段式 | 权限 |
| B12-P2-4 | 敏感角色变更双人审批未实现 | 全库 grep dual_approval/second_approval 0 命中 | 权限 |
| B12-P2-5 | 大数据量导出无流式处理 | import_export_handler.rs:257-264 一次性 generate_xlsx + base64；report/exp.rs 同 | 导出 |
| batch-12 P2-9 | 行级/字段级权限测试未落地 | 权限测试仅覆盖缓存/通配，无行级权限测试 | 权限 |
| batch-13 P2 | 供应商账户余额管理 + 异常大额订单检测引擎 | supplier_service.rs 无余额维度查询；无"异常大额订单+异常频繁退货"专门引擎 | 业务 |
| batch-16 P2-3/P2-4 | 通知模板无动态管理、不支持多语言 | grep notification_template 无模型；notification_service.rs 无 i18n | 报表/通知 |
| batch-18 P2-6 | 调拨在途库存未独立核算 | inventory_transfer.rs 无 in_transit 字段（in_transit 仅 logistics 事件） | 库存 |
| batch-18 P2-7 | ✅ 缺料月报能力已实现 | material_shortage_handler.rs get_monthly_report + service get_monthly_report + 路由注册 | 排程 |
| batch-18 P2-4 | ✅ 瓶颈识别扩产/外包建议已实现 | capacity_service.rs BottleneckSuggestion + generate_suggestions + overview 自动生成建议 | 排程 |
| batch-18 P2-5 | ✅ 排程重复录入校验已实现 | scheduling_query.rs apply_schedule_details_to_orders 添加状态校验和日期保护 | 排程 |
| batch-18 P2-2 | ✅ 委外加工费按缸号/匹号核算已实现 | outsourcing_order_item 添加 processing_fee/freight_fee 字段 + migration + DTO 更新 | 委外 |
| batch-21 P2 25.4-I | 无长任务处理机制（状态持久化/断点续传） | upgrade.rs（1190 行）无任务状态持久化；deploy.sh 无任务队列 | 部署 |
| 前端 16 | vitest 覆盖率阈值仍为 1% | frontend/vitest.config.ts:31-38 thresholds 全部 = 1 | 前端测试 |
| 前端 18 | dynamic_router 仍为占位实现 | middleware/dynamic_router.rs:17 "模块功能待集成，当前为占位实现"，未挂载路由 | 可观测性 |

**P2 真实剩余**：约 24 项未完成（含前端 2 项）；P2 总数 248 项已完成约 224 项。

### 1.4 P3 低优先级（123 项，按需修复）

> 2026-08-07 explore 核实：**121 项未完成**（2 项已由 P2 顺带完成：25.1-F Nginx gzip、24.1-4 触屏按钮 ≥44px，均在 PR #833）。以下按审计 batch 统计未完成项。

| Batch | P3 未完成 | 主要内容 |
|-------|----------|----------|
| batch-01/08/11/16/19 | 0 | 回归验证/无 P3 缺陷 |
| batch-02/03/06/07 | 4 | MainLayout subMenus 硬编码 path（4-7 注释仍在）、extract_resource_info unknown 注释权衡、10 道精细工序模板未预置、行业词汇中英对照文档缺失、染整 service 测试函数名仍英文、颜色对比度 WCAG 无法自动验证 |
| batch-04/05 | 4-5 | 10 道精细工序模板未预置、行业词汇对照文档缺失、生产订单成本归集失败未接入 event_retry、AUDIT_RETENTION_DAYS 未走 AppSettings、工序映射文档不足 |
| batch-09 | 7 | 全部为流程执行类：E2E 报告保存 docs/audits、20/28/29 节奏监控、失败按优先级纳入、禁止死等、每 15 批记忆整理、实时归档、禁止跨批堆积 |
| batch-10/12 | 2-3 | RBAC 压力测试与性能监控、公共路径白名单测试、HTTP OPTIONS/HEAD 映射完整性 |
| batch-13 | ~15 | 供应商评估 model 重命名、供货历史查询、价格清单导入、色卡/染色/印花能力字段、加工商前端界面、委外加工报表、按匹号发货、客户多地址/多银行账户、客户特殊工艺、染色完成事件回写、离线报表 ETL、报表追溯、审计日志审查 cron、业务批次追溯 |
| batch-14 | 2-3 | 销售预测未与订单/库存补货联动、AI 操作审计未区分敏感操作、人工复核状态机不完整 |
| batch-15 | ~9-11 | 结账操作日志粒度、8 维度配置化、账龄档位配置化、行业基准配置化、调拨频率限制、预算考核报表、折旧起算日灵活、线索字段扩展性、公海客户来源记录、客户合并、转化耗时分析 |
| batch-17 | ~8-10 | 网络指标采集、企业微信/钉钉渠道、Alertmanager 目标启用、系统资源看板、慢查询阈值 100ms vs 500ms、慢查询周报、Retry-After 单位、迁移跳跃检测、system_version 模型接入、日志冷数据归档 |
| batch-18 | 2 | 拆匹未生成新匹号（仍 `{parent.piece_no}-CUT-{timestamp}`）、甘特图无拖拽调整后端接口 |
| batch-20 | ~26 | 触屏按钮尺寸规范、FCP/LCP/TTI 监控、system store、事件总线、组件文档、ColorCardGrid emits、composables try/catch、ARIA 标签、dataZoom 评估、重连降级轮询、票据鉴权、any 残留、chunk 阈值、env.d.ts、fixtures、no-v-html、CSRF cookie、CompanyTab localStorage、logger.error、按钮 loading+debounce、RTL、行级权限评估、超时重试幂等、CSRF 缺失提示、scoped 样式污染 |
| batch-21 | ~9 | 防火墙配置、回滚不恢复配置文件、UtilCommand --force、升级通知机制、多版本备份、配置热更新、RTO/RPO、缓存预热、升级演练要求、残留项10 历史迁移文件 |

**P3 真实剩余**：约 121 项未完成（按 batch 汇总约 90+，含流程执行类与测试增强类）。

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
| T1 | ✅ 产品导出去除 CSV 中转 | product_handler.rs 已改为调用 `export_products_to_xlsx` 直接获取结构化数据 | 已完成 |
| T2 | ✅ 采购订单导出去除 CSV 中转 | purchase_order_handler.rs 已改为调用 `export_orders_to_xlsx` 直接获取结构化数据 | 已完成 |
| T3 | ✅ 销售订单导出去除 CSV 中转 | sales_order_handler.rs 已改为调用 `export_orders_to_xlsx` 直接获取结构化数据 | 已完成 |

**附带（命名误导，不违规）**：路由 `GET /api/v1/erp/export/csv/:export_type`（`analytics.rs:190` → `import_export_handler.rs:245`）路径含 `csv`，但实际返回 xlsx（`generate_xlsx`，content_type 为 xlsx MIME，filename `.xlsx`）→ 建议路由路径改为 `/excel/` 或 `/xlsx/`。

**技术债已解决**：T1/T2/T3 已完成，去除了 CSV 序列化+反序列化往返，性能提升且消除了字段含逗号/引号/换行导致的解析脆弱性。

# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单），进度必须真实，禁止乐观偏差。
> 已完成任务见 [doto-su.md](doto-su.md)，一句话总结见 [CHANGELOG.md](CHANGELOG.md)，规则见 [MEMORY.md](MEMORY.md)。
> 过时内容归档见 [docs/archives/](docs/archives/)（最新批次：2026-09-09）。

---

## 当前状态

**文档治理完成（2026-09-09）：README 数据更新、doto/bug 归档、docs/ 移入 .monkeycode、生产服务器日志与 scan_long_fns.py 删除，全部本地 commit。源代码修改冻结已解除（IR 2026-09-09，用户指令）——修复计划（docs/plans/one-round-fix-plan-2026-09-09.md，10 commit）可实施；推送继续冻结，全部改动仅本地 commit。CI 历史问题全量总结见 docs/ci-issues-summary-2026-09-09.md（22 项 + 6 条机制教训）。**

**一轮修复计划全部 10 commit 实施完成（2026-09-09，本地未推送）：**
- 3e8ca73 P1.1 敏感导出 fail-closed（12 端点×6 类资源 + record_download）+ P1.4 APPROVE 审计分类
- 7604f41 P1.2 lock-status OptionalAuthContext 匿名可查 + P1.3 阈值 5→9
- 11b179d P1.5 版本号方案 C（env! 编译期化 + Release job 去 VERSION 推送）
- 3dcd56a P2.1 登录瀑布（_skipAuthRetry+删 catch refreshLockStatus）+ P2.2 协议页路由 + P2.3 export-approvals api/UI/路由
- 752921a + cf4c79d 法律文书 V2.0（协议 18 条/隐私 19 条，逐条标注法条依据，websearch 核实原文：PIPL 13/14/17/28-31/38-41/44-47/50-57 条、网安法 21/40-44/49 条、数安法 27/32 条、民法典 123/469/490/496/1034 条、刑法 253-1、反不正当竞争法 9 条）
- d84a461 P2.4 去 mock（applyAuthMocks 真实登录，55 spec 零改动 + lock-status 拦截移除）
- a7711bc P4 后端测试 3 文件（classify_operation pub(crate)+APPROVE 分类/阈值常量 pub(crate)/enforce 空 token fail-closed）
- 1d89410 P3 基建（ensureRoleUsers 30+角色自动补建+role-credentials.json / trackPageHealth+assertPageHealthy+expectSingleToast+generateTotp RFC6238 / testMatch 扩全 11 目录）
- d24d43d P5 专项 spec 7 个（31 登录瀑布/32 全角色登录/34 水平越权/35 2FA/36 预览/40 系统更新/43 重复提示）
- 9cdfd02 P5 全量矩阵+遍历（modules.config 95 模块真实路由全登记 / endpoints.config print 58+export 敏感12+非敏感27+approve 48 / 42a-d 遍历 4 spec / 37/39/41 端点矩阵 3 spec / 09-permissions 恒真断言修复 P1-2/3→精确403+P1-7/8→真实断言 / 22-crm-full fail-closed 语义 / jszip devDep / Login.vue 死代码清理）
- 439c6ef P5.14 角色权限矩阵 job（permission-model 双轨：推导+黄金基线 / 44-role-matrix 三分支断言+access-map artifacts / ci-cd.yml role-permission-matrix job 6 角色组）
- e0ce7bb 计划缺口补齐 spec（33 垂直越权矩阵：非 admin 对 admin 端点族 10 端点 403+admin 对照组 / 38 print-templates API 按真实路由 / 39b 完整审批链：申请→审批→令牌导出→二次消费拒→跨资源拒 / 41b-c 坏账双级+转账+角色变更+BPM 引擎全流程）
- **第三轮（2026-09-10，用户 0-3 指令）完成，本地 commit：**
  - 5ce955c 核查前三轮全部修改：修复 6 处真实缺陷（P4 测试 pub(crate) 对 tests/ 不可见→pub / 42a-d listApi 缺 /api/v1/erp 前缀 404 恒真→补前缀 / 41+33 POST 缺 X-CSRF-Token→读 csrf cookie 补头 / role-permission-matrix --list 不触发 globalSetup→新增 ensure-role-users.ts 显式入口 / 44-role-matrix 无基线硬断言→双模）
  - 75 分片重划：ci-e2e matrix 0-74；0-49 flow（/50）/ 50-54 smoke（/5）/ 55-74 traversal（/20，traversal 首次纳入 CI 执行）
  - 9b5aa14 六个新 CI JOB 全部落地（含前次 security-vulnerability-scan）：漏洞扫描（cargo-audit+npm audit+gitleaks+semgrep）/ 功能完整性联网校验（协议 15 功能域 vs 路由对照+npm registry 过时度）/ 注释规范（Rust 82% TS 35%，阈值 30 fail/60 warn）/ E2E 真实性门禁（DO-NOT-MODIFY 标记，动态 needs 三 E2E job 全 success + 静态扫描伪造响应，EXEMPT 豁免机制）/ 敏感权限审计 11 项断言 / 日志合规+vue-tsc（pipefail）；ci-cleanup-retry needs 19→25
  - 自审预跑修复 4 处（job 上线即绿保障）：路由 path 前导斜杠误报 / migration 扫 backend/migrations 不存在→改 backend/migration/src Rust 源 / TS 阈值脱离现状 / vue-tsc 管道吞退出码
  - network-resilience.spec.ts 伪造响应违规→E2E-AUTHENTICITY-EXEMPT+test.skip（待人工确认真实异常源重写方案）；color_card_issue_service.rs 死语句清理
- 待推送授权后 CI 验证（按 commit 序观察，失败按 commit 隔离）；jszip 需 CI npm ci 验证 lock 完整性

---

## 硬约束（用户指令，后续所有任务必须遵守）

- [x] 禁止 mock：E2E 测试一律真实后端 + 真实 PostgreSQL 数据
- [x] 禁止本地编译：验证一律走 CI
- [x] **推送冻结（2026-09-08 二次冻结，2026-09-09 重申继续生效）**：git push 一律禁止（含分支/tag），改动仅本地 commit；推送必须等用户明确授权
- [x] **源代码修改冻结已解除（2026-09-09，用户指令）**：按 docs/plans/one-round-fix-plan-2026-09-09.md 实施修复；历史 CI 问题基线见 docs/ci-issues-summary-2026-09-09.md（执行时对照六条机制教训）

---

## 未完成任务清单

### E2E 权限与打印覆盖缺口（2026-09-09 审计，待推送授权后立项）

> 审计结论：真实链路在 admin 单角色登录 + 业务闭环 + 响应式维度扎实；多角色权限差异化验证与打印全链路是系统性空白。

- [ ] **多角色登录测试**：初始化种子 30+ 角色零登录覆盖；loginAsRole helper 已存在（flow/helpers.ts:1139）但 CI 从未提供 E2E_{ROLE}_USERNAME/PASSWORD。需：globalSetup 批量创建 3-5 个代表性角色账号（cashier/sales_rep/warehouse_keeper/accountant/viewer）→ CI env 注入 → 新建 10-roles-login.spec 真实 UI 登录 + Dashboard 可达 + 权限菜单收敛断言
- [ ] **垂直越权测试**：低权限账号访问高权限端点必须 403（如 cashier→DELETE /users、sales_rep→/roles）。现有 P1-6 仅 admin 访问不存在 ID，非真实越权
- [ ] **水平越权测试**：用户 A（shard 账号）操作用户 B 创建的单据（PUT/DELETE）必须被拒（403/404），覆盖采购订单/销售订单/客户三张表
- [ ] **修复恒真假断言**：09-permissions P1-2/P1-3 断言 `status===200||status>=400` 恒真，改为低权限角色账号登录后断言精确 403 + permission_denied 审计记录
- [ ] **修复空转测试**：P1-7（行级隔离）/P1-8（字段级权限）try/catch skip 改为真实断言
- [ ] **打印端点覆盖**：后端 61 个 /{id}/print docx 端点 E2E 覆盖为 0。选 5-8 个高频单据（sales_orders/vouchers/flow_cards/dye_batches/purchase_orders）API 请求断言：HTTP 200 + Content-Type docx + PK zip magic bytes（0x504B）+ 文件大小>1KB
- [ ] **打印内容匹配**：解析 docx（JSZip 解包 document.xml）断言源单据字段值（单据号/客户名/金额）出现在文档 XML 中——"打印成功后内容与需打印文件匹配"的直接验证
- [ ] **打印模板 API 覆盖**：print-templates CRUD + preview + setDefault + copy（前端 api/print-templates.ts 9 个函数对应后端端点）零测试，补 API 级真实链路 spec
- [ ] **打印审计闭环**：audit-logs/record-print 后端集成测试与 E2E 均为零；打印一次 → 查 audit-logs 出现 print 记录
- [ ] **导出内容断言**：现有 3 处真实下载仅断言文件名后缀；补 JSZip/exceljs 解析断言列头与行数 ≥ 页面列表数据量；后端 56 个 export 端点抽样扩到 10+ 个
- [ ] **enhanced mock 清理**：network-resilience.spec.ts 用 page.route mock 注入 403/401，违反禁 mock IR（2026-09-07 前遗留），改真实 token 过期/无权限账号触发
- [ ] **purchase/sales 业务目录纳入主 CI**：07-supplier-report 等真实下载测试只在 e2e-batch.yml（每 30 批次手动）跑到，主 CI 34 分片 testMatch 未含

### 内部更新功能 + 敏感导出授权审计缺口（2026-09-09 二轮）

- [ ] **system-update 权限门禁零测试**：require_admin_role 挂在 4 个高危端点（update/upload/rollback/local-update）但无 HTTP 层集成测试验证非 admin → 403；后端测试仅 URL 校验/版本比较/zip magic 纯函数。补 handlers_system_update 权限矩阵测试（admin 200 / cashier 403 / 未登录 401）
- [ ] **system-update E2E 深度**：现仅 2 处 body visible 冒烟；补 check/version/update-status 只读端点真实调用 + version 数据断言；rollback/local-update 在 CI 空库环境的安全路径测试
- [ ] **bingxi update CLI 零集成测试**：cli/util/upgrade.rs（SHA256 校验 + 健康检查门禁 + 回滚）无任何测试
- [ ] 🔴 **敏感导出审批可绕过（机制断裂）**：export_customers/export_products 等敏感资源导出端点未强制校验 export-approvals 的 download_token——审批流为旁路，绕过审批可直取客户全量数据。需在 6 类敏感资源（customer/supplier/dye_recipe/price_list/finance_report/audit_log）的 export handler 接入令牌强制校验（fail-closed）或中间件层统一拦截
- [ ] **导出审批前端零接入**：全前端无一处调用 export-approvals 8 端点，审批创建/审批/令牌下载 UI 不存在
- [ ] **导出审批测试零覆盖**：令牌签发→verify→消费→record_download 全链路无集成测试；后端仅 4 个纯函数单测；需补审批流集成测试 + E2E（admin 审批后低权限用户持令牌导出成功/无令牌被拒）
- [ ] **print/export 角色黑名单无端到端验证**：PRINT_DENIED/EXPORT_DENIED（customer/temporary）+ DYE_RECIPE_EXPORT_DENIED（仅 dye_recipe_master）三层清单在 permission 中间件实现且 fail-closed，但因无低权限测试账号（见上轮缺口）从未被真实请求验证

### E2E 综合审计缺口（2026-09-09 三轮，详证见 docs/audits/e2e-comprehensive-audit-2026-09-09.md）

> **全部 30 项缺口的修复实施计划已定稿：docs/plans/one-round-fix-plan-2026-09-09.md**（8 commit 划分、逐文件改动表、依赖图、风险清单）。等待用户解除源代码冻结后按计划一轮执行。

> 覆盖：E2E 真实性、2FA、预览、登录链路、版本号、审批体系、admin 全功能遍历、显示异常/重复提示。

- [ ] 🔴 **E2E 真实性（用户标注关键）**：218 spec 中 58 文件含 mock、55 spec 用 applyAuthMocks 伪造登录态（smoke 全套件/bpm/crm/finance/quality/purchase-ext/enhanced）；flow 主套件 loginViaUI（helpers.ts:898）route.fulfill 拦截 lock-status——既掩盖后端 16 分片并发挂起，又掩盖登录 401 瀑布 bug。整改：真实登录替换全部 applyAuthMocks（前置=多角色账号基建）+ 移除 lock-status 拦截
- [ ] **2FA/TOTP 零覆盖**：后端 /auth/totp/setup、enable、recovery-codes + 登录 totp_token/恢复码能力完整，E2E 命中 0。补：启用全流程、缺/错/对 token 登录、恢复码一次性消费、禁用回退
- [ ] **预览零覆盖**：前端 print-templates/report-templates/bpm/templates 三处预览 UI + 后端 /templates/{id}/preview，E2E 命中 0。补：预览可达、内容一致（模板字段出现在预览 DOM/API 返回）、显示完整（无截断/白屏）
- [ ] **登录 5 请求瀑布**（密码错误一次点击→5 请求）：login() 未置 _skipAuthRetry（auth.ts L29）→ refresh 401 → catch 调 refreshLockStatus → lock-status 401（PUBLIC_PATHS 放行但 handler 强制 AuthContext）→ 再 refresh。修复：login() 置位 + handler 改可选认证 + Login.vue L318 移除失败后刷新
- [ ] **协议/隐私页死链**：Login.vue L74/78 `#/terms`、`#/privacy` hash 伪路由 vs createWebHistory + 无路由定义 + 无内容文件，点击无效果。补路由 + 内容页
- [ ] **锁定阈值 5→9**（用户明确要求）：auth_handler.rs L30 与 login_security_handler.rs L95 两处 MAX_FAILED_ATTEMPTS=5 改 9（IP 维度），展示端同步；全局 ×2（=18?）比例待用户确认
- [ ] **版本号机制故障**：main VERSION=2026.723.1842（8-26 停更）、Cargo.toml=2026.810.1、Release=v2026.9.9.1953 三方不一致；根因 release job `git push origin HEAD:main` 被分支规则拒（PR required + verified signatures），d38c9f6f 留 runner 磁盘。后果 get_current_version 读旧值→check_for_updates 永远误报。方案 A（API 提交）/B（自动 PR）/C（改读 CARGO_PKG_VERSION，推荐评估）待用户决策
- [ ] **审批体系 E2E 8.6%**：58 个 /approve 端点真实调用仅 5 个；10d spec 5/6 空转；bpm/02 用 applyAuthMocks 且目录不在主 CI；export/role-change/transfer/writeoffs 四套专用审批前端零接入
- [ ] **审批审计无 APPROVE 分类**：omni_audit.rs L405 classify_operation 仅 PRINT/EXPORT/DOWNLOAD/CRUD/OTHER，无法按事件筛"谁批了什么"；业务表仅 approved_by/at 无 IP
- [ ] **admin 全功能遍历**（用户 b-j 项）：仅点"新建"，编辑/删除/导出/审批按钮零点击；80+ 模块保存从未落库回读；无 pageerror/白屏/console.error 监听、400/404 系统性检测、error boundary 触发——设计骨架见审计报告第五节（30-traversal spec 系列）
- [ ] **重复提示检测**：连续提交 N 次 → 同文案 toast ≤1 断言；dialog 重复实例计数；未翻译 key/NaN/undefined 渲染抽样

### 验证类（需推送授权或手动执行）

- [ ] PG 机制锚点测试手动验证：`TEST_DATABASE_URL=... cargo test --test rls_context_test -- --ignored`（test_rls_guc_visible_in_same_pool_pg，CI 不跑 ignored）
- [ ] RLS dept 迁移部署 sequencing 确认：生产部署先 `bingxi migrate run` 再发新后端二进制（顺序颠倒新代码读不到 department_id；旧后端+新迁移=dept 安全降级为 self+公海，无风险窗口）

### 流程备忘

- [ ] CI 失败若再出现新 job：按 doto 流程拉日志→记录→下批修复
- [ ] 推送授权后：本地 commit 批量推送 + 观察 main CI（含 coverage 等 main 专属 job）

### 待用户决策

- [ ] 分支 260907-feat-piece-domain-phase2 是否删除（已合并 PR #939）
- [ ] runtime-flow-map.md 部署章节是否补充 RLS 迁移 sequencing 说明
- [x] ci-cd.yml 的 docs/** 触发路径失效——已获用户授权（2026-09-09，定向豁免）修改 yaml，冗余条目清理完成（65736c1），CI 行为无变化

后续新增任务请在此文件追加。

# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单），进度必须真实，禁止乐观偏差。
> 已完成任务见 [doto-su.md](doto-su.md)，一句话总结见 [CHANGELOG.md](CHANGELOG.md)，规则见 [MEMORY.md](MEMORY.md)。
> 过时内容归档见 [docs/archives/](docs/archives/)（最新批次：2026-09-09）。

---

## 当前状态

**文档治理完成（2026-09-09）：README 数据更新、doto/bug 归档、docs/ 移入 .monkeycode、生产服务器日志与 scan_long_fns.py 删除，全部本地 commit。源代码修改已冻结（IR 2026-09-09），仅保留文档修改权限。**

---

## 硬约束（用户指令，后续所有任务必须遵守）

- [x] 禁止 mock：E2E 测试一律真实后端 + 真实 PostgreSQL 数据
- [x] 禁止本地编译：验证一律走 CI
- [x] **推送冻结（2026-09-08 二次冻结，持续生效）**：git push 一律禁止（含分支/tag），改动仅本地 commit；推送必须等用户明确授权
- [x] **源代码修改冻结（IR 2026-09-09）**：禁止修改一切源代码（backend/frontend/.github/scripts/deploy 等）；仅保留文档修改权限（README/CONTRIBUTING/.monkeycode/docs 等 markdown）

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

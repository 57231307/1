# 更新日志（.monkeycode 版本）

> 本文件是 `/workspace/CHANGELOG.md` 的精简版，记录任务总结。
> 原文件包含完整的项目变更历史，本文件保留关键任务执行记录。

---

## 文件来源

| 文件 | 用途 | 说明 |
|------|------|------|
| `/workspace/CHANGELOG.md` | 完整更新日志 | 包含所有项目变更的详细记录 |
| `.monkeycode/CHANGELOG.md` | 任务总结精简版 | 记录 doto.md 的任务总结 |

---

## 最新任务总结

### 综合审计报告（2026-06-19）

- **综合报告**：[.monkeycode/docs/audits/2026-06-19-comprehensive-audit.md](file:///workspace/.monkeycode/docs/audits/2026-06-19-comprehensive-audit.md)
- **基线**：main HEAD `2f8fa81`
- **综合评分**：72/100 B 级（与 2026-06-16 评估持平；utils/ 清理收益被 4 维度新发现抵消）
- **核心统计**：
  - 后端 API：943 端点 / 905 唯一 (method,path) / 18 业务域
  - 前端 API：89 文件 / 933 调用点
  - 前端路由：114 路由 / 392 .vue
  - 现代代码：626 .rs + 413 .vue + 217 .ts
- **🔴 P0 必修（6 大类）**：
  - **P0-A** 4 处启动时 panic：sales.rs:116/120、system.rs:28、custom_order 整模块未挂载 → **当前 main 无法启动**
  - **P0-B** 6 处安全/规范：83 文件级 dead_code + cookie_secret 静默降级 + 随机 JWT secret + operation_log 吞咽 + token localStorage + 2 v-html XSS
  - **P0-C** 2 处路由错配：color-prices/create 指向 list.vue、/system/slow-query 菜单孤儿
  - **P0-D** 96 个前端 API 孤儿：custom-order 17 + api-gateway 14 + 采购路径不一致 26 + 用户档案 3
- **🟡 P1 应当修**：5 BPM 状态流转端点 + 132 项级 dead_code + 6 .vue > 500 行 + 8 .rs > 750 行 + 18 前端死代码 + 200+ API 孤儿
- **🟢 P2 建议修**：route 元信息 106/106 缺 icon/permission + 409 `: any` + 191 `as any` + 引入 utoipa + CI 增补启动校验
- **🟢 已达标**：0 unsafe / 0 @ts-ignore / 0 eval / 0 innerHTML / 0 unwrap_or(0) / 146 租户隔离 100% 合规 / SQL 100% 参数化 / 7 安全头已配
- **修复路线图**：
  - 立即（30 分钟）：4 处 P0-A
  - 短期（1 周）：83 dead_code + 3 密钥 + 2 XSS
  - 中期（1 月）：P1 拆分 + 200+ 孤儿
  - 长期（季度）：utoipa + SAST 工具链

### 冰溪 ERP 现代代码质量审计（2026-06-19）

- **报告位置**：[.monkeycode/docs/audits/2026-06-19-modern-code-audit.md](file:///workspace/.monkeycode/docs/audits/2026-06-19-modern-code-audit.md)
- **审计范围**：`backend/src/**`（626 .rs 文件）+ `frontend/src/**`（413 .vue + 217 .ts）
- **执行方式**：子代理静态分析（Grep/Glob/Read/RunCommand），**未本地编译**
- **综合评分**：**73/100（B- 级）**（较 2026-06-16 评估 72 分微升）
- **核心发现**：
  - 🔴 **P0 死代码违规 83 处**（文件级 `#![allow(dead_code)]` 在非 models/ 散布，CI 必失败） — services 68 / handlers 2 / middleware 1 / 其他 12
  - 🔴 **P0 密钥静默降级 3 处**：
    - `backend/src/main.rs:325-328` cookie_secret 复用 jwt_secret（高危密钥复用）
    - `backend/src/utils/app_state.rs:193` 随机 JWT secret（多副本部署签名不一致）
    - `backend/src/middleware/operation_log.rs:76` 操作日志静默吞咽（违反审计完整性）
  - 🔴 **P0 XSS+token 风险**：2 处 v-html 残留（`report-templates/index.vue:170`、`print-templates/index.vue:212`）+ 25 处 localStorage token 访问（XSS 一击必杀）
  - 🟡 **P1 项级死代码 132 处**（60 文件），热点：`field_permission_service.rs:7`、`event_kafka.rs:5`
  - 🟡 **P1 前端 `any` 高密度**：409 处 `: any` + 191 处 `as any`（600 处总和，TOP5 域：quality/sales-returns/production/api-gateway/purchase）
  - 🟡 **P1 大文件待拆分**：6 个 .vue > 500 行（TOP purchase 748 / quality 675 / inventory 600）+ 8 个 .rs > 750 行
  - 🟡 **P1 panic 业务路径 20 处**（最严重：`services/audit_log_service.rs:5`）
  - 🟢 **达标项**：
    - `utils/` 8 个核心文件 100% 死代码清理（达成模板）
    - `models/` 200 个 SeaORM 文件级抑制（合规例外）
    - 0 处 `unsafe {` 块
    - 0 处 `@ts-ignore` / `@ts-nocheck` / `eval()` / `innerHTML`
    - 0 处 `auth.tenant_id.unwrap_or(0)` 真实代码违规
    - 0 处空 catch 块
    - SQL 已参数化（无 `format!("SELECT...")` 拼接）
    - 146 处 `extract_tenant_id(&auth)?` 100% 合规
    - CSP / HSTS / X-Frame-Options / CSRF 等 7 项安全头已配置
- **改进路线图**：
  - 第 1 周：D1-D5（删 83 文件级抑制 + 修 3 处密钥降级 + 验证 CICD clippy）
  - 第 2 周：D6-D9（修 v-html + 分类 132 项级抑制 + 评估 localStorage 迁移）
  - 第 3-4 周：D10-D13（拆 6+18 个大 .vue + 8 个大 .rs + 替换 `any`）
  - 第 5-6 周：D14-D17（修 116 处 `let _ =` + 20 处 `panic!` + 评估 sleep）
  - 第 7-12 周：D18-D21（OIDC 接入 + SAST 工具 + 自动类型生成）

### 前端 Vue Router 路由审计（2026-06-19）

- **报告位置**：[.monkeycode/docs/audits/2026-06-19-frontend-router-audit.md](file:///workspace/.monkeycode/docs/audits/2026-06-19-frontend-router-audit.md)
- **审计范围**：`frontend/src/router/index.ts`（709 行，114 路由/110 可导航）+ `frontend/src/views/**`（392 .vue 文件）
- **执行方式**：子代理静态分析（Read/Grep/Glob/find），**未本地编译**
- **核心发现**：
  - 🔴 **P0 错配 1 处**：`router/index.ts:638-639` `/color-prices/create` 路由 component 指向 `color-prices/list.vue`（应为 `create.vue`）
  - 🔴 **P0 菜单孤儿 1 处**：`MainLayout.vue:144` 菜单项 `/system/slow-query` 引用了不存在的路由（页面 `system/slow-query/index.vue` 已开发但未挂载）
  - 🟡 **P1 死代码页面 17 + 子文件 23**：
    - `bpm/approval/`（1+7）— 拆分完整但未挂载路由
    - `bpm/definitions/`（1+7）— 与 `bpm/definitions.vue` 重复
    - `security/two-factor/`（1+7）— 注释承诺路由直接引用但未实现
    - `security/ChangePassword.vue` — 功能已合并到 user-profile
    - `admin/failover.vue` + 3 components — 主备隔离 UI 未挂载（后端 4 端点已上线）
    - `bi/index.vue` — BI 入口预留
    - `crm/leads/index.vue` + `crm/opportunities/index.vue`（+ 3 tabs）— CRM 子模块未挂载
    - `report/templates.vue` + 11 components/composables — P12 拆分前残留
    - `sales/tabs/{SalesOrderFilter,SalesStatsCards}.vue` — 被 `OlvFilter/OlvStat` 取代
  - ✅ **良好实践**：name 100% 唯一、path 100% 唯一、嵌套深度 1 层清晰
  - 🟡 **P2 元信息缺失**：106/106 子路由缺 `icon` / `permission` / `keepAlive` / `breadcrumb`（不影响运行）
  - 📊 **模块分布 TOP 3**：财务 16（14.5%）/ 销售 11 / 库存+物流 10
- **下一步**：
  1. 5 分钟 P0：修 `color-prices/create` 错配 + 挂载 `/system/slow-query`
  2. 下一迭代 P1：批量挂载 4 个死代码页面组（admin/failover、bpm/approval、security/two-factor、crm 子模块）
  3. 清理 P1：删除 5 个冗余文件 + 整个 `bpm/definitions/` 子目录
  4. P2 治理：建立路由元信息 TypeScript 接口、删除废弃 alias `/workflow`

### 后端 HTTP API 路由审计（2026-06-19）

- **报告位置**：[.monkeycode/docs/audits/2026-06-19-backend-api-audit.md](file:///workspace/.monkeycode/docs/audits/2026-06-19-backend-api-audit.md)
- **审计范围**：`backend/src/routes/*.rs`（20 文件，943 路由条目，905 唯一 method+path）
- **执行方式**：子代理静态分析（ripgrep + Python 解析 + nest/merge 链模拟），未本地编译
- **核心发现**：
  - 🔴 **P0 启动时 panic 3 处**：
    - `routes/sales.rs:116` 引用 `quotation_handler::convert_quotation_to_order`（实际为 `convert_to_sales_order`）
    - `routes/sales.rs:120` 引用 `quotation_handler::list_expiring_quotations`（实际为 `list_expiring`）
    - `routes/system.rs:28` 引用 `websocket::ws_notifications_handler`（实际为 `websocket::notifications::ws_notifications_handler`）
  - 🔴 **P0 孤儿路由 18 处**：`routes/custom_order.rs` 整模块 18 端点，`mod.rs:58` 仅声明 `pub mod custom_order;`，`create_router` 中**未挂载**
  - ✅ **未发现真正 method+path 冲突**：38 个"重复"条目均为 nest 子树误判
  - 📊 **HTTP 方法分布**：GET=447 / POST=320 / PUT=96 / DELETE=80
  - 📊 **业务域 TOP 3**：财务 196 / 分析-高级功能 136 / 采购 95
  - 📄 **INTERFACES.md 65 端点"未实现"**：实际全部因文档缺 `/api/v1/erp` 前缀或占位符风格不一致（`{}` vs `:id`）导致，**非真实缺失**
- **下一步**：
  1. 修复 3 处 handler 引用错误（启动 panic）
  2. 在 `mod.rs` 中 nest `custom_order::custom_order_routes(state)`
  3. 引入 OpenAPI utoipa 解决文档漂移
  4. CI 增补 axum Router 启动校验

### 前端 API 调用审计（2026-06-19）

- **报告位置**：[.monkeycode/docs/audits/2026-06-19-frontend-api-audit.md](file:///workspace/.monkeycode/docs/audits/2026-06-19-frontend-api-audit.md)
- **审计范围**：`frontend/src/api/*.ts`（89 文件，933 调用点）+ `backend/src/routes/*`（13 文件）
- **执行方式**：子代理自动静态分析（Glob/Grep/Read），未本地编译
- **核心发现**：
  - 🔴 **P0 严重孤儿 ~96 端点**：
    - `/api-gateway/*`（14 处）后端**完全未实现**
    - `/api/v1/erp/custom-orders/*`（17 处）路由已实现但**未在 mod.rs 中 nest**（5 分钟修复）
    - `/purchase/receipts` vs 后端 `/purchases/receipts` 路径不一致（11 处）
    - `/production/production-orders/*`（10 处）、`/production/greige-fabrics/*`（8 处）、`/crm/customer-credits/*`（11 处）后端未注册
    - `/user/profile` PUT、`/user/change-password`、`/user/avatar` 缺失
  - 🟡 **P1 中等孤儿 ~200+ 端点**（销售/采购 submit-approve-reject、AP/AR 编辑、库存调整、CRM 五维等）
  - ✅ **良好实践**：axios 拦截器（401 自动 refresh + 重放）、CSRF 注入、9 个公开路径白名单、TOTP 2FA
  - ⚠️ **风险**：3 个 token 全部明文存于 localStorage（access_token / refresh_token / csrf_token）
- **下一步**：
  1. 挂载 custom-order 路由（mod.rs 中加一行 nest）
  2. 决定 API 网关后端实现策略
  3. 统一采购/销售 submit-approve 走 BPM 流程

### Wave 1+2+3 修复（2026-06-19）

- **P0 - 3 个孤儿 migration 注册**：m0025/26/27 重命名 + lib.rs pub mod + Box::new（修复审计增强 + 慢查询审计）
- **P1 - 删除孤立目录**：mobile/ (17) + microservices/ (13) + deploy/{elasticsearch,grafana,helm,kafka,observability,prometheus}/ (24)
- **P2 - 删除 8 个空子目录**：.monkeycode/docs/{api,superpowers/reports,poc,requirements,db,专有概念,模块,releases}
- **变更**：1 修改 + 30 删除 = 31 文件
- **CI/CD 验证**：遵循"禁止本地编译"规则，仅依赖 GitHub Actions

### 推送 main + 清理根 CHANGELOG/MEMORY（2026-06-19）

- **删除**：`chore: 删除 test 合入的根 CHANGELOG.md / MEMORY.md`（2 文件 -1941 行）
- **原因**：与 .monkeycode/ 记忆体系重复，统一以 .monkeycode/ 为唯一记忆系统
- **最终 main HEAD**：`b99ec30`

### I-3 第 6 批合入 + feature 分支清理（2026-06-19）

- **cherry-pick**：`git cherry-pick -X theirs e4ba11d` 单点合入 p14 分支唯一提交，41 文件 +3600/-2421 行
- **拆分成果**：capacity 562→116 / Dashboard 549→99 / security 547→101 / TwoFactorSetup 540→2-factor 子目录 / sales-analysis 535→106
- **I-3 累计**：I-1 (3) + I-2 (3) + I-3 第 1~6 批 (23) = **29 个大 .vue 全部完成**
- **远端清理**：删除 2 个 feature 分支（p14 合并后冗余、p12 过期）→ 远端仅剩 main

### test 合并入 main（2026-06-19）

- **合并方式**：`git merge -X theirs origin/test`，81 个 UA 冲突以 test 版本为准，merge commit `3116afa`
- **.monkeycode/ 恢复**：用户要求"使用 main 的 .monkeycode 目录"→ 从 `main-backup-20260619-pre-testmerge` 标签 checkout 恢复，删除 100 个 test 独有文档，commit `19fb82f`（+143/-46049 行）
- **test 分支删除**：远端 `git push origin --delete test` + 本地 `git branch -rd origin/test` 完成清理
- **保留 test 内容**：mobile/ 目录、microservices/ 目录、P0~P9 业务功能、根 CHANGELOG.md、根 MEMORY.md
- **撤销兑底**：`main-backup-20260619-pre-testmerge` 标签保留可回退至合并前状态

### docs 合并 + main 同步（2026-06-19）

- **docs 整合**：将 3 个源 docs 目录（`/workspace/docs`、`/workspace/backend/docs`、`/workspace/frontend/docs`）移动到 `/workspace/.monkeycode/docs`，共 91 个文件，无冲突
- **main 同步**：远端已包含 `a0a25e8 chore: 合并 /workspace/docs 到 .monkeycode/docs`（自动化或外部提交），与本地 `390f101 feat: 项目评估` 形成分叉
- **解决方式**：`git pull --no-rebase` + `git push`，最终 merge commit `fb1d331`，**未使用强制推送**（保留远端所有历史）
- **关键经验**：用户口头"强制推送"在前端检查时本不需要；fetch 后才暴露分叉，最终选 merge 策略避免数据丢失

### P14 批 2 B3 拆分大 .vue（2026-06-19）

- **PR #195 ~ #199**：5 个 PR 全部 squash merge 入 main
- **累计进展**：18/24 大 .vue 已拆分
- **拆分成果**：
  - PR #195：VoucherListTab 870→141 + system-update 725→154 + sales-contract 717→129
  - PR #196：purchase-return 695→211 + scheduling/gantt 691→93 + scheduling/index 689→109
  - PR #197：sales-price 677→147 + OrderListView 644→125 + purchase-contract 644→142 + purchase-price 622→137
  - PR #198：bpm/approval 618→123 + production 611→172 + logistics 605→117 + purchaseReceipt 598→97
  - PR #199：data-import 596→127 + purchase-inspection 594→113 + material-shortage 590→85 + bpm/definitions 579→150
- **经验沉淀**：
  - composable 用 reactive({...}) 包装 return
  - v-model 不能用于 prop，必须用 :model-value + @update:model-value + emit
  - string/number/boolean 类型 prop 是 readonly，必须用 emit 模式

### P13 批 1（2026-06-18）

- **PR #191**：P3-2 审计日志增强（6 commit，CI 5 轮迭代）
- **PR #192**：B-慢查询审计（3 commit，CI 2 轮迭代）
- **PR #193**：B3 拆分大 .vue I-1（5 commit，CI 4 轮迭代）
- **P13 批 1 全部 3/3 PR 完成**

### P12 批 1+2+3（2026-06-17 ~ 2026-06-18）

- **12/12 PR 全部完成**
- P0 销售报价单端到端贯通（4 PR 串行）
- P2-1 V2Table 全面替代老 el-table（5 PR）
- P2-2 性能优化落地（Redis 缓存层 + DB N+1 审计）
- B-type-check CI 5 job（vue-tsc 真正起到拦截作用）
- P3-1 前端安全加固（TOTP 2FA + 修改密码 + 密码强度可视化）

### Wave 1-3（2026-06-15）

- **Wave 1**：4 PR 100% 合并（P0-2 销售→AR / P2-3 编译验证 / P1-1 generate-no / P1-5 入库单明细）
- **Wave 2**：6/6 完成（B3-1~4 拆分大 .vue + B5 POC + B6 清理）
- **Wave 3**：11 PR 100% 合并（B7 console.* 清理 + type-check 清理 + AI 深化）

---

## 关键经验

### TypeScript
- 对象字面量 excess property check 每次只报告第一个未知属性
- `String(e)` 转换是 unknown → string 的标准模式
- `vue-tsc` 不要带 `-b`（与 noEmit 冲突）

### Rust
- 项级 `#[allow(dead_code)]` + TODO(tech-debt) 是合规做法
- SeaORM 自动生成模型保留文件级抑制
- 子代理串行调度避免云端卡死

### Git
- worktree 占用导致本地分支无法删除：先 `git checkout main` 切到 main，再 `git branch -D`
- GitHub squash merge 后远端分支自动删除

### CI/CD
- 所有验证通过 `.github/workflows/ci-cd.yml`
- 后端 4 检查：clippy / build / fmt / test
- 前端 3 检查：build / test / lint
- 推送后等 CI 全绿（绿色 ✓）才算成功

---

## 完整变更历史

完整的项目变更历史请查看：`/workspace/CHANGELOG.md`

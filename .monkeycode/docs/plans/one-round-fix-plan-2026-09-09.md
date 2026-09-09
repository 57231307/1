# 一轮修复总计划（2026-09-09）

> 依据：docs/audits/e2e-comprehensive-audit-2026-09-09.md + doto.md 全部 30 项缺口。
> 前提：用户解除源代码修改冻结（本计划即为解冻后的一轮执行清单）；推送仍冻结，全程本地 commit。
> 每个任务给出：改动文件:行号、现状、精确改法、依赖、验证。执行顺序即文中批次顺序。

---

## 0. 前置决策（默认值已选，用户可否决）

| 决策点 | 默认方案 | 备选 |
|--------|---------|------|
| 锁定阈值全局比例 | 维持 ×2（IP 9 次 → 用户全局 18 次） | 用户确认后可改 |
| 版本号机制 | 方案 C：get_current_version 改编译期 env!；Release job 移除 VERSION 文件提交推送（tag 保留） | A（API 提交）/ B（自动 PR） |
| 敏感导出 fail-closed | 直接强制，无过渡开关 | 加 env 开关灰度（不推荐，留缺口） |
| 业务目录纳入主 CI | 本轮仅纳入 purchase/sales（用户点名）；quality/finance/crm/bpm/purchase-ext 去 mock 后观察一轮再纳入 | 全部纳入（CI 时长风险） |
| 2FA TOTP 码生成 | E2E 内置 RFC 6238 生成器（crypto HMAC-SHA1，免装包） | npm 装 otpauth |
| 文档/依赖新增 | JSZip（docx/xlsx 解析）加入 frontend devDependencies | exceljs（xlsx 列断言用轻量 XML 解析替代，暂不引入） |

---

## 1. 批次依赖图

```
P1 后端修复 ──┬─→ P3 E2E 基建 ─→ P5 E2E 新增/改造 ─→ P7 CI 验证（需推送授权）
P2 前端修复 ──┘        ↑
P4 后端测试（可与 P3 并行，依赖 P1）
P6 CI 配置（依赖 P1 方案 C 定稿）
```

关键顺序约束：
- `loginViaUI` 的 lock-status 拦截移除（P2.5）必须在后端 lock-status 改造（P1.2）之后同轮完成；
- 导出内容断言类 E2E（P5.3）必须在敏感导出 fail-closed（P1.1）之后，走审批链取令牌；
- 55 spec 去 mock（P2.4）落地后需先跑 smoke 分片验证，再执行 P5。

---

## 2. P1 后端修复（5 个 commit 候选 → 合并为 2 个 commit）

### P1.1 敏感导出审批令牌强制校验（fail-closed）【P0 安全】

**改动文件：**
1. `backend/src/services/export_approval_service.rs`
   - 新增公共方法 `pub async fn enforce_export_download(&self, token: Option<&str>, expected_resource: &str) -> Result<Model, AppError>`：
     - token 为 None/空 → `Err(AppError::permission_denied("敏感资源导出需先获得导出审批令牌"))`（fail-closed）；
     - 调用现有 `verify_download_token(token)`（L365，已校验存在性 + status==Approved + 有效期）；
     - 校验返回 Model 的 `resource_type == expected_resource`（防令牌跨资源复用）；
     - 通过后返回 Model 供 handler 调 `record_download`（L407）。
2. 六类敏感资源 handler（在函数体第一行业务逻辑前插入校验）：
   - `customer_handler.rs:556 export_customers`：Query 结构 `CustomerListQuery` 增加 `download_token: Option<String>`；函数开头 `let approval = state.export_approval_service().enforce_export_download(query.download_token.as_deref(), "customer").await?;` 导出成功后 `record_download(approval.id, file_path, checksum)`；
   - `product_handler.rs:491 export_products`（resource="product"，同模式）；
   - `supplier_handler.rs:347 export_suppliers`（resource="supplier"）；
   - `dye_recipe_handler.rs:199 export_dye_recipes`（resource="dye_recipe"）；
   - `audit_log_handler.rs:355 export_audit_logs`（resource="audit_log"；`audit_enhanced_handler.rs:130` 同名端点同步改）；
   - `finance_report_handler.rs` 6 个：L242 trial_balance / L289 balance_sheet / L342 income_statement / L400 cash_flow / L465 general_ledger / L523 subsidiary_ledger（resource="finance_report"，6 处同一模式）；
   - price_list 类：执行时 `grep -rn "export" backend/src/handlers/{purchase_price,sales_price,color_price}_handler.rs` 定位后按同模式加（is_sensitive 的 "price_list" 映射到哪个 handler 以 models/export_approval_request.rs L194 注释为准）。
3. 路由层无需改（token 走 Query 参数）。

**执行时检查点：**
- `grep -rn "export_customers\|export_products" backend/tests/` ——现有后端测试若直接调用会 403，需同步补 token 或走 service 层测试；
- 前端现有导出按钮（customer/product 列表页）将 403 —— P2.3 的 export-approvals UI 同轮落地承接；`22-crm-full.spec.ts:55 verifyEndpointHealthy('/crm/customers/export')` 改走审批链（P5.6）。

**验证**：cargo test（现有 + P4.3 新增）；CI。

### P1.2 lock-status 认证矛盾修复（登录瀑布后端侧）

**改动文件：`backend/src/handlers/login_security_handler.rs:154`**
- 签名 `auth: AuthContext` → `auth: OptionalAuthContext`（import 加 `crate::middleware::auth_context::OptionalAuthContext`，init_handler.rs:5 已有先例）；
- 越权防护逻辑改造：
  - `auth` 为 None（登录页匿名预检场景）：放行，仅返回该 username 的 IP/用户维度锁定状态（公开语义，泄露面仅失败计数）；
  - `auth` 为 Some：保持现有逻辑（非 admin 只能查自己）；
- 路由 PUBLIC_PATHS（public_routes.rs L6 含 /auth/lock-status）不动。

**验证**：P4.1 权限矩阵测试新增匿名查询 200 断言；P5.1 登录瀑布 E2E。

### P1.3 锁定阈值 5 → 9

**改动文件（2 处常量 + 全局比例维持 ×2=18）：**
- `backend/src/handlers/auth_handler.rs:30`：`MAX_FAILED_ATTEMPTS: i32 = 5` → `9`；
- `backend/src/handlers/login_security_handler.rs:95`：同名常量 `5` → `9`；
- 全局锁定 `MAX_FAILED_ATTEMPTS * 2`（auth_handler.rs L31 附近）自动变 18，逻辑不动；
- 检查两文件内注释/测试中的硬编码 5（`grep -n "= 5\|== 5\| attempts" auth_handler.rs login_security_handler.rs`）同步更新。

**验证**：后端现有 lockout 测试若有阈值断言需同步（执行时 grep tests）。

### P1.4 审计 APPROVE 分类

**改动文件：`backend/src/middleware/omni_audit.rs:405 classify_operation`**
- 在 PRINT 检查（L411）之前插入最高优先级分支：
```rust
// APPROVE：审批/驳回/提交动作（路径末段或含 /approve、/reject）
if last_segment.contains("approve") || last_segment == "reject" || last_segment == "submit" {
    return "APPROVE".to_string();
}
```
- 覆盖面：58 个 /approve 端点 + reject/submit + writeoffs/transfer/role-change 的 approve 子路径；
- 风险检查：`grep -rn '"APPROVE"' backend/ frontend/src` 确认 event_type 无既有同名字段冲突；audit-logs 前端筛选下拉（执行时定位 event_type 选项所在 vue）补 APPROVE 选项（1 行）。

**验证**：P4.2 集成测试断言 approve 请求后 omni_audit_logs 出现 event_type='APPROVE'。

### P1.5 版本号机制修复（方案 C）

**改动文件：**
1. `backend/src/services/system_update_ops/status.rs:122 get_current_version`：
   - 实现改为 `env!("CARGO_PKG_VERSION").to_string()`（编译期真实版本，消除对部署目录 VERSION 文件的依赖）；
   - 保留函数签名；VERSION 文件继续作为更新包结构文件存在（apply.rs L156/174/226 的包校验逻辑全部不动）；
   - 已确认 backend/tests/ 无 get_current_version 直接依赖（grep 无命中）。
2. `.github/workflows/ci-cd.yml` Release job（L2160-2230）：
   - 删除"生成本地 VERSION commit → git push origin HEAD:main"段（即产生 d38c9f6f 且被分支规则拒绝的整段）；
   - VERSION 文件仍写入 release 压缩包（部署目录元数据保留，向后兼容）；
   - tag/Release 发布逻辑不动。
   - 注意：ci-cd.yml 修改已获过一次定向豁免（65736c1 用毕），本次属新增授权范围，计划默认包含（方案 C 已在决策表列出）。

**验证**：CI 构建后 `/api/v1/erp/system-update/version` 返回 2026.9.x 与 Release tag 一致；check_for_updates 不再误报。

---

## 3. P2 前端修复

### P2.1 登录 5 请求瀑布（前端侧，3 文件）

1. `frontend/src/api/auth.ts:24 login()`：函数体首行加
   `const cfg = { _skipAuthRetry: true } as Parameters<typeof request.post>[2];`，post 第三参传 cfg（与 L58 refreshToken 同模式，附同款注释：登录失败 401 不应触发 refresh 重试）。
2. `frontend/src/views/Login.vue:318`：catch 内删除 `refreshLockStatus();` 调用（失败提示保留）。
3. lock-status 匿名调用：Login.vue 的 `refreshLockStatus`（blur 预检 L54 + 失败后）在 P1.2 落地后不再 401，调用本身保留（预检是有意功能）。

### P2.2 协议/隐私页（4 文件）

1. `frontend/src/router/index.ts`：新增两条公开路由（login 同级，不在 layout 下）：
   `{ path: '/terms', name: 'Terms', component: () => import('@/views/legal/TermsView.vue') }` 与 `/privacy`；
2. 新建 `frontend/src/views/legal/TermsView.vue`、`PrivacyView.vue`：简单静态内容页（标题 + 条款正文 + 返回登录链接）；正文占位由用户后续提供法务文本，本轮给结构化中文模板（服务说明/账号责任/数据权属/免责）；
3. `frontend/src/views/Login.vue:74/78`：`<a href="#/terms">` → `<router-link to="/terms">`（privacy 同理），样式类保持。

### P2.3 导出审批前端接入（最小闭环，2 新文件 + 1 路由）

1. 新建 `frontend/src/api/export-approvals.ts`：封装 8 端点（create/list/approve/reject/pending/detail/download-by-token/cancel，执行时以后端 routes 里 export-approvals 路由签名为准）；
2. 新建 `frontend/src/views/system/ExportApprovals.vue`：列表（状态筛选）+ 审批/驳回操作 + 令牌展示/复制（5 分钟有效期提示）；
3. 路由 + system 菜单入口注册；
4. 客户/产品列表页导出按钮改造：点击后弹出"申请导出审批"对话框（调 create），提示等待审批——完整 UI 流程本轮做 export-approvals 一套；role-change/writeoffs/transfer 审批 UI 立项下轮（doto 保留）。

### P2.4 E2E 去 mock（55 spec 零改动方案，核心 2 文件）

**策略：保持 `applyAuthMocks` 函数名与签名不变，实现从 mock 换为真实登录。**

1. `frontend/e2e/fixtures/auth.ts:135 applyAuthMocks`：
   - 实现改为：API 登录（`request.newContext` POST `${API_PREFIX}/auth/login`，账号 `process.env.TEST_USERNAME/TEST_PASSWORD`——与 global-setup 分片账号一致，根除 CSRF 互踢）→ `context.addCookies(登录 Set-Cookie 全量)`；
   - 删除内部 `injectAuthToken/mockAuthMe/mockInitStatus` 调用（三个 mock 函数体保留但不再被引用，后续清理）；
   - 登录失败直接 throw（与 global-setup 同风格）。
2. `frontend/e2e/smoke/_helpers.ts:11`：re-export 不变，55 个 spec 的 `import { applyAuthMocks } from '../smoke/_helpers'` 与调用点全部无需改动。
3. `frontend/e2e/flow/helpers.ts:898`：删除 loginViaUI 中 lock-status 拦截段（P1.2 已修复后端矛盾；16 分片并发挂起若复现，属后端性能问题另立项）。
4. `frontend/e2e/enhanced/network-resilience.spec.ts`：page.route 注入的 401/403 改为真实触发——过期 token 场景改为：登录后手工清除 access_token cookie 再请求；403 场景改用 viewer 角色账号（依赖 P3.1 种子账号）。
5. `frontend/e2e/bpm/02-approval.spec.ts`：applyAuthMocks 自动被 P2.4.1 覆盖为真实登录，无需单独改。

**验证**：本地跑 smoke 1 个分片 + flow 1 个分片（真实后端 docker-compose 已有启动方式，执行时用 background terminal 起 backend+frontend）。

### P2.5 前端 2FA 无需改

TwoFactorSetup.vue + api（setupTotp/enableTotp/generateRecoveryCodes）+ 路由（router L666）均已存在，本轮仅补 E2E（P5.4）。

---

## 4. P4 后端测试（3 个新文件，放 backend/tests/）

1. `handlers_system_update_authz_test.rs`：权限矩阵——admin 调 update/upload/rollback/local-update 200（或业务码）、cashier 角色用户 403、未登录 401；参考现有 handlers 测试的 mock AppState 模式（执行时参考 backend/tests/ 现有 handler 测试基建）。
2. `omni_audit_approve_test.rs`：构造 POST /approve 路径请求过中间件断言 event_type='APPROVE'；print/export/download 回归断言分类不变。
3. `export_approval_enforce_test.rs`：六类 handler 中抽 customer/product/dye_recipe 三类代表——无 token 403、无效 token 403、他人 token 403（resource_type 不匹配）、有效 approved token 200 且 record_download 落库。
4. lockout 阈值断言更新：执行时 `grep -rn "MAX_FAILED_ATTEMPTS\|attempts.*5" backend/tests backend/src/handlers/*.rs 的测试段`，9 同步。

---

## 5. P3 E2E 基建

### P3.1 种子角色账号（global-setup.ts 扩展）

- `frontend/e2e/global-setup.ts`：新增 `ensureRoleUsers()`——用 e2e_admin 的 API token（复用现有登录 context）调 POST /users + 角色分配端点（执行时以 /users 路由签名为准；若 API 创建被禁则按 ensureShardUserViaUI 的 UI 模式，成本 5 角色 × UI 流程）；
- 角色 5 个：cashier / sales_rep / warehouse_keeper / accountant / viewer（后端 seed 角色名执行时以 roles 表 seed 文件为准 `grep -rn "cashier\|sales_rep" backend/src/**/seed*` 或迁移）；
- 凭证写入 `e2e/.auth/role-credentials.json` + CI env（E2E_CASHIER_USERNAME/... 与 loginAsRole 约定对齐）；
- 幂等：已存在（409/唯一约束）视为成功。

### P3.2 E2E 公共断言库（flow/helpers.ts 追加）

- `trackPageHealth(page)`：注册 pageerror/console.error/response 监听，返回 collector；
- `assertPageHealthy(collector, { allowConsoleWarn })`：断言零 pageerror、主容器非白屏（innerText 长度阈值）、响应无 5xx（400/404 白名单可配）；
- `expectSingleToast(page, textPattern)`：同一文案 toast 实例计数 ≤1；
- `TOTP.generate(secret)`：crypto HMAC-SHA1 RFC 6238 实现（~25 行，30s 步长 6 位）。

### P3.3 playwright.config.ts

- 主 project（L85）testMatch 扩为含 `purchase|sales` 目录：`/(flow|smoke|enhanced|purchase|sales)\/.*\.spec\.ts`（两处 L85/L95 同步）；
- 其余业务目录维持 e2e-batch.yml 路径（观察一轮后再纳入）。

---

## 6. P5 E2E 新增/改造（13 个新 spec + 3 个改造）

| # | spec 文件（frontend/e2e/ 下） | 内容 | 依赖 |
|---|------------------------------|------|------|
| 5.1 | flow/31-login-waterfall.spec.ts | 错密码一次点击：断言请求总数 ≤2（login + lock-status 200）、无 /auth/refresh 调用；成功路径 1 请求 | P1.2/P2.1 |
| 5.2 | flow/32-roles-login.spec.ts | 5 角色真实 UI 登录 + Dashboard 可达 + 侧边栏菜单项随角色收敛（数量/关键项断言） | P3.1 |
| 5.3 | flow/33-vertical-privilege.spec.ts | viewer/cashier 调高权限端点（DELETE /users 等 6 个）断言 403 + 审计记录；修复 09-permissions P1-2/P1-3 恒真断言（改精确 403）与 P1-7/P1-8 空转（真实断言） | P3.1 |
| 5.4 | flow/34-horizontal-privilege.spec.ts | 用户 A（分片账号）PUT/DELETE 用户 B 创建的采购/销售/客户单据 → 403/404 | P3.1 |
| 5.5 | flow/35-2fa-totp.spec.ts | setup 拿 secret → TOTP 生成 → enable → 退出后登录带 token 成功、错 token 拒绝 → recovery-codes 一次性消费 → 禁用回退单因素 | P3.2 |
| 5.6 | flow/36-preview.spec.ts | print-templates 预览（字段出现在 DOM）、report-templates 预览、BPM /templates/{id}/preview API 内容断言、预览容器无截断 | P3.2 |
| 5.7 | flow/37-print-endpoints.spec.ts | 5 个高频单据 print API：200 + docx Content-Type + PK zip magic + >1KB；JSZip 解包 document.xml 断言单据号/客户名字段 | P3.2（devDeps 加 jszip） |
| 5.8 | flow/38-print-templates-api.spec.ts | print-templates CRUD + preview + setDefault + copy 全 API 链路（api/print-templates.ts 9 函数对应） | - |
| 5.9 | flow/39-export-approval.spec.ts | 敏感导出全链路：无 token 403 → 申请 → admin 审批 → 持 token 导出 200 → 令牌二次消费拒；改造 22-crm-full:55 健康检查走审批链 | P1.1/P2.3 |
| 5.10 | flow/40-system-update-authz.spec.ts | check/version/status 真实调用 + 数据断言；非 admin 403（用 viewer）；rollback/local-update 在 CI 空库环境安全路径 | P3.1 |
| 5.11 | flow/41-approval-flows.spec.ts | 业务审批扩容：10+ /approve 端点四态（申→批→生效→驳回）；export/role-change/writeoffs/transfer 后端流真实调用（API 级）；audit-logs 出现 APPROVE 记录断言 | P1.1/P1.4 |
| 5.12 | flow/42-traversal-skeleton.spec.ts | admin 遍历骨架：抽 8-10 模块（覆盖每业务域 1 个）走 新建→保存→列表回读→字段匹配 三段式 + trackPageHealth 全程 + 404（坏 ID）/400（坏参数）抽样 | P3.2 |
| 5.13 | flow/43-duplicate-toast.spec.ts | 连续点击提交 5 次 → expectSingleToast + 按钮 loading 禁用断言；dialog 重复实例计数 | P3.2 |

改造项：`09-permissions.spec.ts`（见 5.3）、`22-crm-full.spec.ts:55`（见 5.9）、打印内容断言依赖 jszip（package.json devDependencies +1）。

导出内容断言（xlsx）：现有 3 处下载断言升级——jszip 解包 xl/worksheets/sheet1.xml 断言列头与行数 ≥ 列表数据量（exceljs 暂不引入）。

---

## 7. P6 CI 配置（1 commit）

- `.github/workflows/ci-cd.yml`：
  - E2E 分片矩阵 env 增补角色账号注入（E2E_{ROLE}_USERNAME/PASSWORD 从 secrets/变量展开，或 global-setup 生成后写文件由 spec 读取——采用后者则 CI 零改动）；
  - main coverage job 不动。
- `frontend/package.json`：devDependencies + jszip。

---

## 8. Commit 划分（本地，推送待授权）

| commit | 内容 |
|--------|------|
| 1 | feat(backend): 敏感导出 fail-closed 令牌校验 + APPROVE 审计分类（P1.1/P1.4） |
| 2 | fix(backend): lock-status 匿名可查 + 锁定阈值 5→9（P1.2/P1.3） |
| 3 | fix(backend/version): get_current_version 编译期化 + release job 去 VERSION 推送（P1.5） |
| 4 | feat(frontend): 登录瀑布修复 + 协议/隐私页 + 导出审批 UI（P2.1/2.2/2.3） |
| 5 | test(e2e): 去 mock 化真实登录 + lock-status 拦截移除（P2.4） |
| 6 | test(backend): 权限矩阵/APPROVE 分类/导出令牌集成测试（P4） |
| 7 | test(e2e): 角色账号基建 + 健康断言库 + 13 新 spec + 3 改造（P3/P5） |
| 8 | chore(ci): playwright 业务目录纳入 + jszip（P6） |

---

## 9. 风险清单与执行时检查点

| 风险 | 缓解 |
|------|------|
| P1.1 后现有后端测试直接调 export handler 403 | 执行时 grep tests，同步补 token |
| mock 换真实登录后 55 spec 行为漂移（依赖 mockAuthMe 的用户名/角色断言） | 先跑 smoke 全量分片；漂移 spec 逐个修断言（预期少量） |
| 种子角色账号 API 创建路径受限 | 回退 ensureShardUserViaUI 的 UI 创建模式 |
| price_list 敏感映射 handler 未定 | 执行时按 is_sensitive 注释定位 |
| flow 16 分片 lock-status 并发挂起可能在拦截移除后复现 | 后端 handler 已改轻量匿名查询；若复现属真实性能问题，记录后另立项（不回加 mock） |
| 版本号方案 C 影响 check_for_updates 对比口径 | CI 部署验证 /system-update/version 与 tag 一致 |
| 本地禁止编译 | 后端语法/逻辑靠 CI cargo test；前端 tsc/eslint 本地可跑（属 lint 非编译，允许） |
| 一轮工作量饱和 | P2.3 仅做 export-approvals 最小闭环；遍历测试仅骨架抽样 8-10 模块；其余按 doto 分批 |

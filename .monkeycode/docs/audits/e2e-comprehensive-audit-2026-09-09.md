# E2E 综合审计报告（审批 / 版本号 / 登录链路 / 全功能遍历 / 真实性）

> 审计日期：2026-09-09
> 范围：审批体系 E2E、版本号机制、登录链路、协议页、锁定阈值、admin 全功能遍历、页面显示异常、多次提示、预览功能、双因素认证（2FA）、E2E 真实性（用户标注"关键"）
> 状态：仅审计与盘点，源代码冻结（IR 2026-09-09）下未做任何实施；修复项全部待用户解冻授权。

---

## 一、E2E 真实性盘点（用户关键问题："E2E 测试是否全部基于项目真实链路"）

**结论：不是全部真实。218 个 spec 中 58 个文件含 mock，其中 55 个 spec 使用 `applyAuthMocks` 伪造登录态，直接违反禁 mock IR。**

| 类别 | 数量 | 明细 |
|------|------|------|
| 使用 applyAuthMocks（伪造登录/权限 API） | 55 spec | smoke/_helpers.ts（smoke 全套件入口）、bpm/01-02、purchase-ext/01-03、quality/01-02、finance/01-03、crm/01-03、enhanced/network-resilience 等 |
| 主 flow 套件内关键 mock | 1 处 | flow/helpers.ts:898 `loginViaUI` 拦截 `**/api/v1/erp/lock-status**` 用 route.fulfill 返回固定 200 |
| 完全真实链路 | 主 flow 套件（394 测试，除上述 lock-status 拦截外） | 真实后端 + 真实 PostgreSQL |

**flow/helpers.ts:898 lock-status 拦截的双重危害**：
1. 注释自述"避免 16 shard 并发时后端挂起 5s+ 导致登录超时"——把后端并发性能问题掩盖为测试基建补丁；
2. 把登录失败链路的 lock-status 401 瀑布 bug（见第四节）完全掩盖——E2E 永远测不到真实登录失败路径。

**整改方向**（待解冻）：
- 55 个 applyAuthMocks spec 改为真实登录；前置依赖 = 多角色种子账号基建（E2E_{ROLE}_USERNAME/PASSWORD 注入 CI，见 doto.md 权限审计节）；
- 移除 loginViaUI 的 lock-status 拦截，前置依赖 = 后端 lock-status 并发挂起修复 + 登录瀑布修复；
- enhanced/network-resilience 的 page.route 注入 401/403 改为真实无权限账号/过期 token 触发。

---

## 二、双因素认证（2FA / TOTP）E2E 零覆盖

**后端能力完整，测试覆盖为零。**

- 端点：`/auth/totp/setup`、`/auth/totp/enable`、`/auth/totp/recovery-codes`（routes 盘点确认）；
- 登录链路支持：LoginRequest.totp_token（auth_handler.rs L46），v11 批次 141 支持恢复码替代（L47），响应含 is_totp_enabled（L91/L124，users 表已有列）；
- 服务层：services/totp_service.rs 存在；
- E2E：`grep totp` 在 frontend/e2e 命中 0 行（仅 29c 路由清单文案出现"双因素"字样），无任何 2FA 功能测试。

**缺口清单**（待解冻后立项）：
- [ ] setup → enable 全流程（真实扫码/secret 提取 → 生成码启用）；
- [ ] 启用后登录必须带 totp_token：缺 token 拒绝、错 token 拒绝、对 token 成功；
- [ ] 恢复码登录（含恢复码一次性消费）；
- [ ] 禁用 2FA 后登录恢复单因素；
- [ ] is_totp_enabled 在 /auth/me 与登录响应的一致性。

---

## 三、预览功能 E2E 零覆盖

**前端三处预览 UI + 后端 1 个预览端点，E2E `grep preview` 命中 0 行。**

- 后端：`/templates/{id}/preview`（BPM 模板预览）；
- 前端：print-templates/index.vue、report-templates/index.vue、bpm/templates.vue 三处预览；
- 用户要求三个维度全部零测试：能否成功预览 / 预览与需预览内容一致 / 显示完整。

**缺口清单**：
- [ ] 预览可达性：三处预览按钮点击 → 预览界面出现（dialog/新页）且无 pageerror；
- [ ] 内容一致性：print-templates 预览断言模板字段（单据号/公司名/表格头）出现在预览 DOM 中；BPM 模板 preview API 返回内容含模板占位符替换结果；
- [ ] 完整性：预览容器无横向截断/空白主体（白屏断言复用第 5 节基建）；
- [ ] print-templates CRUD + preview + setDefault + copy 的 API 级真实链路 spec（与 doto.md 打印模板项合并）。

---

## 四、登录链路三项根因（已定位，待解冻修复）

### 4.1 一次点击 5 请求瀑布（密码错误场景）

请求链（已逐行定位）：
1. POST /auth/login → 401；
2. `login()`（api/auth.ts L29）未置 `_skipAuthRetry`（仅 refreshToken L58 置位）→ request.ts L194 的 401 分支触发 POST /auth/refresh → 401；
3. Login.vue L318 catch 调 refreshLockStatus() → GET /lock-status；
4. lock-status 路径在 PUBLIC_PATHS（public_routes.rs L6）但 handler 签名强制 AuthContext（login_security_handler.rs L154）→ 401（路由放行与 handler 需认证自相矛盾）；
5. 该 401 再次触发 refresh → 401。

成功路径干净（200 直返，无多余请求）。

**修复方案**（待解冻）：
- [ ] auth.ts `login()` 添加 `_skipAuthRetry`（与 refreshToken 一致）；
- [ ] lock-status handler 改 OptionalAuthContext 或从 PUBLIC_PATHS 语义对齐（二选一，建议 handler 改可选认证以匹配路由公开意图）；
- [ ] Login.vue 失败 catch 移除 refreshLockStatus 或独立 catch 不走 401 拦截器。

### 4.2 协议/隐私页死链

- Login.vue L74/78：`<a href="#/terms">`、`<a href="#/privacy">`——hash 伪路由；
- router/index.ts 用 `createWebHistory`（非 hash 模式）→ 点击后 URL 变 `/#/terms` 但 Vue Router 不响应；
- router 无 /terms、/privacy 路由；frontend/public/ 无任何协议/隐私内容文件；
- 结论：点击无任何效果，纯死链（L74 附近"fix"注释有误导性）。

**修复方案**（待解冻）：新增 /terms、/privacy 路由 + 静态内容页（或 public/ 下独立 HTML），链接改为 router-link。

### 4.3 锁定阈值 5 → 9（用户明确要求）

- auth_handler.rs L30 `MAX_FAILED_ATTEMPTS: i32 = 5`（IP 维度，recent_ip_failures ≥ 5 锁定）；
- 用户全局维度 = MAX_FAILED_ATTEMPTS × 2 = 10；
- login_security_handler.rs L95 同名常量 5（lock-status API 展示）；
- LOCKOUT_DURATION_MINUTES = 30（两处）。

**待解冻修改**：两处常量 5 → 9；展示端（lock-status）同步；需用户确认全局锁定是否维持 ×2 比例（9 → 18）。

---

## 五、admin 全功能遍历 E2E 设计（用户 b-j 项逐条对照）

现状：29 系列 64 测试仅三层浅验证（visitPage / verifyTable 表头 / verifyNewButton 弹窗开合）；业务流 394 测试覆盖核心 20+ 实体 CRUD，其余 80+ 模块的保存按钮从未被点击。

| 用户要求 | 现状 | 缺口 |
|---------|------|------|
| b) 按钮全点 | 仅点"新建" | 编辑/删除/导出/打印/审批/提交按钮零点击 |
| c) 保存落库 | 核心实体有，80+ 模块无 | 全模块保存 → 后端 200 + DB 落库 |
| d) 创建后回读 | 零（29 系列） | 列表/详情 API 回读 created 记录 |
| e) 数据显示验证 | verifyTable 只验表头 | 行内容与源数据匹配 |
| f) 崩溃/白屏 | 无 pageerror 监听、无白屏断言 | 全局异常采集 + 空主体检测 |
| g) 参数错误 400 | 无系统性检测 | 表单边界值 → 400/422 + 友好提示 |
| h) 资源不存在 404 | 仅 18 号 spec 2 端点 | 访问不存在 ID → 404 页面/提示 |
| i) error boundary | 零 | 触发渲染错误 → boundary 兜底 UI |
| j) 未知错误聚合 | 无 console.error 汇总 | 全局监听上报 + 测试失败时输出 |

**设计骨架**（新增 30-traversal spec 系列，前置 = 真实登录基建）：
- [ ] 全局监听器注入（helpers 层）：pageerror / console.error / response 状态采集器，测试结束统一断言零未捕获异常；
- [ ] 白屏断言：每次 visit 后断言主容器非空（innerText 长度阈值）；
- [ ] 保存→落库→回读三段式：每模块 1 条代表记录走通 新建→保存→列表回读→字段匹配；
- [ ] 404/400 抽样：每系列 1-2 个端点的坏 ID / 坏参数。

---

## 六、页面显示异常与多次提示（用户本轮新增）

- [ ] **重复提示**：E2E 无任何 toast 去重断言。补：连续点击提交按钮 N 次 → 断言 ElMessage/ElNotification 同文案实例 ≤1（或按钮 loading 禁用生效）；
- [ ] **重复弹窗**：同一 dialog 重复打开实例检测（DOM 计数）；
- [ ] **显示异常**：与第 5 节 f/j 项合并（pageerror/白屏/console.error 三件套）；
- [ ] **多语言/样式残留**：抽样断言页面无未翻译 key（`xxx.key` 字面量）与 NaN/undefined 渲染值。

---

## 七、审批体系审计（前轮排查补录）

- 后端规模：58 个 /approve 端点 + 15 个 reject/submit；专用审批流 4 套：export-approvals（8 端点）、role-change-approvals（L1/L2 两级）、transfer-approvals（manager/director 两级）、writeoffs（finance/general-manager 两级）+ BPM 引擎（bpm_instances/bpm_tasks/bpm_process_definition）；
- 业务审批门禁真实存在：预算未审批不可执行、质量标准未审批不可发布、大货批色未审批阻断发货、固定资产状态机审批、报价单金额阶梯（<10 万自批 / ≥10 万入 BPM）；
- SoD：9 对角色互斥 + sales_manager/purchase_manager 创建/审批分离。

**缺口**：
- [ ] E2E 真实调用过的审批端点仅 5/58 = 8.6%（purchase/orders、quotations、sales/orders、dye-recipes、production-recipes）；
- [ ] 10d-extended-approval-bpm.spec.ts：6 测试 5 个 try/skip 空转，金额阶梯断言恒真（status ∈ 四值集合）；
- [ ] bpm/02-approval.spec.ts 用 applyAuthMocks 且 bpm 目录不在主 CI testMatch（仅 e2e-batch.yml）；
- [ ] 前端审批 UI 零接入：export-approvals、role-change-approvals、transfer-approvals、writeoffs 双审批在 frontend 无任何调用；BPM 审批中心仅接 /bpm/tasks 监控端点；
- [ ] 审批追溯：omni_audit 五要素齐（ip_address/user_agent/username/method/body，resolve_client_ip 解析 X-Forwarded-For/X-Real-IP）但 classify_operation（omni_audit.rs L405）仅 PRINT/EXPORT/DOWNLOAD/CRUD/OTHER，**无 APPROVE 分类**——无法按事件类型筛"谁在何时批了什么"；export_approval_request 模型追溯字段完整（approver_user_id/username/comments/approved_at/applicant_ip）；业务表仅 approved_by/approved_at（无 IP）。

**修复方向**（待解冻）：
- [ ] classify_operation 增加 APPROVE 分类（按路径 /approve|/reject|/writeoff|/transfer-approve 匹配）；
- [ ] 审批真实流 E2E：申→批→生效→驳回四态，覆盖 4 套专用流 + BPM 主干；
- [ ] 前端审批中心接入 4 套专用审批流（UI 开发，需用户立项）。

---

## 八、版本号机制故障（根因已定位，含证据链）

**三方不一致**：
| 位置 | 值 | 说明 |
|------|-----|------|
| main VERSION 文件 | 2026.723.1842 | 最后成功写入 2026-08-26，之后停更 |
| main Cargo.toml | 2026.810.1 | RLS PR #939 手工改 |
| 最新 Release/tag | v2026.9.9.1953 | run 4483（id 34347843357，2026-09-09） |

**根因**：release job 在 runner 上本地 commit d38c9f6f（VERSION 更新）后 `git push origin HEAD:main` 被分支保护规则拒绝——日志原文 `! [remote rejected] HEAD -> main`，原因 "Changes must be made through a pull request" + "Commits must have verified signatures"（github-actions[bot] 签名不被认可）。commit 遗留 runner 磁盘未入 git。

**影响**：二进制内版本经 `env!("CARGO_PKG_VERSION")` 是新的（CLI upgrade.rs L331/L472），但 system_update 的 get_current_version 读 VERSION 文件 → 升级对比逻辑读到 2026.723.1842 旧值 → check_for_updates 永远误报"有可用更新"。

**修复方向**（待用户决策 + 授权 workflow 变更）：
- [ ] 方案 A：release job 改用 GitHub API contents 接口提交（需 bypass 权限 token，绕过分支规则）；
- [ ] 方案 B：版本落库改走 PR 流程（job 自动开 PR → auto-merge → tag）；
- [ ] 方案 C：get_current_version 改读编译期 CARGO_PKG_VERSION，VERSION 文件仅作打包元数据（改动最小，推荐评估）；
- [ ] 分支规则放行 github-actions[bot]（需仓库管理员操作）。

---

## 九、修复实施序列建议

```
P0 基建（前置）：多角色种子账号 → 55 spec 去 mock 化 → loginViaUI 去 lock-status 拦截
P0 缺陷修复：登录 5 请求瀑布（4.1）、lock-status 路由/handler 矛盾、版本号（第八节）、协议页死链（4.2）
P0 安全：敏感导出令牌强制校验（doto.md 二轮审计项，fail-closed）
P1 阈值：MAX_FAILED_ATTEMPTS 5→9（两处 + 展示端）
P1 新测试（按序）：2FA 全流程（二节）→ 预览三维度（三节）→ 审批真实流 + APPROVE 分类（七节）→ 越权/黑名单/更新门禁（doto.md 一二节）→ 全功能遍历骨架 + 显示异常/重复提示（五六节）
```

## 十、证据文件索引

- 版本号：/tmp/release_job.log、/tmp/build_job.log、/tmp/pkg_job.log、/tmp/jobs4483.json（run 34347843357）
- 登录链路：frontend/src/views/Login.vue L54/74/78/267/318、frontend/src/api/auth.ts L29/58、frontend/src/api/request.ts L194、backend/src/middleware/public_routes.rs L6、backend/src/handlers/login_security_handler.rs L95/154、backend/src/handlers/auth_handler.rs L30
- 审批：backend/src/middleware/omni_audit.rs L59/405、backend/src/models/export_approval_request.rs L107/194、backend/src/services/export_approval_service.rs L198/366/407
- 真实性：frontend/e2e/flow/helpers.ts L898、frontend/e2e/smoke/_helpers.ts L11、grep applyAuthMocks = 55 spec
- 2FA：backend/src/handlers/auth_handler.rs L11/46/47/91/124、backend/src/services/totp_service.rs
- 遍历基线：frontend/e2e/flow/29a-d-route-coverage-*.spec.ts（64 测试）

# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单），进度必须真实，禁止乐观偏差。
> 已完成任务见 [doto-su.md](doto-su.md)，一句话总结见 [CHANGELOG.md](CHANGELOG.md)，规则见 [MEMORY.md](MEMORY.md)。

---

## 当前状态

**Setup 向导真实链路 E2E 已重构为零 mock 版（用户指令：禁 mock/真实数据/禁本地编译/禁推送），CI 专用 job `ci-e2e-setup-wizard` 就绪（独立空库 bingxi_setup_test + Release 真实二进制 + UI 真实点击初始化 + 真实登录验证），静态检查全绿。全部改动已本地提交（commit 见 git log），推送冻结等用户指令，验证将在 CI push 后进行。**

---

## 未完成任务清单

### 硬约束（用户指令 2026-09-07，后续所有任务必须遵守）

- [x] 禁止 mock：E2E 测试一律真实后端 + 真实 PostgreSQL 数据（00-setup-wizard.spec.ts 已重写，0 处 page.route）
- [x] 禁止本地编译：真实二进制从 GitHub Release 下载（本地 glibc 不兼容 CI 产物，本地运行/编译路线已废弃，rustup 与 release 临时文件已清理出工作区）
- [x] 禁止推送：git push 冻结，仅本地 commit；CI 验证等用户明确授权推送后触发

### Setup 向导真实链路 E2E（CI 端，待推送后首跑验证）

- [x] 首跑（run 34135035908）结果：step#9 安装 Playwright 失败（--with-deps 的 apt 锁竞争），后续 step 全跳过——真实链路本身未执行
- [x] 修复①：拆分 install/install-deps（deps 失败仅告警）；修复②：补 Playwright 缓存层（与 ci-e2e 共用 key，命中免下载，用户指正）
- [ ] 推送授权后重跑：观察 ci-e2-setup-wizard job 9 个真实测试是否全绿
- [x] Clippy failure 根因定位：`validate_admin_role` 中 `if let Some(id)=auth.role_id { id } else { ... }` 的 else 分支不可达（前面 `if auth.role_id.is_none() { return Ok(()); }` 已 early return）→ clippy 不可达代码警告 → NEW_COUNT>0 阻塞。已修复（改用 `auth.role_id.expect(...)`，None 已由守卫保证为 Some）
- [ ] Rust 测试 (2/30) failure：测试不直接构造 AuthContext struct literal、不调用 cookie 函数，本批改动不应破坏现有测试——partition-2 可能是存量 flaky 或与本批无关，待授权重跑确认
- [ ] E2E 分片 3 failure：待 logs 确认（API 限流，job logs 需 admin 权限）
  - [x] 🔴 **部署脚本遗漏 INIT_TOKEN 生成**：deploy.sh/deploy-latest.sh 自动生成 JWT/COOKIE/WEBHOOK/AUDIT 四把密钥但无 INIT_TOKEN；init_token_middleware fail-secure（未配置/占位值/长度<32 一律 401）→ 全新部署走引导页第 4 步安装必然 401，用户无从得知应填令牌值（backend/.env.example 占位值也被 is_init_token_strong 拒绝）。修复：两脚本按其它密钥同策略自动生成+持久化 /etc/bingxi/.env（deploy.sh 完成横幅输出值；deploy-latest.sh 提示 grep 命令）
  - [x] 🔴 **HTTP 部署 Cookie Secure 冲突**：nginx.conf 改 80 端口直接服务（443 可选），但 config.yaml env=production → is_production()=true → 登录 Cookie secure(true) → 浏览器 HTTP 下拒存 → 登录成功但会话无法建立（登录异常根因）。修复：utils/config.rs 新增 cookie_secure_for_request(headers)（X-Forwarded-Proto 优先，回退 is_production），login/refresh/logout 三链路统一接入（nginx 已配置 proxy_set_header X-Forwarded-Proto $scheme）
- 渲染异常（无缺陷）
  - [x] Setup.vue 编译契约核验：FormItemRule 导入、resetInitStatus 导出（router/index.ts:1190）、i18n key（zh-CN/en-US passwordComplexity 新增）、E2E 选择器锚点（.check-item/.install-summary/.success-icon）全部一致
- 访问异常（无缺陷，含既有修复回归确认）
  - [x] 路由守卫：checkInitStatus 失败不缓存（重启窗口不锁死）、兼容 setup/完整双模式响应形态
  - [x] CORS 已放行 x-init-token 头；PUBLIC_PATHS 已放行 /init/test-database（handler 内 OptionalAuthContext 门禁）
  - [x] 自退切模式：initialize 成功 → 2s 后 exit(0) → systemd Restart=always 拉起（bingxi-backend.service:15）→ 完整模式
- 登录异常（1 个真实缺陷，即上述 Cookie Secure 修复）
  - [x] 初始化创建管理员走 AuthService::hash_password_async + password_changed_at 锚点，登录链路正常
  - [x] 前端密码校验与后端 PasswordPolicy::default() 对齐（≥8+大小写+数字+特殊字符）

### Setup 向导真实链路 E2E 设计（CI job ci-e2e-setup-wizard，零 mock）

- [x] 专用空库 bingxi_setup_test：setup-wizard-real-env.sh 每次 DROP+CREATE（后端空库无表 → 启动即 Setup 模式）
- [x] 真实后端二进制：CI 复用 ci-build-rust 的 backend-binary artifact（端口 8083，独立于 34 分片共享的 8082/bingxi_test）
- [x] UI 真实点击初始化：环境检查 → test-database（真实 SELECT 1）→ initialize-with-db（真实迁移 60+ + 种子 + Argon2id 管理员）→ 后端自退
- [x] 后端重启：restart-backend-full.sh 测试内触发（等效 systemd Restart=always）→ 完整模式
- [x] 真实登录验证：PSQL 直查 users 行 + API 登录（Argon2id + Cookie + *:* 权限）+ UI 登录进 Dashboard
- [x] vite.config.ts 代理可配置：VITE_PROXY_TARGET / VITE_DEV_PORT 环境变量（默认 8082/3000 不变，Setup job 用 8083/3100）
- [x] 专用 Playwright 配置 playwright.setup-wizard.config.ts（baseURL 3100、serial 顺序、storageState 关闭——初始化是单向状态跃迁）
- [x] 9 个真实测试：前置 Setup 模式断言 / 环境检查 / test-database / UI 初始化全流程 / PSQL 数据级断言 / 自退重启完整模式 / API 真实登录 / 登录页渲染 / UI 登录进主应用

### 流程备忘

- [ ] CI 失败若再出现新 job：按 doto 流程拉日志→记录→下批修复

### 00-responsive 多机型矩阵（已完成待推送）

- [x] 21 机型视口规格定稿：CN_PHONES 7（华为 Mate60Pro/P50Pro、荣耀 Magic5、小米14、vivo X90 Pro、OPPO Find X6 Pro、一加11）+ CN_TABLETS 4（华为 MatePad11、小米 Pad6、OPPO Pad2、三星 Tab S9）+ 苹果手机 3（SE 375/14 390/14 Pro Max 430）+ 苹果平板 2（Mini/Pro 11）
- [x] 测试结构：登录页 10（storageState 置空）+ 主应用 16（登录态+抽屉交互）+ 断点边界 991/992 + 横屏 2 + 桌面 3（含热切换）= 32 测试
- [x] 契约锚点核对：`.aside` 全项目唯一；汉堡 `v-if="isMobile"`；抽屉 closeOnPressEscape=true + destroyOnClose=false（ESC 关闭断言成立）
- [x] 静态验证：esbuild 语法 + prettier 格式双通过

后续新增任务请在此文件追加。

# 项目规则记忆

> 本文件是项目的**规则记忆**，记录必须遵守的规则、指令、偏好和工作流规范。
> 本文件为本地工作记录（`.monkeycode/` 目录在 `.gitignore` 中），不通过 PR 推送。

---

## 文件定义

| 文件 | 用途 | 说明 |
|------|------|------|
| `MEMORY.md` | 项目规则记忆 | 记录必须遵守的规则、指令、偏好和工作流规范 |
| `doto.md` | 项目任务记忆 | 详细记录所有执行过的任务、任务状态、任务结果、遇到的问题等 |
| `CHANGELOG.md` | 任务总结精简版 | doto.md 的精简版，记录任务总结 |

---

## 一、格式说明

### 用户指令条目
```
[用户指令摘要]
- Date: [YYYY-MM-DD]
- Context: [提及的场景或时间]
- Instructions:
  - [用户教导或指示的内容]
```

### 项目知识条目
```
[项目知识摘要]
- Date: [YYYY-MM-DD]
- Context: Agent 在执行 [具体任务描述] 时发现
- Category: [运维部署|构建方法|测试方法|排错调试|工作流协作|环境配置]
- Instructions:
  - [具体的知识点]
```

---

## 二、基本要求

[任务管理]
- Date: 2026-06-19
- Context: 用户明确要求任务管理规范
- Category: 工作流协作
- Instructions:
  - 使用中文建立 DOTO 待办任务
  - 每完成一个待办任务，需立即标记为"已完成"状态
  - doto.md 文件需要反映任务的最新情况，也需要实时更新

[任务规划管理]
- Date: 2026-06-19
- Context: 用户明确要求任务规划文件存放位置
- Category: 工作流协作
- Instructions:
  - 所有任务规划文件必须保存在 `.monkeycode` 文件夹下面 `docs` 文件夹里面

[沟通语言]
- Date: 2026-06-19
- Context: 用户明确要求沟通语言规范
- Category: 基础偏好
- Instructions:
  - 使用中文进行回复和沟通

[编码规范]
- Date: 2026-06-19
- Context: 用户明确要求编码规范
- Category: 开发规范
- Instructions:
  - 项目禁止硬编码，所有文本需要使用中文
  - 代码注释必须使用中文

[开发辅助]
- Date: 2026-06-19
- Context: 用户明确要求开发辅助规范
- Category: 工作流协作
- Instructions:
  - 每次新增或修改功能时，必须指定合适的技能或 MCP 工具进行开发辅助
  - 严格按照技能规范进行开发，不允许例外

[记忆管理]
- Date: 2026-06-19
- Context: 用户明确要求记忆管理规范
- Category: 工作流协作
- Instructions:
  - 实时查看和更新 `.monkeycode` 文件夹 MEMORY.md 记忆文档
  - 所有关键内容需存储在保存在 `.monkeycode` 文件夹里面 MEMORY.md 文档记忆中
  - 生成的文档需实时更新，删除旧文档时需提取关键内容存储
  - 项目的重要变更需要查找并记录到保存在 `.monkeycode` 文件夹 CHANGELOG.md 文档
  - **.monkeycode/ 路径策略（2026-06-19 确认）**：test 分支合并入 main 时，`-X theirs` 策略会覆盖 `.monkeycode/` 内容，必须以 main 版本的 `.monkeycode/` 为准；test 自身的 `.monkeycode/docs/` 不应混入 main

[数据库配置]
- Date: 2026-06-19
- Context: 用户明确要求数据库配置规范
- Category: 环境配置
- Instructions:
  - 数据库类型：PostgreSQL
  - 连接方式：使用远程数据库连接模式，确保数据库连接模块的稳定性和安全性

[功能实现依据]
- Date: 2026-06-19
- Context: 用户明确要求功能实现依据
- Category: 开发规范
- Instructions:
  - 功能实现必须严格按照技能进行推进
  - 新增功能的接口、数据库操作需遵循现有规范

[打包与发布要求]
- Date: 2026-06-19
- Context: 用户明确要求打包与发布规范
- Category: 运维部署
- Instructions:
  - 打包时必须进行全面的项目测试，包括但不限于：
    - 全面的功能测试
    - 兼容性测试
    - 稳定性测试
  - 确保打包后的程序能够在目标环境中正常启动并完整运行所有功能模块，无运行时错误或功能缺失

[项目标识]
- Date: 2026-06-19
- Context: 用户明确要求项目标识规范
- Category: 基础偏好
- Instructions:
  - 项目名称：询问用户项目基础信息，所有相关文档、界面及输出信息中必须统一使用

---

## 三、工作流协作

[工作角色定位]
- Date: 2026-05-27
- Context: 用户明确要求助手的角色定位和工作方式
- Category: 工作流协作
- Instructions:
  - 我的角色是总控（项目经理/架构师）
  - 子代理（Task工具）是我的员工，负责具体执行任务
  - 所有任务都有对应的员工，员工拥有完成该任务的所有技能
  - 用户输入的所有内容都需要我进行分析，然后分配任务给员工
  - 我的职责是：分析用户任务 → 拆解任务 → 分配给员工 → 总结员工成果 → 推送PR
  - 不要自己直接写代码，而是分配给员工执行
  - 员工完成后，我需要总结他们的工作成果，然后推送到PR

[前端 Vue Router 路由架构知识]
- Date: 2026-06-19
- Context: Agent 在执行"前端 Vue Router 路由审计"任务时发现
- Category: 工作流协作
- Instructions:
  - **路由规模**：`frontend/src/router/index.ts` 共 709 行，**114 路由条目 / 110 可导航路由**（含 1 MainLayout 父路由 + 106 子路由 + 3 独立页 Login/Setup/403/404 + 3 redirect/catch-all）
  - **嵌套深度**：仅 1 层（MainLayout 一级嵌套）
  - **路径别名**：`vite.config.ts:19-21` 配置 `@` → `src`；`tsconfig.json:18-21` 同步配置
  - **name/path 唯一性**：✅ 100% 唯一
  - **meta 字段**：当前 110 可导航路由 100% 含 `title` + `requiresAuth`（除 4 个公开页），**0 条**含 `icon`/`permission`/`roles`/`keepAlive`/`breadcrumb`
  - **模块分布 TOP 3**：财务 16（14.5%）/ 销售 11 / 库存+物流 10
  - **动态路由 10 条**：crm/detail/:id、quotations/:id{,.edit,.approval}、custom-orders/:id{,.track}、color-cards/detail/:id、color-prices/detail/:id、ai-extend/process-detail/:id
  - **审计报告**：[.monkeycode/docs/audits/2026-06-19-frontend-router-audit.md](file:///workspace/.monkeycode/docs/audits/2026-06-19-frontend-router-audit.md)
  - **关键 P0 错配 1 处**：
    - `router/index.ts:638-639` `ColorPriceCreate` 路由 component 错配为 `color-prices/list.vue`（应为不存在的 `create.vue`）— 5 分钟内修复
  - **关键 P0 菜单孤儿 1 处**：
    - `MainLayout.vue:144` 菜单项 `/system/slow-query` → 无路由（页面 `system/slow-query/index.vue` 已存在但未挂载）— 后端 P13 批 1 B 慢查询审计 4 端点已上线
  - **P1 死代码页面 17 + 子文件 23**（4 大死代码页面组）：
    - `bpm/approval/`（1+7）— 拆分完整但未挂载
    - `bpm/definitions/`（1+7）— 与 `bpm/definitions.vue` 重复
    - `security/two-factor/`（1+7）— 注释承诺路由引用但未实现
    - `admin/failover.vue` + 3 components — 主备隔离 UI 未挂载
    - `crm/leads/index.vue` + `crm/opportunities/index.vue`（+ 3 tabs）— CRM 子模块未挂载
    - `bi/index.vue` — BI 入口预留
    - `security/ChangePassword.vue` — 功能合并到 user-profile
    - `report/templates.vue` + 11 子文件 — P12 拆分前残留
    - `sales/tabs/{SalesOrderFilter,SalesStatsCards}.vue` — 被 OlvFilter/OlvStat 取代
  - **P2 元信息缺失**：106/106 子路由缺 `icon`/`permission`/`keepAlive`/`breadcrumb`（暂不影响运行）
  - **治理建议**：建立 `frontend/src/router/types.ts` 的 `RouteMeta` 接口、删除废弃 alias `/workflow`、统一 children 路径前缀策略

[后端 HTTP API 路由架构知识]
- Date: 2026-06-19
- Context: Agent 在执行"后端 HTTP API 路由审计"任务时发现
- Category: 工作流协作
- Instructions:
  - **路由规模**：`backend/src/routes/*.rs` 共 20 文件，**943 路由条目 / 905 唯一 method+path**
  - **HTTP 方法分布**：GET=447 / POST=320 / PUT=96 / DELETE=80
  - **业务域 TOP 3**：财务 196（finance.rs 双挂载） / 分析-高级功能 136 / 采购 95
  - **路由聚合链**：`main.rs:96-103 → routes::create_router(state) → mod.rs:138-202`
  - **挂载模式**：
    - `.nest("/api/v1/erp/{domain}", module::routes())` 适用于独立子域
    - `.merge(module::routes(state))` 适用于无前缀或与根共享路径的子路由
    - `pub fn router()` 内部 nest 模式：crm/purchase 等域内多个 handler 模块各自 `pub fn router()`，由域级 routes/*.rs 统一 merge
  - **挂载前缀异常点**：
    - `routes/failover.rs` 使用**完整路径**（`/admin/failover/*`），不挂到 `/api/v1/erp` 下
    - `routes/static.rs` 挂到根 `/`
    - `routes/v1.rs` 仅 1 个 `GET /api/v1/placeholder`
  - **审计报告**：[.monkeycode/docs/audits/2026-06-19-backend-api-audit.md](file:///workspace/.monkeycode/docs/audits/2026-06-19-backend-api-audit.md)
  - **关键 P0 启动时 panic**：
    - `routes/sales.rs:116` 引用 `quotation_handler::convert_quotation_to_order`（实际为 `convert_to_sales_order`）
    - `routes/sales.rs:120` 引用 `quotation_handler::list_expiring_quotations`（实际为 `list_expiring`）
    - `routes/system.rs:28` 引用 `websocket::ws_notifications_handler`（实际为 `websocket::notifications::ws_notifications_handler`）
  - **关键 P0 孤儿路由 18 个**：`routes/custom_order.rs` 整模块 18 端点，`mod.rs:58` 仅声明 `pub mod custom_order;`，`create_router` 中**未 nest/merge**
  - **handler 命名规范**：
    - `define_tenant_crud_handlers!` / `define_crud_handlers!` 宏自动生成 `list/create/get/update/delete` 5 个函数
    - 兼容层：`ar_reconciliation_enhanced_handler` / `currency_enhanced_handler` 全部 `pub use` 真实 handler（薄别名）
  - **INTERFACES.md 漂移**：65 个"未实现"端点实际全部因文档缺 `/api/v1/erp` 前缀或占位符风格不一致（`{}` vs `:id`）导致，**非真实缺失**

[前端 API 架构知识]
- Date: 2026-06-19
- Context: Agent 在执行"前端 API 调用审计"任务时发现
- Category: 工作流协作
- Instructions:
  - **baseURL 链路**：`vite.config.ts:server.proxy['/api'] → http://localhost:8082`，运行时走 `.env.{VITE_API_BASE_URL}` 默认为 `/api/v1/erp`，最终被 `frontend/src/api/request.ts:54` 的 `axios.create({baseURL})` 消费
  - **axios 拦截器（request.ts:66-181）**：注入 `Authorization: Bearer` + `X-CSRF-Token`；401 自动 refresh + 重放；502/503/504 指数退避重试 3 次
  - **CSRF 公开路径白名单（request.ts:30-40）**：`/auth/login, /auth/refresh, /auth/logout, /auth/csrf-token, /init, /health, /ready, /live, /tracking/page-view`
  - **敏感信息存储**：3 个 token（access_token / refresh_token / csrf_token）均明文存于 `localStorage`，未加密，无 httpOnly 兜底
  - **兼容层文件**（re-export，无实际调用）：`ap-invoice.ts` / `ap-payment.ts` / `ap-reconciliation.ts` / `ap-verification.ts` 全部 re-export from `ap.ts`
  - **API 文件总数**：89 业务文件 + 2 基础设施（`request.ts` / `index.ts`） = 91 合计
  - **审计报告**：[.monkeycode/docs/audits/2026-06-19-frontend-api-audit.md](file:///workspace/.monkeycode/docs/audits/2026-06-19-frontend-api-audit.md)
  - **关键孤儿 P0 路径**（前端调用后端未注册）：
    - `/api-gateway/*`（14 处，后端完全未实现）
    - `/api/v1/erp/custom-orders/*`（17 处，路由文件存在但 `mod.rs:330-360` 未 nest）
    - `/purchase/receipts/*`、`/purchase/suppliers/*`、`/purchase/purchase-contracts/*`（前端用单数、后端用复数 `/purchases/*`）
    - `/production/production-orders/orders/*`（10 处）、`/production/greige-fabrics/*`（8 处）、`/crm/customer-credits/*`（11 处）后端未注册

[doto.md 实时更新规则]
- Date: 2026-06-16
- Context: 用户明确要求 doto.md 需实时根据任务更新
- Category: 工作流协作
- Instructions:
  - **触发场景**：每次任务进展（启动/完成/重新规划/状态变化）时，必须实时更新 `/workspace/.monkeycode/doto.md`
  - **更新内容**：
    - 当前待办表格（任务、状态、备注）
    - 任务规划变更（重新规划、波次调整）
    - 波次执行总结（已完成/进行中）
    - 完成回顾（PR 列表、CI 结果、自动发版版本号）
  - **同步规则**：
    - 重要变更（任务完成/重新规划/分支策略调整）需同步更新 `/workspace/CHANGELOG.md`
    - 本地 `.monkeycode/` 目录在 `.gitignore` 中，不通过 PR 推送
    - CHANGELOG.md 通过 PR 推送到 test/main 分支

[2026 现代代码质量审计结果]
- Date: 2026-06-19
- Context: Agent 在执行"冰溪 ERP 现代代码质量审计"任务时发现
- Category: 项目评估
- Instructions:
  - **综合评分**：73/100（B- 级）
  - **报告位置**：[.monkeycode/docs/audits/2026-06-19-modern-code-audit.md](file:///workspace/.monkeycode/docs/audits/2026-06-19-modern-code-audit.md)
  - **6 大 P0 风险**（必须立即修复）：
    1. **文件级 `#![allow(dead_code)]` 83 处越界**（CI 必失败）— services 68 / handlers 2 / middleware 1 / 其他 12
    2. **cookie_secret 静默降级**（`main.rs:325-328` 复用 jwt_secret）— 高危密钥复用
    3. **生产环境随机 JWT secret**（`utils/app_state.rs:193`）— 多副本部署签名不一致
    4. **操作日志静默吞咽**（`middleware/operation_log.rs:76`）— 违反审计完整性
    5. **2 处 v-html XSS 风险**（`report-templates/index.vue:170`、`print-templates/index.vue:212`）
    6. **JWT 存 localStorage**（`utils/storage.ts:1-23`，25 处访问）— XSS 一击必杀
  - **P1 重要**（应当修复）：
    - 132 处 `#[allow(dead_code)]` 项级抑制（60 文件）
    - 409 处 `: any` + 191 处 `as any`（前端类型系统失效）
    - 6 个 .vue > 500 行（TOP purchase 748 / quality 675 / inventory 600）
    - 8 个 .rs > 750 行（TOP so/order 1041 / scheduling 948 / customer_credit 926）
    - 20 处 `panic!`（最严重 `audit_log_service.rs:5`）
    - 116 处 `let _ =` 静默吞咽
  - **已达标项**（无需修改）：
    - `utils/` 8 个核心文件 100% 死代码清理（达成模板）
    - `models/` 200 个 SeaORM 文件级抑制（合规例外）
    - 0 处 `unsafe {` 块
    - 0 处 `@ts-ignore` / `@ts-nocheck` / `eval()` / `innerHTML`
    - 0 处 `auth.tenant_id.unwrap_or(0)` 真实代码违规
    - 0 处空 catch 块
    - SQL 已参数化（无 `format!("SELECT...")` 拼接）
    - 146 处 `extract_tenant_id(&auth)?` 100% 合规
    - CSP / HSTS / X-Frame-Options / CSRF 等 7 项安全头已配置
  - **改进路线图**（D1-D21）：
    - 第 1 周：删 83 文件级抑制 + 修 3 处密钥降级 + 验证 CICD clippy
    - 第 2 周：修 v-html + 分类 132 项级抑制 + 评估 localStorage 迁移
    - 第 3-4 周：拆 6+18 个大 .vue + 8 个大 .rs + 替换 `any`
    - 第 5-6 周：修 116 处 `let _ =` + 20 处 `panic!` + 评估 sleep
    - 第 7-12 周：OIDC 接入 + SAST 工具 + 自动类型生成

[GitHub 版本管理分支策略]
- Date: 2026-06-16
- Context: 用户要求建立规范的分支管理策略
- Category: 工作流协作
- Instructions:
  - **分支结构**：
    - `main` 为主分支（正式版），不允许删除
    - `test` 为测试分支，不允许删除
  - **测试分支 (test)**：
    - 所有修复/功能变更/功能新增等测试均在测试分支进行
    - 测试版分支不需要发布产品安装包
    - 测试分支需要特别详细的全量日志系统
    - 测试版需要自动触发 CI/CD
    - 所有 AI 创建的修复分支在验证后都合并入测试版分支
    - 合并到测试版分支后自动删除修复分支
  - **正式版分支 (main)**：
    - 正式版分支需要发版（发布产品安装包）
    - 不需要详细的日志系统，只保留基础的日志验证
    - 正式版的发版需要手动触发
  - **修复分支流程**：
    - AI 创建的修复分支验证后合并入 test 分支
    - 合并完成后自动删除修复分支
    - test 分支进行自动触发全面测试、代码审计、功能健康校验
    - 全部测试通过后由 test 分支自动合并入 main 分支

[日志诊断技能自动触发规则]
- Date: 2026-06-07
- Context: 用户要求将日志诊断技能改为自动触发模式
- Category: 工作流协作
- Instructions:
  - **技能名称**：`/log-diagnosis` 日志诊断技能
  - **触发方式**：自动触发（无需用户手动输入命令）
  - **自动触发条件**：
    - 用户提到"日志"、"错误日志"、"异常日志"、"崩溃日志"等关键词
    - 用户要求分析服务器日志、应用日志、系统日志
    - 用户提到服务异常、崩溃、报错等问题
    - 用户要求查看服务器日志、拉取日志、分析错误
    - 用户提到 traceId、错误码、异常堆栈等信息
  - **执行流程**：
    1. 环境检查与配置加载（读取 `.diagnosis/config.json`）
    2. 日志搜索与提取（使用 grep/awk/sed 等工具）
    3. 代码联动分析（从日志中提取类名/方法名/文件名）
    4. 根因分析与诊断（综合分析，不可片面下结论）
    5. 生成诊断报告（保存到 `.diagnosis/reports/` 目录）
    6. 清理与恢复
  - **核心规则**：
    - 全量原则：必须统计总数并分批读取完毕，禁止只看前几行就下结论
    - 上下文原则：必须包含前后文（前5行后10行），避免脱离上下文误判
    - 代码验证原则：必须在代码中找到对应位置验证，禁止纯靠日志猜测
    - 报告原则：必须生成结构化报告文件，不可只口头描述
    - 配置优先原则：优先读取配置文件
  - **报告格式**：
    - 保存路径：`.diagnosis/reports/{YYYY-MM-DD}_{问题描述摘要}.md`
    - 包含内容：基本信息、问题描述、日志分析、根因分析、修复建议、附录

---

## 四、运维部署

[项目运行时知识]
- Date: 2026-05-27
- Context: 用户提供项目运维和配置信息
- Category: 运维部署
- Instructions:
  - **服务器信息**：生产服务器 111.230.99.236（SSH: root），数据库服务器 39.99.34.194:5432（用户: bingxi）
  - **敏感信息**：密码等敏感信息禁止记录在记忆文件中，仅通过环境变量或安全配置管理
  - 服务名称: bingxi-backend (systemd)，安装目录: /opt/bingxi-erp
  - 后端端口: 8082，日志目录: /opt/bingxi-erp/backend/logs，备份目录: /opt/bingxi-erp/backups
  - 环境配置: /etc/bingxi-erp/.env
  - 构建限制: 禁止本地编译，只允许 CICD 编译，CICD 自动部署到 GitHub Release，手动部署到生产服务器
  - **服务器不安装PostgreSQL客户端**：有专门的数据库服务器(39.99.34.194)，应用服务器只连接远程数据库
  - **服务器不安装Redis**：有专门的Redis服务器，应用服务器只连接远程Redis
  - **服务器只需安装**: Nginx、curl
  - 部署命令: `bingxi update` (CLI工具)

[部署限制]
- Date: 2026-05-29
- Context: 用户明确要求禁止使用Docker容器部署
- Category: 运维部署
- Instructions:
  - 项目禁止使用Docker容器部署
  - 不得创建Dockerfile、docker-compose.yml等Docker相关文件
  - 部署方式为：CICD构建 → GitHub Release → 手动部署到生产服务器
  - 使用systemd管理服务，不使用容器化部署

---

## 五、代码规范

[命名约定]
- Date: 2026-06-19
- Context: 用户明确要求命名规范
- Category: 开发规范
- Instructions:
  - 使用有意义的、描述性的名称
  - 遵循项目或语言的命名规范
  - 避免缩写和单字母变量（除非是约定俗成的，如循环中的 i）

[代码组织]
- Date: 2026-06-19
- Context: 用户明确要求代码组织规范
- Category: 开发规范
- Instructions:
  - 相关代码放在一起
  - 保持适当的抽象层次
  - 函数只做一件事
  - 保持单一职责原则

[注释与文档]
- Date: 2026-06-19
- Context: 用户明确要求注释与文档规范
- Category: 开发规范
- Instructions:
  - 注释应该解释"为什么"，而不是"做什么"
  - 为公共API提供清晰的文档
  - 更新注释以反映代码变化

---

## 六、安全规范

[租户隔离]
- Date: 2026-06-19
- Context: 用户明确要求租户隔离规范
- Category: 安全规范
- Instructions:
  - 严禁使用 `auth.tenant_id.unwrap_or(0)` 获取租户ID
  - 必须使用 `extract_tenant_id(&auth)?` 进行租户ID提取
  - 确保所有涉及租户数据的操作都进行严格的租户隔离验证

[敏感信息保护]
- Date: 2026-06-19
- Context: 用户明确要求敏感信息保护规范
- Category: 安全规范
- Instructions:
  - 禁止在代码中硬编码敏感信息（密码、密钥、令牌等）
  - 使用环境变量或配置管理工具管理敏感信息
  - 禁止将敏感信息提交到版本控制系统

[输入验证]
- Date: 2026-06-19
- Context: 用户明确要求输入验证规范
- Category: 安全规范
- Instructions:
  - 所有用户输入必须进行验证和清理
  - 使用参数化查询防止SQL注入
  - 对输出进行编码防止XSS攻击

---

## 七、测试规范

[测试要求]
- Date: 2026-06-19
- Context: 用户明确要求测试规范
- Category: 测试规范
- Instructions:
  - 新增功能必须编写单元测试
  - 修改现有功能需要更新相关测试
  - 测试覆盖率应保持在合理水平

[测试类型]
- Date: 2026-06-19
- Context: 用户明确要求测试类型规范
- Category: 测试规范
- Instructions:
  - 单元测试：测试单个函数或方法的功能
  - 集成测试：测试模块间的交互
  - 端到端测试：测试完整的用户流程

[测试命名]
- Date: 2026-06-19
- Context: 用户明确要求测试命名规范
- Category: 测试规范
- Instructions:
  - 测试函数名应该清晰描述测试的场景
  - 使用中文描述测试目的

---

## 八、死代码处理规范

[总体原则]
- Date: 2026-06-19
- Context: 用户明确要求死代码处理规范
- Category: 开发规范
- Instructions:
  - **禁止**使用文件级 `#![allow(dead_code)]` 全局抑制；CI 会在 clippy 检查中失败
  - **禁止**使用 crate 级 `#![allow(unused_imports)]` / `#![allow(unused_variables)]`
  - 真正未使用的项应**显式删除**；保留的项应接入业务或加 `pub` 修饰以表明意图
  - **例外**：backend/src/models/ 下的 SeaORM 自动生成模型可保留文件级 `#![allow(dead_code)]`，原因是模型字段由 SeaORM 派生宏使用，不能手工逐字段标注。**禁止**用此项例外绕过 utils/ 等核心模块的清理

[处理流程]
- Date: 2026-06-19
- Context: 用户明确要求死代码处理流程
- Category: 开发规范
- Instructions:
  1. 编译器/clippy 报告具体 `dead_code` 位置
  2. 评估该项是否仍需要：
     - 需要保留：接入业务、添加 `pub` 或 `pub(crate)` 修饰
     - 不再需要：立即删除（git 会保留历史）
  3. 个别 `pub` API 当前未被业务引用时：
     - 在该项上加 `#[allow(dead_code)]` + TODO 注释
     - 编写计划任务在下一个迭代接入

[TODO 注释标准模板]
- Date: 2026-06-19
- Context: 用户明确要求 TODO 注释标准
- Category: 开发规范
- Instructions:
  - 文件级抑制（仅限业务文件，不再用于 utils/ 等核心模块）：
    ```rust
    #![allow(dead_code)]
    // TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
    ```
  - 项级抑制（推荐）：
    ```rust
    /// 商品全字段查询 DTO（预留 API）
    #[allow(dead_code)] // TODO(tech-debt): 报表模块接入后移除
    pub struct ProductFullDto { ... }
    ```

[CI 强制]
- Date: 2026-06-19
- Context: 用户明确要求 CI 强制检查
- Category: 开发规范
- Instructions:
  - 配置：backend/.clippy.toml `warn` 段开启 `dead_code`/`unused_imports`/`unused_variables`
  - 工作流：.github/workflows/ci-cd.yml `cargo clippy --all-targets -- -D warnings`
  - 任何死代码警告都会让 CI 失败，开发者必须立即处理

---

## 九、版本控制规范

[提交信息]
- Date: 2026-06-19
- Context: 用户明确要求提交信息规范
- Category: 版本控制
- Instructions:
  - 使用有意义的提交信息
  - 提交信息应该描述"做了什么"和"为什么"
  - 使用中文编写提交信息

[分支管理]
- Date: 2026-06-19
- Context: 用户明确要求分支管理规范
- Category: 版本控制
- Instructions:
  - 功能开发使用功能分支
  - 修复bug使用修复分支
  - 保持主分支的稳定性

[代码审查]
- Date: 2026-06-19
- Context: 用户明确要求代码审查规范
- Category: 版本控制
- Instructions:
  - 所有代码变更需要经过审查
  - 审查重点包括：代码质量、安全性、性能、测试覆盖

---

## 十、性能规范

[数据库查询]
- Date: 2026-06-19
- Context: 用户明确要求数据库查询规范
- Category: 性能规范
- Instructions:
  - 优化数据库查询，避免N+1查询问题
  - 使用适当的索引
  - 对大数据量查询进行分页处理

[缓存策略]
- Date: 2026-06-19
- Context: 用户明确要求缓存策略规范
- Category: 性能规范
- Instructions:
  - 合理使用缓存提高性能
  - 明确缓存失效策略
  - 避免缓存过期数据

[资源管理]
- Date: 2026-06-19
- Context: 用户明确要求资源管理规范
- Category: 性能规范
- Instructions:
  - 及时释放不再使用的资源
  - 避免内存泄漏
  - 合理控制并发数量

---

## 十一、错误处理规范

[错误处理原则]
- Date: 2026-06-19
- Context: 用户明确要求错误处理原则
- Category: 开发规范
- Instructions:
  - 所有可能失败的操作都需要错误处理
  - 提供有意义的错误信息
  - 记录错误日志便于调试

[错误分类]
- Date: 2026-06-19
- Context: 用户明确要求错误分类规范
- Category: 开发规范
- Instructions:
  - 业务错误：返回友好的错误提示
  - 系统错误：记录详细日志，返回通用错误信息
  - 验证错误：明确指出验证失败的原因

[错误恢复]
- Date: 2026-06-19
- Context: 用户明确要求错误恢复规范
- Category: 开发规范
- Instructions:
  - 尽可能实现优雅降级
  - 提供重试机制
  - 保持系统稳定性

---

## 十二、文档规范

[API文档]
- Date: 2026-06-19
- Context: 用户明确要求 API 文档规范
- Category: 文档规范
- Instructions:
  - 所有API接口必须有文档说明
  - 文档包括：接口路径、请求参数、响应格式、示例

[代码文档]
- Date: 2026-06-19
- Context: 用户明确要求代码文档规范
- Category: 文档规范
- Instructions:
  - 复杂逻辑必须有注释说明
  - 公共函数必须有文档注释
  - 保持文档与代码同步更新

[用户文档]
- Date: 2026-06-19
- Context: 用户明确要求用户文档规范
- Category: 文档规范
- Instructions:
  - 提供清晰的用户操作指南
  - 包含常见问题解答
  - 定期更新文档内容

---

## 十三、持续改进

[代码重构]
- Date: 2026-06-19
- Context: 用户明确要求代码重构规范
- Category: 开发规范
- Instructions:
  - 定期审查代码质量
  - 及时重构低质量代码
  - 保持代码简洁清晰

[技术债务]
- Date: 2026-06-19
- Context: 用户明确要求技术债务管理
- Category: 开发规范
- Instructions:
  - 记录技术债务
  - 制定偿还计划
  - 避免技术债务积累

[学习成长]
- Date: 2026-06-19
- Context: 用户明确要求学习成长规范
- Category: 工作流协作
- Instructions:
  - 关注新技术发展
  - 定期团队技术分享
  - 持续改进开发流程

---

## 十四、架构设计

[主备隔离设计原则]
- Date: 2026-06-16
- Context: 8 大核心功能主备隔离设计
- Category: 架构设计
- Instructions:
  - **核心约束**: 仅主调用不可用（超时/失败/熔断）时才切换备用
  - **主调用正常时禁用备用**: 避免资源浪费和双写不一致
  - **故障转移后支持回切**: 半开状态探测
  - **关键参数**: 主调用超时 3s、熔断阈值 5 次、熔断时长 30s
  - **8 大功能**: 数据库/缓存/消息队列/文件存储/短信/邮件/搜索/OCR
  - **统一抽象接口**: `FailoverCall<T,E>` trait（异步）
  - **告警规则**: 切换频率 > 5/小时 P2，备用失败率 > 10% P1，熔断持续 > 5 分钟 P1，双不可用 P0
  - **设计文档**: docs/superpowers/reports/2026-06-16-failover-design.md

---

## 十五、行业标准

[行业规则校验结果]
- Date: 2026-06-16
- Context: 纺织行业业务规则联网校验
- Category: 行业标准
- Instructions:
  - **合规度加权平均**: 37/100
  - **GB/T 关键标准**:
    - GB/T 21898-2023 纺织品颜色表示方法
    - GB/T 15608-2006 中国颜色体系（CNCS）
    - GB/T 26377-2022 纺织品颜色标准样品技术规范
    - GB/T 8424.3 纺织品色牢度试验色差计算
    - GB/T 3899.2-2007 染料命名标准色卡
  - **色差行业标准**: ΔECMC ≤ 3
  - **PANTONE**: 行业通用（出口导向企业广泛采用）
  - **当前合规**:
    - 面料分类 85/100、染整工艺 70/100
    - 色号命名 30/100（缺 CNCS/色差/色彩空间）
    - 报价单 0/100、定制订单 0/100
  - **CIELab 色彩空间**: 国际标准颜色表示（需 RGB → Lab 转换）
  - **P0 改进后预期**: 37 → 76（+39 分）

---

## 十六、项目评估

[项目评估结果 2026-06-16]
- Date: 2026-06-16
- Context: 全项目评估（5 维度并行扫描）
- Category: 项目评估
- Instructions:
  - **整体评分**: 72/100（B+ 级）
  - **5 大模块**: 核心业务 28/30、多租户 15/15、权限 12/15、行业 5/20、代码质量 12/20
  - **最大短板**: 行业专属功能（5/20）— 销售报价单 0%、定制订单全流程 0%、色号命名 30%
  - **关键缺口**:
    1. P0: 销售报价单（缺 sales_quotations/quote_items/quote_terms 三表）
    2. P0: 定制订单全流程跟踪（缺 custom_orders/process_nodes/process_logs/quality_issues/after_sales 五表）
    3. P0: 主备隔离（数据库/缓存单点）
  - **代码审计 Top 5 高危文件**:
    1. backend/src/utils/dual_unit_converter.rs (26 expect)
    2. backend/src/services/auth_service.rs (13 expect)
    3. frontend/src/views/system/index.vue (56 any / 1521 行)
    4. frontend/src/views/ap/index.vue (31 any / 1035 行)
  - **健康指标**: 0 panic / 0 unsafe / 0 @ts-ignore / 0 as unknown
  - **改进路线图**:
    - 短期 6 周（目标 80 分）: P0 行业功能 + 主备隔离
    - 中期 6 周（目标 88 分）: P1 行业功能 + 代码清理
    - 长期 24 周（目标 95 分）: 行业标准化 + 智能制造
  - **评估报告位置**: docs/superpowers/reports/2026-06-16-*.md (5 份)
  - **汇总报告位置**: .monkeycode/docs/项目评估报告.md

---

## 十七、2026年编程工作流

[2026年最新编程工作流]
- Date: 2026-05-28
- Context: 用户要求搜索最新的编程工作流，用于真实交付编程项目
- Category: 工作流协作
- Instructions:
  - **CI/CD 自动化流水线**（2026年最佳实践）：
    - 流水线即交付契约（Delivery Contract）
    - 三重角色：自动化引擎、安全闸门、治理平面
    - 多阶段Docker构建、缓存优化、并行测试
    - SAST/DAST安全扫描、SBOM生成、镜像签名
    - OIDC身份认证、RBAC权限控制、不可变标签、自动回滚
  - **DevOps 核心原则**：
    - 流动原则：加速从开发到交付的流程，大需求拆小，工作可视化
    - 反馈原则：及时发现问题，源头保障质量
    - 持续学习原则：将改进制度化，建立学习型组织
  - **Agentic Workflow（智能体工作流）**：
    - AI Agent协同开发，80%重复劳动交给AI
    - 代码生成、单元测试生成、代码审查、日志分析、性能优化
    - AI写代码，人负责审查和修改
  - **开发流程优化**：
    - 敏捷开发：小步快跑，每次迭代交付可审查版本
    - 持续集成：频繁将代码变更合并到共享主干
    - 持续交付：确保代码始终处于可部署状态
    - 基础设施即代码：使用声明式配置管理服务器和环境
  - **质量保障**：
    - 自动化测试：单元测试、集成测试、E2E测试
    - 代码质量门禁：SonarQube、ESLint、Clippy
    - 安全扫描：SAST/DAST、依赖漏洞检查
    - 代码审查：PR审查、AI辅助审查
  - **部署策略**：
    - 蓝绿部署：零停机部署
    - 灰度发布：按比例逐步发布
    - 特性开关：功能级别的发布控制
    - 自动回滚：失败时自动回滚到上一版本
  - **监控与可观测性**：
    - 日志：结构化日志、日志聚合
    - 指标：Prometheus、Grafana
    - 链路追踪：分布式追踪
    - 告警：实时告警、自动恢复

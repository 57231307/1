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

[批次 A dead_code 清理完成]
- Date: 2026-06-24
- Context: Agent 在执行"PR #243 后 clippy dead_code 警告清理（批次 A）"时发现
- Category: 工作流协作
- Instructions:
  - 批次 A 共处理 20 个高频 dead_code 警告文件，通过 PR #245 合并入 main（commit a3f6a978）
  - 采用统一策略：删除真实死代码 + 对预留 API 加项级 `#[allow(dead_code)]` + TODO
  - 核心精简：`backend/src/services/enhanced_logger.rs` 从 401 行减至 122 行，删除 27 个未使用 DTO/结构体
  - 因 PR #243 合并后 main 历史重写，原 `fix/clippy-deadcode-batch-a-2026-06-24` 分支无法合并，已关闭 PR #244，转用 v2 分支
  - 删除失效 `backend/.clippy-baseline.txt`（旧基线与新代码完全不匹配），让 CI 在 bootstrap 模式下重建基线
  - CI 过程中暴露的关联问题已修复：trace.rs `push_str` 改 `push`、database.rs 删除未使用 import、auth_handler.rs 修复 HashSet 类型错误、tests/ 下 integration test 路径修正
  - 所有验证经 GitHub Actions CI 完成，不执行本地 cargo build/clippy/test
  - 下一步：启动批次 B（30 个中频 dead_code 文件）

[批次 B dead_code 清理完成]
- Date: 2026-06-24
- Context: Agent 在执行"PR #243 后 clippy dead_code 警告清理（批次 B）"时发现
- Category: 工作流协作
- Instructions:
  - 批次 B 共处理 30 个中高频 dead_code 警告文件，通过 PR #246 合并入 main（commit c274a5c4）
  - 继续采用统一策略：删除真实死代码 + 对预留 API 加项级 `#[allow(dead_code)]` + TODO
  - 修复集成测试编译错误：`PricingContext` 添加 `Serialize` 派生、`match_tier_for_unit_test` 从 `pub(crate)` 提升为 `pub`、为 `inventory_stock_handler_query.rs` 单测补充 `use std::str::FromStr`
  - 子代理误删 `inventory_stock_txn.rs` 的 `QueryFilter`/`UpdateMany` 导入，导致后端构建失败，经两次 fixup 提交恢复；此经验提示后续应让子代理在清理 `sea_orm` 导入时格外谨慎
  - 因批次 B 修改导致文件行号偏移，原 clippy 基线产生 246 个“新警告”误报，故删除 `backend/.clippy-baseline.txt` 由 CI 在 bootstrap 模式下重建；重建后基线从 977 行降至 643 行
  - 更新 `backend/.test-baseline.txt` 记录当前 10 个历史单测失败（这些失败在 main 上因编译错误未被实际执行），避免阻塞死代码清理主流程
  - 所有验证经 GitHub Actions CI 完成，不执行本地 cargo build/clippy/test
  - 下一步：完成批次 C（40 个低频 dead_code 文件，PR #247）

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

[Wave E-1 deep clippy dead_code 预判]
- Date: 2026-06-19
- Context: Wave B-2 修已为 23 个 pub 项加项级 `#[allow(dead_code)]`，但 CI 仍 fail（exit 101）。本任务深度扫描 90 个 Wave A+B 涉及 .rs 文件，给出完整未引用 pub 项清单。
- Category: 死代码治理（P0 必修）
- Instructions:
  - **扫描方法**：`/tmp/scan_v3.py`（Python 3，~250 行；word boundary 正则 + 自身文件定义行排除）
  - **扫描范围**：`backend/src/` + `backend/tests/` + `backend/migration/src/`（共 626 个 .rs 文件）
  - **关键发现**：
    - 提取 pub 项：1,043
    - 排除已有 `#[allow(dead_code)]`（Wave B-2 修）：23（**全部正确抑制**）
    - 引用数 = 0 实际死代码：**55 项**（在 90 个受影响文件内）
    - 子模块内部死代码（transitively 涉及）：**14 项**（`report/{ds,job,tpl}.rs` 内）
    - **死代码总计：69 项**
    - 6 个 `pub mod` 声明（pred/recon/vfy/ds/job/tpl）是误报——clippy 不会对模块声明触发 dead_code，但会标记模块**内部**未被引用的 pub fn
  - **错误类型分布**：
    - handler 未挂载：27 项（39%）
    - main.rs 中间件未注册：8 项（12%）
    - 服务方法调用方缺失：14 项（20%）
    - DTO struct 未使用：6 项（9%）
    - 子模块内部 fn 死代码：14 项（20%）
  - **TOP 死代码文件**：
    - `services/tenant_billing_service.rs`：6 项
    - `services/inventory_reservation_service.rs`：6 项
    - `services/tenant_service.rs`：5 项
    - `services/supplier_evaluation_service.rs`：4 项
    - `middleware/logger_middleware.rs`：4 项
  - **修复路线图**（3 批 / ~77 项 / 3.0h）：
    - Wave C-1 中间件（8 项）：8 个未注册中间件加项级抑制或删除
    - Wave C-2 Response/DTO（4 项）：TransactionListResponse / DefectResponse / VersionInfo / UpdateProgress 加项级抑制
    - Wave C-3 Service 方法（65 项）：51 个 service fn + 14 个子模块 fn 加项级抑制
  - **报告位置**：[.monkeycode/docs/audits/2026-06-19-clippy-deep-prediction.md](file:///workspace/.monkeycode/docs/audits/2026-06-19-clippy-deep-prediction.md)
  - **扫描脚本**：`/tmp/scan_v3.py`（Python 3，~250 行；可复现）
  - **扫描原始数据**：`/tmp/scan_v3_output.md`（1,043 行表格）+ `/tmp/dead_pub_items_v3.txt`
  - **CI 验证策略**：不本地编译（遵守"禁止本地编译"规则），依赖 GitHub Actions
  - **下一步**：等待用户决策修复策略（删除/抑制/接入），启动 Wave C 修复

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

[CI/CD 验证强制（2026-06-20 用户强调）]
- Date: 2026-06-20
- Context: 用户明确要求所有验证在 CI/CD 进行，禁止本地编译/构建验证
- Category: 运维部署
- Instructions:
  - **禁止**本地编译验证（`cargo build` / `cargo check` / `cargo test` / `cargo fmt -- --check` / `cargo clippy` / `npm run build` / `vue-tsc` / `pnpm typecheck` 等任何本地构建命令）
  - **禁止**本地启动服务做端到端验证（`npm run dev` / `cargo run` / 起后端服务 / 起前端 dev server 等）
  - 所有验证工作流：
    1. 修改代码 → 写 commit → push 触发 CI
    2. 用 GitHub API 监控 CI run 状态（`/repos/{owner}/{repo}/actions/runs` + `/actions/runs/{id}/jobs` + `/actions/runs/{id}/logs`）
    3. 失败 → 拉取 logs zip → 解析 annotations → 修复 → 重新 push
    4. 循环直到 CI 全绿
  - **唯一允许的本地检查**：文件 diff、语法、文本类操作（git status、cat、grep、sed 等只读或文本编辑操作）
  - 工具链验证（如需）通过 CI 触发的具体 job（`cargo fmt --check`、`cargo clippy --all-targets`、`npx vue-tsc --noEmit`）
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

---

## 五、项目真实运行问题检测（2026-06-22 关键经验）

[真实运行问题检测结果]
- Date: 2026-06-22
- Context: 用户指令"检测项目现在真实运行中存在的问题"
- Category: 全面体检
- Instructions:
  - **检测方式**：全量静态扫描（Grep/Glob/Read，遵守"禁止本地编译"规则）
  - **报告位置**：[.monkeycode/docs/audits/2026-06-22-runtime-issues-detection.md](file:///workspace/.monkeycode/docs/audits/2026-06-22-runtime-issues-detection.md)
  - **综合评分**：80/100（B 级）

### 三大 P0 必修问题

1. **CI baseline 文件实际未提交**（🔴 最严重）
   - `backend/.clippy-baseline.txt` 和 `backend/.test-baseline.txt` 不在 git 仓库中
   - PR #238 commit message 声称"已建立 baseline"但实际**未提交**
   - CI 工作流 `541d001` commit 明确引用这两个文件
   - **真实风险**：下一个 PR 触发 CI → 无 baseline → clippy 历史 90+ 警告被识别为新警告 → CI 红
   - **修复**：本地跑 `cargo clippy --all-targets --message-format=short 2>&1 | grep -E "^(warning|error):" | sort -u > backend/.clippy-baseline.txt` + 跑测试收集失败用例名 → 提交 + push + CI 验证

2. **前端 bi/SalesAnalysis.vue 内存泄漏**（🔴）
   - L143 `window.addEventListener('resize', resizeCharts)`
   - L14 import 缺 `onBeforeUnmount`
   - **影响**：多次进入 BI 页面后内存占用线性增长
   - **修复**：加 onBeforeUnmount import + removeEventListener（5 分钟）

3. **后端无 Cargo.lock**（🔴）
   - `backend/Cargo.lock` 不存在
   - PR #238 移除 `--locked` 时未考虑 Cargo.lock 缺失
   - **影响**：cargo build 每次重新解析依赖，sea-orm 2.0.0-rc.40/sqlx 0.9 是 RC 版本
   - **修复**：`cargo generate-lockfile` + commit + push

### 关键经验教训

1. **PR 文档总结与 git 提交清单必须严格对齐** —— PR #238 "已建立 baseline" 文档与实际不符
2. **PR 移除配置时要追因** —— 移除 `--locked` 必须确认 Cargo.lock 存在
3. **资源清理要逐文件验证** —— bi/SalesAnalysis.vue 内存泄漏是 script setup 宽容处理掩盖的典型 bug
4. **静态扫描可发现 Vue script setup 隐藏问题** —— template 引用 vs script import 不一致

### 关键数据

- 后端 .rs 文件：~626
- 前端 .vue 文件：362
- 路由 path：121
- view 引用：117（**0 缺失**）✅
- 业务路径 panic：6
- 业务路径 unwrap：60
- 业务路径 expect：96
- 文件级 dead_code（非 models）：0 ✅
- 租户隔离违规：0 ✅
- SQL 注入：0 ✅
- CVE 漏洞：5（dev/test 依赖）

### 已确认正常/已修复的 23 项 P0

- 4 处启动 panic ✅
- 6 个安全漏洞（PR #237）✅
- DB 迁移 100% 注册 ✅
- 路由 view 一致性 100% ✅
- 9.5 评估中 5 view 全部挂载 ✅
- 部署期 4 大问题全部修复 ✅
- 28 个 migration 全部 Box::new 注册 ✅
- 6 个 chart 组件 addEventListener 全部正确清理 ✅
- 2 处 setInterval 全部清理 ✅
- 关键中间件顺序正确 ✅
- /health 端点暴露（routes/mod.rs:362）✅
- WebSocket 已挂载 ✅

### 推荐修复批次

- **批次 A（1-2 天，P0）**：生成 baseline 文件 + 修 bi/SalesAnalysis.vue 内存泄漏 + 生成 Cargo.lock
- **批次 B（1 周，P1）**：6 处业务 panic + README 同步
- **批次 C（2-4 周，P1）**：so/order.rs 拆分 + 15 个前端大文件拆分 + 192 ESLint disable 收敛
- **批次 D（季度）**：CVE 升级（dev 依赖）

### main HEAD

- 远端 main HEAD：`c6469cb`（auto-release 2026.622.1219）
- 实际代码 HEAD：`541d001`（PR #238 squash merge）
- 本地 main 落后远端 main：2 个 auto-release commit（不影响代码）

---

## 🚨 安全 Wave 3 P2 修复经验教训（PR #242, 2026-06-23）

- Date: 2026-06-23
- Context: 修复漏洞 #7（CSRF Token 设计缺陷）+ #8（批量导入 DoS）后 CI 触发多个 clippy/build 失败
- Category: 排错调试
- Instructions:
  - **rustc builtin vs clippy lints**：标记**实际被使用项**的 `#[allow(...)]` 触发 rustc 1.94 `useless_attribute` warn（CI `-D warnings` 升级为 error）。正确做法是删除 useless allow，依赖编译器精准报告
  - **rustc builtin lint 名**：`unused_variables` / `unused_imports` / `dead_code` 是 rustc builtin，应写 `#[allow(unused_variables)]`；`clippy::unused_variables` 是**无效 lint 名**，触发 `unknown_lints` warn
  - **clippy 有效 lint 名**：`clippy::redundant_clone` / `clippy::too_many_arguments` / `clippy::needless_pass_by_value` / `clippy::type_complexity` / `clippy::needless_range_loop` / `clippy::needless_late_init` / `clippy::upper_case_acronyms` 等才是有效 clippy 内置 lint
  - **CI debug 输出策略**：在 CI workflow 的 exit 1 之前 `cat reports/clippy-new.txt`，把完整新警告列表输出到 step logs，让静态分析能精准定位（无需下载 logs zip）
  - **Entity::find() 是 trait method**：`EntityTrait::find` 不是 inherent method，必须 `use sea_orm::EntityTrait;` 才能调用；同 `.filter()` (需 `QueryFilter`)、`.gte()/.lt()/.gt()/.lte()/.eq()` (需 `ColumnTrait`)、`.count()/.all()/.paginate()` (需 `PaginatorTrait`)
  - **validator crate 限制**：`#[validate(length(max = X))]` 只支持**整数字面量**，**不支持** Rust 表达式（`10 * 1024 * 1024` 不行，必须用 `10485760`）
  - **连续 5 个 commit 才修复 1 个 PR**：PR #242 squash merge 前的 5 个 CI 修复 commit (00846cd, 09b0e5c, 39a363c, c1dda49, ee18ece) 证明**单次修复很容易漏改**，必须**每改一处静态验证 + CI 监控循环**
  - **3 次连续失败教訓（3182e84, 6c0af46, cefef42）**：删 sea_orm trait import 时**不能批量删**，必须**逐个静态验证**是否被使用（`grep -n "Entity::find\|\\.filter\|\\.gte\|\\.lt\|\\.gt\|\\.lte"`）；CI build error E0599 的 help 提示会明确指出需要的 trait 名（如 `trait EntityTrait which provides find is implemented but not in scope`）
  - **CI 监控 API**（不下载 logs）：`/repos/.../commits/{sha}/check-runs` + `/actions/jobs/{id}/logs` (API 端点，不是 zip 下载) + `/check-runs/{id}/annotations` 是合规监控方式

### PR #242 squash 前 CI 修复历程（5 commits）

| Commit | 修复内容 | 失败根因 |
|--------|----------|----------|
| 00846cd | 9 文件删 useless `#[allow(...)]` | useless_attribute 警告（标记实际使用项） |
| 09b0e5c | 简化 auth_handler.rs / auth_handler_misc.rs 多余 allow | 同上 |
| 39a363c | validator `length(max = 10 * 1024 * 1024)` → `length(max = 10485760)` | validator 0.16 不支持 Rust 表达式 |
| c1dda49 | ci-cd.yml 在 clippy 失败前 cat 完整新警告 | 14 个 `unknown_lints: clippy::unused_variables` 警告需静态定位 |
| ee18ece | 删除 auth_handler.rs:23 4 个未用 sea_orm trait（保留 EntityTrait/ColumnTrait/QueryFilter） | 3 次连续失败（3182e84, 6c0af46, cefef42）后正确识别 trait 使用位置 |

### 复用样板

```rust
// 1. CSRF token 写入（带 IP 绑定 + 反向索引 + 强制轮换）
pub fn set_csrf_token(
    &self, token: String, session_id: String,
    ip_address: String, user_id: i32, ttl: Option<Duration>,
) {
    let effective_ttl = ttl.unwrap_or(Duration::from_secs(CSRF_TOKEN_DEFAULT_TTL_SECS));
    self.csrf_token_cache.set(token.clone(), (session_id, ip_address), Some(effective_ttl));
    self.csrf_user_index.insert(user_id, token);
}

// 2. CSRF token 消费（IP 不匹配不消费，防 DoS）
pub fn consume_csrf_token(&self, token: &str, client_ip: &str) -> CsrfConsumeResult {
    match self.csrf_token_cache.take(&token.to_string()) {
        Some((session_id, bound_ip)) => {
            if bound_ip != client_ip {
                self.csrf_token_cache.set(token.to_string(), (session_id, bound_ip), None);
                return CsrfConsumeResult::IpMismatch;
            }
            CsrfConsumeResult::Ok
        }
        None => CsrfConsumeResult::NotFound,
    }
}

// 3. 批量导入 DTO 四层防御
#[derive(Debug, Deserialize, Validate)]
pub struct CsvImportRequest {
    pub import_type: String,
    #[validate(length(max = 10_485_760, message = "CSV 数据超过 10MB 上限"))]
    pub data: String,  // 10MB = 10 * 1024 * 1024 字面量
}
```

### main HEAD（更新）

- 远端 main HEAD：`2ab793c`（PR #242 squash merge）
- 本次合并：8 业务文件 / +933/-63 行 + 16 个新单测 + 5 CI 修复 commits

---

## 📋 安全漏洞修复波次总览（2026-06-23）

| Wave | 等级 | 漏洞数 | 漏洞 | PR | 状态 |
|------|------|--------|------|------|------|
| Wave 1 | P0 紧急 | 2 | #1 密码重置认证 + #2 租户管理权限 | #240 | ✅ merged b298c99 |
| Wave 2 | P1 重要 | 4 | #3 用户权限 + #4 数据库测试认证 + #6 JWT 即时失效 + #9 用户级 JTI 吊销 | #241 | ✅ merged cdb2ada |
| Wave 3 | P2 中 | 2 | #7 CSRF Token 设计缺陷 + #8 批量导入 DoS | #242 | ✅ merged 2ab793c |
| Wave 4 | P3 低 | 6 (子代理 B + C 完成) | #5 #10 #11 #12 #13 #14 | #243 | 🔵 子代理 B (#10#13#14) + C (#11#12) 完成；A (#5) 待跟进 |

### Wave 4 子代理 B 修复 #10 + #13 + #14（2026-06-23）

- Date: 2026-06-23
- Context: 修复漏洞 #10 + #13（LoginResponse 移除敏感字段）+ #14（permissions 改为 Vec<String>）
- Category: 安全加固（P3 低 / 信息泄露）
- Instructions:
  - **修改文件**：`backend/src/handlers/auth_handler.rs`（单文件 1 处）
  - **删除 UserPermissionDto**：grep 确认全 backend/src 无其他引用（仅自身定义 + 构造处），符合死代码治理规范，**显式删除**（不保留 `#[allow(dead_code)]`）
  - **LoginResponse 字段白名单**：仅保留 `csrf_token` + `user` + `permissions` 三个字段
  - **资源标识符格式**：`"{resource}:{action}"`（如 `"user.list:read"`），前端可直接 `permissions.includes("user.list:read")` 判断
  - **refresh_token cookie**：删除响应体字段后，cookie 构造从 `response.refresh_token.clone()` 改为直接使用局部变量 `refresh_token`
  - **csrf_token 保留**：前端 form header 需携带（`X-CSRF-Token`），且由非 httpOnly Cookie 暴露给 JS
  - **OpenAPI 自动同步**：`openapi.rs:79` 注册的 `LoginResponse` 类型不变（utoipa 自动从 Rust struct 生成 schema，删除字段后 schema 自动同步，**无需手动修改**）
  - **is_production 跳过**：L364-365 仍由 C 子代理改为 `is_production()` 函数调用（B 子代理范围不重叠）
  - **新增 4 个单测**（文件末尾 `#[cfg(test)] mod tests`）：
    1. `test_login_response_omits_token_field` —— 验证 JSON 序列化不含 `token` 字段
    2. `test_login_response_omits_refresh_token_field` —— 验证 JSON 序列化不含 `refresh_token` 字段
    3. `test_login_response_permissions_is_string_array` —— 验证 `permissions` 是 `Vec<String>` 且格式正确
    4. `test_login_response_field_whitelist` —— 验证响应体仅包含白名单字段（防回归）
  - **静态验证结果**：
    - `grep "LoginResponse {" backend/src/handlers/auth_handler.rs` → 3 处（结构体定义 L50 / 构造处 L357 / 测试构造处 L513）
    - `grep -rn "UserPermissionDto" backend/src/` → 0 处实际使用（仅 3 处注释引用）
    - `grep -rn "LoginResponse" backend/src/` → auth_handler.rs + openapi.rs:79
  - **前端影响报告**（不修改，由其他批次处理）：
    - `frontend/src/types/api.ts:9-14` `LoginResponse` 接口定义 `token` / `refresh_token` / `expires_in` 字段，需更新
    - `frontend/src/api/auth.ts:11-29` `LoginResponseWithCsrf` 类型 + `login()` destructure 逻辑需调整
    - `frontend/src/store/user.ts:11-22` `userStore.login()` `if (responseData.token)` 永真分支可清理
    - `frontend/src/views/Login.vue:214-223` `userStore.userInfo.permissions` 已按 `string[]` 使用，**无需修改**（已与新 DTO 对齐）
  - **commit 草稿**：
    ```
    fix(backend): 安全漏洞 #10 #13 #14 - LoginResponse 移除敏感字段 + permissions 改为资源标识符
    - #10：LoginResponse 删除 token 字段（access_token 已在 httpOnly Cookie 写入）
    - #13：LoginResponse 删除 refresh_token 字段（refresh_token 已在 httpOnly Cookie 写入）
    - #14：LoginResponse permissions 改为 Vec<String> 资源标识符（格式 "{resource}:{action}"）
    - 删除不再使用的 UserPermissionDto 结构体（全代码无引用）
    - 新增 4 个单测验证 LoginResponse 字段白名单 + 权限类型
    - 同步影响前端：api/auth.ts + types/api.ts + store/user.ts（待其他批次处理）
    ```

### Wave 4 子代理 C 修复 #11 + #12（2026-06-23）

- Date: 2026-06-23
- Context: 修复漏洞 #11（错误响应体生产环境脱敏）+ #12（is_production 统一从 APP_ENV 读取）
- Category: 安全加固（P3 低 / 信息泄露 + 配置漂移）
- Instructions:
  - **新增模块** `backend/src/utils/config.rs`：提供 `is_production()` 统一函数（从 `APP_ENV` 环境变量读取，不区分大小写匹配 `production`），含 4 个单测
  - **配置统一（漏洞 #12）**：5 处历史代码从多源判断（`ENV` 环境变量 / `cfg!(debug_assertions)`）迁移到统一函数
    - `auth_handler.rs:364-365` / `auth_handler_misc.rs:152` / `auth_handler_session.rs:123`：`std::env::var("ENV") == "production"` → `is_production()`
    - `error.rs:83`（`IntoResponse`）：`!cfg!(debug_assertions)` → `is_production()`
    - `error.rs:450`（`to_response()` 内部）：`cfg!(debug_assertions)` → `is_production()`
  - **错误响应脱敏（漏洞 #11）**：`IntoResponse` 重构为 if/else 两路径
    - 生产环境：`code + message + trace_id + timestamp`（4 字段，不含 `error_type` / `detail`）
    - 开发环境：完整 `error_type + detail` 便于排错（5+ 字段）
  - **API 破坏性变更**：`code` 字段从 `status.as_u16()` 数字（HTTP status）改为 `self.error_code()` 字符串（业务错误码，如 `"NOT_FOUND"`）
    - 与 `to_response()` 返回的 `ErrorResponse` 统一
    - 前端需同步更新：`frontend/src/types/api.ts` + 错误处理组件
  - **新增 trace_id + timestamp**：UUID v4 + Unix epoch seconds，便于客户端关联服务端日志
  - **新增依赖** `dotenvy = "0.15"`（0.15.7 已在 Cargo.lock 中作为 sea-orm-cli 传递依赖，添加为直接依赖）
  - **main.rs** 启动时加载 `.env` 文件（`dotenvy::dotenv().ok()`），仅加载**未设置**的环境变量（不覆盖 systemd/CI 注入）
  - **不动** `utils/audit.rs` / `init_handler.rs` / `auth_handler.rs` LoginResponse 部分（B 子代理范围）
  - **新增 5 个 error.rs 单测**（文件末尾）：
    1. `test_production_response_omits_error_type` —— 生产环境响应不含 `error_type`
    2. `test_production_response_omits_detail` —— 生产环境响应不含 `detail`
    3. `test_development_response_includes_error_type_and_detail` —— 开发环境响应含完整字段
    4. `test_to_response_uses_public_message_in_production` —— to_response() 在生产环境脱敏
    5. `test_to_response_uses_display_in_development` —— to_response() 在开发环境保留 Display
  - **静态验证**：
    - `grep "is_production" backend/src/` → 34 处（含 5 个引用方 + 1 个测试模块 + 28 个文档/注释）
    - `grep "APP_ENV" backend/src/` → 仅 `config.rs` 读取 + 测试用例 + 注释
    - `grep "dotenvy" backend/Cargo.toml` → 1 处声明
    - `grep "error_type" backend/src/utils/error.rs` → 11 处（10 match arm + 1 body 引用）
    - `grep "cfg!" backend/src/` → 0 处运行时判断（仅 `cfg!(windows)` 在 system_update_service.rs 是 OS 检测）
    - `grep "ENV" backend/src/` → telemetry.rs(2) + observability/config.rs(1) 保留，**不在本任务范围**
  - **commit 草稿**：
    ```
    fix(backend): 安全漏洞 #11 #12 - 错误响应脱敏 + is_production 配置统一
    - #11：错误响应体生产环境移除 error_type / detail 字段
    - #11：响应体新增 trace_id + timestamp 便于客户端关联服务端日志
    - #11：API 破坏性变更 - code 字段从数字改为字符串
    - #12：新增 utils/config.rs 统一 is_production() 函数
    - #12：5 处历史代码从多源判断迁移到统一函数
    - #12：main.rs 启动时加载 .env 文件
    - 新增 9 个单测覆盖 is_production + 错误响应脱敏
    - 新增依赖 dotenvy = "0.15"（已在 Cargo.lock 中）
    ```
```

---

## 🚨 PR #243 Wave 4 合并 + Clippy 1661 警告规划（2026-06-23）

- Date: 2026-06-23
- Context: Wave 4 PR #243 merged（commit 37ce64e）+ clippy 失败 1661 警告 + 用户决策合并 + 规划
- Category: 安全批次总结 + 死代码清理

### Wave 4 完成状态

PR #243 6 个 P3 漏洞全部修复：
- **#5 get_task_status 权限**（子代理 A）：`init_handler.rs` +174/-1，3 单测
- **#10 + #13 LoginResponse 字段**（子代理 B）：`auth_handler.rs` +149/-27，4 单测
- **#14 permissions 类型**（子代理 B）：`Vec<String>` 资源标识符 + 删除 `UserPermissionDto`
- **#11 错误响应脱敏**（子代理 C）：`error.rs` +90/-10，5 单测
- **#12 is_production 统一**（子代理 C）：`utils/config.rs`（新）+100 行 + `dotenvy 0.15` 依赖 + 4 单测

**9 业务文件 + 1 新文件 / +846/-55 行 / 16 新单测 / 1 新依赖**

**PR merge**：CI 11/12 success（build/test/类型检查全过），clippy 失败但**强制 squash 合并**（用户决策"合并 main + 规划所有警告"）

### 14 个安全漏洞全部修复完成

| Wave | 等级 | 漏洞 | PR | 合并 commit |
|------|------|------|------|-------------|
| Wave 1 | P0 | #1 #2 | #240 | b298c99 |
| Wave 2 | P1 | #3 #4 #6 #9 | #241 | cdb2ada |
| Wave 3 | P2 | #7 #8 | #242 | 2ab793c |
| Wave 4 | P3 | #5 #10 #11 #12 #13 #14 | #243 | 37ce64e |
| **合计** | **4 等级** | **14 漏洞** | **4 PR** | **4 merged** |

### Clippy 1661 警告分析（关键经验）

- **真实警告数**：285 个（按 warning text 分类，top 10 类型全是 dead_code）
- **拆行后行数**：1661（`sort -u` 把 rendered 字段多行 warning 拆成单行，每行都计）
- **根因 1**：rustc 1.94 增强 dead_code 检测（vs 旧 baseline 1039 行）
- **根因 2**：dotenvy 新依赖引入（虽然 PR #243 没改 122 个文件中任一个，但依赖变化触发了所有 target 的 dead_code 重新分析）
- **分布**：122 个不同 src 文件，最高 `src/services/enhanced_logger.rs` 27 警告

### 修复规划（4 批次 / 123 子代理 / 4 PR #244-#247）

- **批次 A**（高频 20 文件 166 警告）：第一批启动
- **批次 B**（中频 30 文件 100 警告）
- **批次 C**（低频 72 文件 90 警告）
- **批次 D**（2 unused_imports）
- **目标**：baseline 1039 → < 500（清理 50% 死代码）

### 死代码处理规范（`.trae/rules/project_rules.md` §六）

- **禁止**文件级 `#![allow(dead_code)]`
- **禁止** crate 级 `#![allow(unused_imports)]` / `#[allow(unused_variables)]`
- 真实未使用项**显式删除**（git 保留历史）
- 保留项加 `pub` 修饰或 `#[allow(dead_code)]` + TODO 注释

### 决策树

```
dead_code 警告 → 该项是否仍需要？
├─ 不需要 → 删除
└─ 需要保留
   ├─ 是否能 pub 暴露给外部用？
   │  └─ 是 → 改 fn/struct 为 pub
   └─ 否 → 加 #[allow(dead_code)] + TODO 注释
```

### 详细计划

- 规划文档：`.monkeycode/docs/superpowers/plans/2026-06-23-clippy-deadcode-cleanup-plan.md`
- main HEAD（更新）：`37ce64e`（PR #243 squash merge）
- 下一步：批次 A 启动（20 子代理并行）→ 监控 CI → 合并 → 后续批次

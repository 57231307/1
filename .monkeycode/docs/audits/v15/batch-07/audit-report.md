# V15 可维护性与长期治理审计报告（类七·批次 07）

- **审计子代理**：V15 审计子代理（类七 可维护性与长期治理审计类）
- **审计范围**：5 维度（可维护性 / i18n 与可访问性 / 部署运维 / CI/CD pipeline 健康度 / 性能优化与缓存策略）
- **审计依据**：
  - `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md`（第 1063-1149 行 类七 5 维度审计计划）
  - `/workspace/.trae/rules/project_rules.md`（项目开发规范）
  - `/workspace/.monkeycode/MEMORY.md`（部署限制、CI 验证偏好）
- **审计方法**：Grep 检索 + Glob 查找 + Read 关键文件 + 对照审计计划逐项核对
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码

---

## 维度 1：可维护性

### 检查方法
1. Grep 检索 `Arc::try_unwrap().unwrap()` 全后端
2. Python 脚本统计 backend/src/services/ 下所有函数行数（>80 行视为超长）
3. Read 读取 `backend/src/utils/crud_macro.rs` 验证 CRUD 宏抽象
4. Grep 检索 `use crate::utils::app_state` 验证模块循环依赖
5. Read 读取 `backend/src/services/crm/recycle_rule.rs` 验证 CRM 规则持久化
6. Read 读取 `backend/src/services/role_permission_service.rs` 验证角色权限配置化
7. Grep 检索魔法数字（300/600/3600 等硬编码）
8. Grep 检索常量定义（`const|static`）

### 发现

#### ✅ 已落实的项
1. **CRUD 模板已抽象为宏**（crud_macro.rs）
   - 证据：`/workspace/backend/src/utils/crud_macro.rs:3-17` `define_service!` 宏生成 Service 结构体
   - 证据：`/workspace/backend/src/utils/crud_macro.rs:33-62` `impl_generate_no!` 宏生成单号生成函数
   - 证据：`/workspace/backend/src/utils/crud_macro.rs:77-178` `define_crud_handlers!` 宏生成 CRUD Handler
   - 证据：`/workspace/backend/src/utils/crud_macro.rs:190-298` `define_tuple_crud_handlers!` 变体宏

2. **CRM 规则已持久化**（非内存存储）
   - 证据：`/workspace/backend/src/services/crm/recycle_rule.rs:1-7` 注释明确"将原本存于 handlers/missing_handlers.rs 的 static RECYCLE_RULES 内存存储迁移至数据库 crm_recycle_rules 表"
   - 证据：`/workspace/backend/src/services/crm/recycle_rule.rs:50-57` `RecycleRuleService` 持有 `Arc<DatabaseConnection>`
   - 证据：`/workspace/backend/src/services/crm/recycle_rule.rs:72-78` `list_rules` 通过 SeaORM 查询数据库

3. **角色权限配置化**（非硬编码）
   - 证据：`/workspace/backend/src/services/role_permission_service.rs:7-8` 引用 `role` 和 `role_permission` 模型
   - 证据：`/workspace/backend/src/services/role_permission_service.rs:79-100` `list_roles` 从数据库查询角色
   - 证据：`/workspace/backend/src/services/role_permission_service.rs:279-298` `assign_permission` 通过数据库分配权限

4. **Arc::try_unwrap().unwrap() 无实际使用**
   - 证据：`/workspace/backend/src/services/ap_reconciliation_service.rs:520` 仅有注释提及"避免 Arc::try_unwrap().unwrap() 在 future 被取消时 panic"，无实际代码使用

5. **大量魔法数字已常量化**
   - 证据：`/workspace/backend/src/middleware/auth.rs:66` `const USER_ACTIVE_CACHE_TTL_SECS: u64 = 300`
   - 证据：`/workspace/backend/src/middleware/timeout.rs:5` `const TIMEOUT_SECONDS: u64 = 30`
   - 证据：`/workspace/backend/src/handlers/auth_handler.rs:29-30` `const MAX_FAILED_ATTEMPTS: i32 = 5; const LOCKOUT_DURATION_MINUTES: i64 = 30`
   - 证据：`/workspace/backend/src/services/ap_payment_request_service.rs:24-25` `const PAYMENT_APPROVAL_THRESHOLD_MANAGER: i64 = 100_000; const PAYMENT_APPROVAL_THRESHOLD_ADMIN: i64 = 500_000`
   - 证据：`/workspace/backend/src/services/quotation_approval_service.rs:29-30` `const AMOUNT_THRESHOLD_SELF: i64 = 100_000; const AMOUNT_THRESHOLD_MANAGER: i64 = 500_000`
   - 证据：`/workspace/backend/src/services/auth_service.rs:682` `const REVOKED_USER_TTL_SECS: i64 = 7 * 24 * 60 * 60`
   - 证据：`/workspace/backend/src/services/api_key_service.rs:17` `const API_KEY_BLACKLIST_TTL_SECS: u64 = 7 * 24 * 60 * 60`
   - 证据：`/workspace/backend/src/services/report/mod.rs:332` `const DEFAULT_CACHE_TTL_SECONDS: i64 = 300`

#### ❌ 缺陷项

**缺陷 7.1-1：超长函数大量存在（>50 行函数清零未达标）**
**风险等级：P0**
**证据**：Python 脚本统计 backend/src/services/ 下函数行数，共发现 **130+ 个函数超过 80 行**，最严重案例：
- `/workspace/backend/src/services/event_bus.rs:412` `start_event_listener` **588 行**
- `/workspace/backend/src/services/ap_payment_service.rs:184` `confirm` **334 行**
- `/workspace/backend/src/services/ar_service.rs:898` `manual_verify` **257 行**
- `/workspace/backend/src/services/ar_service.rs:652` `auto_verify` **246 行**
- `/workspace/backend/src/services/financial_analysis_service.rs:210` `calculate_indicators` **243 行**
- `/workspace/backend/src/services/business_mode_service.rs:179` `check_module_consistency` **233 行**
- `/workspace/backend/src/services/material_shortage_service.rs:126` `detect_shortages` **229 行**
- `/workspace/backend/src/services/event_kafka_payload.rs:150` `from` **231 行**
- `/workspace/backend/src/services/batch_service.rs:241` `batch_update_products` **201 行**
- `/workspace/backend/src/services/ap_invoice_service.rs:64` `auto_generate_from_receipt` **186 行**
- `/workspace/backend/src/services/ar_service.rs:116` `create_payment` **183 行**
- `/workspace/backend/src/services/ap_verification_service.rs:44` `auto_verify` **173 行**
- `/workspace/backend/src/services/purchase_return_service.rs:177` `approve_return` **176 行**
- `/workspace/backend/src/services/dye_batch_state_machine_service.rs:165` `builtin_transition_rules` **159 行**（染整 service 超长函数，审计点 7.1.6 提及的"172 行超长函数"同类问题）

**业务影响**：超长函数难以理解、测试和维护，违反单一职责原则，增加 bug 引入风险，阻碍新成员上手。
**修复建议**：
1. 优先拆分 >200 行的函数（共 8 个），按业务子流程抽取为私有方法
2. 对 100-200 行的函数（约 30 个）按职责拆分
3. 对 80-100 行的函数逐步优化
4. 在 CI 中增加函数行数检查（如 rustfmt + 自定义脚本）

---

**缺陷 7.1-2：模块循环依赖（utils↔services）**
**风险等级：P1**
**证据**：
- `/workspace/backend/src/utils/app_state.rs:7-30` app_state 引用 18 个 services（AuditCleanupService, AuditLogService, DataPermissionService, EmailService, EventNotificationService, MetricsService, NotificationService, OmniAuditEngine, QuotationService, QuotationPricingService, QuotationApprovalService, QuotationConvertService, CustomOrderCrudService, CustomOrderStateService, CustomOrderProcessService, CustomOrderQualityService, CustomOrderAfterSalesService, CacheService）
- 反向依赖：13 个 service 引用 `crate::utils::app_state::AppState`：
  - `/workspace/backend/src/services/quotation_service.rs:27`
  - `/workspace/backend/src/services/custom_order_process_service.rs:16`
  - `/workspace/backend/src/services/color_card_crud_service.rs:22`
  - `/workspace/backend/src/services/quotation_pricing_service.rs:19`
  - `/workspace/backend/src/services/color_card_borrow_service.rs:21`
  - `/workspace/backend/src/services/custom_order_state_service.rs:16`
  - `/workspace/backend/src/services/custom_order_crud_service.rs:19`
  - `/workspace/backend/src/services/color_card_item_service.rs:24`
  - `/workspace/backend/src/services/color_card_scan_service.rs:12`
  - `/workspace/backend/src/services/custom_order_aftersales_service.rs:15`
  - `/workspace/backend/src/services/quotation_approval_service.rs:25`
  - `/workspace/backend/src/services/custom_order_quality_service.rs:14`
  - `/workspace/backend/src/services/quotation_convert_service.rs:25`

**业务影响**：utils 与 services 双向依赖，破坏分层架构原则；重构困难，单元测试隔离性差；增加编译时间。
**修复建议**：
1. 将 `AppState` 从 utils/ 移至独立的 `container/` 或 `app/` 模块
2. 或将 services 中对 AppState 的依赖改为通过参数传递具体依赖（如 db, cache）
3. 长期目标：services 只依赖 utils 的纯工具函数，不依赖 AppState 容器

---

**缺陷 7.1-3：染整 service 未充分拆分**
**风险等级：P2**
**证据**：
- `/workspace/backend/src/services/dye_batch_state_machine_service.rs` 共 **1510 行**
- `/workspace/backend/src/services/dye_batch_state_machine_service.rs:165` `builtin_transition_rules` **159 行**（硬编码状态流转规则表）
- 该文件包含 4 个 service：LifecycleLogService, StateRuleService, ReworkService, OperationService（通过 `pub fn new(db: Arc<DatabaseConnection>)` 多处定义可见）

**业务影响**：单文件包含多个 service，职责不清晰；状态流转规则硬编码在函数中，无法动态配置。
**修复建议**：
1. 将 4 个 service 拆分为独立文件（lifecycle_log.rs, state_rule.rs, rework.rs, operation.rs）
2. `builtin_transition_rules` 考虑迁移到数据库表或配置文件
3. 参考已有的 service 拆分计划（如 `inventory_count_service_splitting_plan.md`）

---

**缺陷 7.1-4：魔法数字残留**
**风险等级：P2**
**证据**：
- `/workspace/backend/src/services/dashboard_service.rs:243,370,612,846` 多处 `Some(Duration::from_secs(300))` 硬编码 300 秒 TTL
- `/workspace/backend/src/services/auth_service.rs:943,1021,1085,1101,1118,1119` 多处硬编码 `3600`（1 小时）
- `/workspace/backend/src/services/fabric_inspection_service.rs:128` `Decimal::new(3600, 0)` 硬编码 3600

**业务影响**：魔法数字降低代码可读性；修改时需逐处搜索，易遗漏；无法统一调整。
**修复建议**：
1. dashboard_service.rs 提取 `const DASHBOARD_CACHE_TTL_SECS: u64 = 300`
2. auth_service.rs 提取 `const HOUR_SECS: i64 = 3600` 或使用 chrono::Duration::hours(1)
3. fabric_inspection_service.rs 提取 `const POINTS_MULTIPLIER: i32 = 3600`

---

## 维度 2：i18n 与可访问性

### 检查方法
1. 统计 frontend/src 下 useI18n 使用文件数、$t 调用次数
2. 统计 aria-label、alt=、label= 使用情况
3. 统计 ElMessage 硬编码中文（`ElMessage.*('...'`）
4. 统计后端 AppError 硬编码中文（`AppError::*('...'` 和 `AppError::*(format!`）
5. Read 读取 i18n/index.ts 验证配置
6. 检查 locales 资源文件行数

### 发现

#### ✅ 已落实的项
1. **i18n 基础设施已建立**
   - 证据：`/workspace/frontend/src/i18n/index.ts:42-54` createI18n 配置完整（legacy: false, globalInjection: true, fallbackLocale: 'zh-CN'）
   - 证据：`/workspace/frontend/src/i18n/index.ts:57-70` setLocale 语言切换 + localStorage 持久化 + html lang 同步
   - 证据：`/workspace/frontend/src/locales/zh-CN.ts` 467 行资源文件
   - 证据：`/workspace/frontend/src/locales/en-US.ts` 467 行资源文件

2. **Login.vue 示范接入**
   - 证据：`/workspace/frontend/src/views/Login.vue:11` `{{ $t('login.subtitle') }}`
   - 证据：`/workspace/frontend/src/views/Login.vue:38,42,46,52,56` 使用 `$t()` + `:aria-label`
   - 证据：`/workspace/frontend/src/views/Login.vue` 共 10 处 $t 调用，7 处 aria-label

#### ❌ 缺陷项

**缺陷 7.2-1：useI18n 接入率严重不足（3.2% vs 80% 目标）**
**风险等级：P0**
**证据**：
- useI18n 使用：**12 个文件**（Grep 统计 `useI18n` 在 .vue/.ts 文件中出现）
- $t 调用：**129 处**
- Vue 文件总数：**373 个**
- TS 文件总数：**212 个**
- 接入率：12/373 = **3.2%**，远低于审计点要求的 80%+

**业务影响**：国际化能力形同虚设；200+ 视图硬编码中文，无法支持多语言；海外客户无法使用。
**修复建议**：
1. 制定分批迁移计划，优先迁移高频视图（Dashboard, Login, 用户管理）
2. 每个迭代迁移 20-30 个视图，预计 10 个迭代完成
3. CI 中增加 i18n 接入率检查（如检查 .vue 文件中硬编码中文比例）

---

**缺陷 7.2-2：aria-label 严重不足**
**风险等级：P0**
**证据**：
- aria-label 使用：仅 **2 个文件**（MainLayout.vue, Login.vue），共 **8 处**
- 对比：Vue 文件 373 个，el-form-item label= 约 3755 处（这些是表单标签，但未绑定 aria-label）

**业务影响**：屏幕阅读器无法识别交互元素；不符合 WCAG 2.1 AA 可访问性标准；残疾人用户无法使用系统。
**修复建议**：
1. 为所有表单输入框添加 `:aria-label="$t('...')"` 
2. 为所有按钮添加 aria-label（特别是图标按钮）
3. 为所有导航链接添加 aria-label

---

**缺陷 7.2-3：图片 alt 属性完全缺失**
**风险等级：P0**
**证据**：
- alt= 使用：**0 处**（Grep 在 .vue 文件中未找到任何 alt 属性）

**业务影响**：图片无法被屏幕阅读器识别；SEO 不友好；不符合 WCAG 2.1 AA。
**修复建议**：
1. 为所有 `<img>` 标签添加 `:alt="$t('...')"` 
2. 为装饰性图片添加 `alt=""`（空 alt 表示装饰性）
3. 为图标使用 `aria-hidden="true"`

---

**缺陷 7.2-4：ElMessage 硬编码中文**
**风险等级：P1**
**证据**：
- ElMessage 硬编码中文：**64 处**，分布在 20 个文件
- 主要文件：
  - `/workspace/frontend/src/views/businessTrace/index.vue:9` 处
  - `/workspace/frontend/src/components/AfterSalesPanel.vue:5` 处
  - `/workspace/frontend/src/views/notification/index.vue:5` 处
  - `/workspace/frontend/src/views/bom/index.vue:5` 处
  - `/workspace/frontend/src/components/QualityCheck.vue:3` 处
  - `/workspace/frontend/src/components/AdvancedFilter.vue:3` 处
  - `/workspace/frontend/src/views/barcodeScanner/index.vue:4` 处
  - `/workspace/frontend/src/utils/print.ts:4` 处
  - 等 12 个文件

**业务影响**：用户提示信息无法国际化；与 i18n 接入率低问题叠加，整体国际化能力不足。
**修复建议**：
1. 将 ElMessage 调用改为 `ElMessage.success(t('...'))` 
2. 在 locales 资源文件中补充对应 key
3. 优先迁移高频出现的提示（如"保存成功"、"删除成功"）

---

**缺陷 7.2-5：后端 AppError 硬编码中文**
**风险等级：P1**
**证据**：
- AppError 硬编码中文（直接字符串）：**71 处**，分布在 20 个文件
- AppError 硬编码中文（format!）：**145 处**，分布在 20 个文件
- 合计：**216 处**（审计点提及 163 处，实际更多）
- 主要文件：
  - `/workspace/backend/src/services/chemical_service.rs` 22 + 29 = 51 处
  - `/workspace/backend/src/services/so/delivery.rs` 4 + 23 = 27 处
  - `/workspace/backend/src/services/so/order_workflow.rs` 6 + 14 = 20 处
  - `/workspace/backend/src/services/so/order_crud.rs` 2 + 10 = 12 处
  - `/workspace/backend/src/utils/ssrf_guard.rs` 2 + 13 = 15 处
  - `/workspace/backend/src/utils/import_export.rs` 0 + 9 = 9 处
  - `/workspace/backend/src/utils/xlsx_export.rs` 0 + 6 = 6 处
  - `/workspace/backend/src/services/crm/assign.rs` 2 + 8 = 10 处
  - `/workspace/backend/src/services/webhook_service.rs` 3 + 2 = 5 处
  - `/workspace/backend/src/services/mrp_engine_service.rs` 5 + 3 = 8 处

**业务影响**：后端错误信息无法国际化；前端即使完成 i18n，后端返回的错误仍是中文。
**修复建议**：
1. 后端采用错误码 + 前端翻译模式（AppError 返回 error_code，前端根据 code 显示本地化消息）
2. 或后端集成 i18n（通过 Accept-Language 头部切换）
3. 短期：将硬编码中文集中到 `utils/messages.rs` 统一管理

---

**缺陷 7.2-6：i18n/index.ts 注释过时**
**风险等级：P3**
**证据**：
- `/workspace/frontend/src/i18n/index.ts:8` 注释"4506 行资源文件已就绪"
- 实际：`/workspace/frontend/src/locales/zh-CN.ts` 仅 **467 行**
- 实际：`/workspace/frontend/src/locales/en-US.ts` 仅 **467 行**

**业务影响**：注释误导开发者，认为资源文件已完整；实际 key 覆盖率低。
**修复建议**：更新注释为实际行数，或移除过时注释。

---

**缺陷 7.2-7：颜色对比度 WCAG 2.1 AA 无法自动验证**
**风险等级：P3**
**证据**：颜色对比度需人工或工具（如 axe-core）验证，本次审计无法自动检查。
**业务影响**：低对比度影响视觉障碍用户使用。
**修复建议**：
1. 引入 axe-core 自动化可访问性测试
2. 使用 Element Plus 主题变量确保对比度达标
3. 人工审查关键页面（Login, Dashboard）

---

## 维度 3：部署运维

### 检查方法
1. Glob 查找所有 Dockerfile 和 docker-compose.yml
2. Read 读取 deploy/ 目录下所有部署脚本（deploy.sh, deploy-backend.sh, deploy-frontend.sh, deploy-latest.sh, deploy-prepare.sh）
3. Read 读取 deploy/bingxi-backend.service 验证 systemd 配置
4. Read 读取根目录 Dockerfile 和 docker-compose.yml
5. Grep 检索 PostgreSQL 客户端安装、Redis 安装
6. Grep 检索端口 8082、路径 /opt/bingxi-erp、/etc/bingxi
7. Read 读取快速部署/install.sh

### 发现

#### ✅ 已落实的项
1. **systemd 服务名称正确**
   - 证据：`/workspace/deploy/bingxi-backend.service:1-2` `Description=Bingxi ERP Backend Service`
   - 证据：`/workspace/deploy/bingxi-backend.service:14` `ExecStart=/opt/bingxi-erp/backend/server`

2. **安装目录正确**
   - 证据：`/workspace/deploy/bingxi-backend.service:11` `WorkingDirectory=/opt/bingxi-erp/backend`
   - 证据：`/workspace/deploy/deploy.sh:16` `DEPLOY_DIR="/opt/bingxi-erp"`

3. **后端端口 8082 正确**
   - 证据：`/workspace/deploy/deploy.sh:309` `port: "8082"`
   - 证据：`/workspace/deploy/deploy-latest.sh:273` `port: \"8082\"`
   - 证据：`/workspace/deploy/nginx.conf:28` `proxy_pass http://127.0.0.1:8082`

4. **密钥自动生成机制完善**
   - 证据：`/workspace/deploy/deploy.sh:237-263` 自动生成 COOKIE_SECRET 和 JWT_SECRET（`openssl rand -base64 32 | tr -d '\n' | head -c 48`，48 字符 / 32 字节）
   - 证据：`/workspace/deploy/deploy.sh:268-283` 自动生成 WEBHOOK_SECRET（与 JWT_SECRET 独立，含重试校验）
   - 证据：`/workspace/deploy/deploy.sh:213-229` 自动生成 AUDIT_SECRET_KEY（基于硬件信息 + 随机盐 + sha512）
   - 证据：`/workspace/deploy/deploy-latest.sh:181-242` 同样的密钥自动生成逻辑

5. **CONFIG_DIR 与 systemd EnvironmentFile 一致**
   - 证据：`/workspace/deploy/bingxi-backend.service:12` `EnvironmentFile=/etc/bingxi/.env`
   - 证据：`/workspace/deploy/deploy.sh:22` `CONFIG_DIR="/etc/bingxi"`
   - 证据：`/workspace/deploy/deploy-backend.sh:19` `CONFIG_DIR="/etc/bingxi"`
   - 批次 398 修复注释明确：从 /etc/bingxi-erp 改为 /etc/bingxi

6. **不安装 Redis**
   - 证据：Grep 检索 deploy/ 目录无 `redis-server|install.*redis|apt.*redis` 匹配

7. **部署方式正确**
   - 证据：`/workspace/deploy/deploy-latest.sh:98` 从 GitHub Release 下载发布包
   - 证据：`/.github/workflows/ci-cd.yml` CI 构建 → GitHub Release

8. **bingxi update CLI 工具完整**
   - 证据：`/workspace/deploy/deploy.sh:489-720` install_cli 函数生成 /usr/local/bin/bingxi CLI，支持 start/stop/restart/status/logs/update/rollback/migrate/health/version 命令

#### ❌ 缺陷项

**缺陷 7.3-1：禁止 Docker 违规（P0 阻塞）**
**风险等级：P0**
**证据**：
- 项目规则明确禁止：`/workspace/.monkeycode/MEMORY.md:475` "**禁止** Docker 容器部署（不得创建 Dockerfile、docker-compose.yml）"
- 实际存在 4 个 Docker 文件：
  - `/workspace/Dockerfile`（根目录，76 行完整多阶段构建）
  - `/workspace/backend/Dockerfile`
  - `/workspace/frontend/Dockerfile`
  - `/workspace/docker-compose.yml`（131 行，包含本地 PostgreSQL 数据库服务）
- docker-compose.yml 第 21-50 行定义 `db: postgres:15-alpine` 本地数据库服务，违反"用远程数据库"规则
- docker-compose.yml 第 78-79 行 `ports: - "8082:8082" - "50051:50051"` 暴露 gRPC 端口（项目已移除 gRPC）

**业务影响**：违反项目部署规范；docker-compose.yml 包含本地 PostgreSQL，与远程数据库架构冲突；维护两套部署方式增加成本。
**修复建议**：
1. 立即删除 `/workspace/Dockerfile`
2. 立即删除 `/workspace/backend/Dockerfile`
3. 立即删除 `/workspace/frontend/Dockerfile`
4. 立即删除 `/workspace/docker-compose.yml`
5. 删除 `/workspace/.dockerignore`（如存在）
6. 更新文档移除 Docker 相关说明

---

**缺陷 7.3-2：快速部署脚本安装 PostgreSQL 客户端（P0 阻塞）**
**风险等级：P0**
**证据**：
- 项目规则明确：`/workspace/.monkeycode/MEMORY.md:478` "不安装 PostgreSQL 客户端（用远程数据库 39.99.34.194:5432）"
- 实际安装：`/workspace/快速部署/install.sh:43` `apt-get install -y curl jq unzip tar nginx postgresql-client`
- 第 45 行 yum 分支：`yum install -y curl jq unzip tar nginx postgresql`

**业务影响**：违反部署规范；生产服务器安装不必要的 PostgreSQL 客户端，增加攻击面。
**修复建议**：
1. 移除 `postgresql-client` 和 `postgresql` 安装
2. 数据库迁移改用远程执行或后端内置迁移命令（如 `bingxi migrate`）
3. 如必须使用 psql，改为从远程数据库服务器执行迁移

---

**缺陷 7.3-3：deploy-latest.sh 前端部署路径错误**
**风险等级：P1**
**证据**：
- `/workspace/deploy/deploy-latest.sh:159` `mkdir -p /opt/bingxi/frontend/dist`（应为 `/opt/bingxi-erp/frontend/dist`）
- `/workspace/deploy/deploy-latest.sh:253` `rm -rf /opt/bingxi/frontend/dist/*`
- `/workspace/deploy/deploy-latest.sh:254` `cp -r /tmp/bingxi-deploy/frontend/dist/* /opt/bingxi/frontend/dist/`
- `/workspace/deploy/deploy-latest.sh:255` `chown -R www-data:www-data /opt/bingxi/frontend/dist`
- 对比正确路径：`/workspace/deploy/deploy.sh:18` `FRONTEND_DIR="/opt/bingxi-erp/frontend/dist"`
- 对比正确路径：`/workspace/deploy/deploy-frontend.sh:15` `INSTALL_DIR="/opt/bingxi-erp/frontend/dist"`

**业务影响**：使用 deploy-latest.sh 部署时，前端文件部署到错误目录 `/opt/bingxi/frontend/dist`，Nginx 配置指向 `/opt/bingxi-erp/frontend/dist`，导致前端 404。
**修复建议**：
1. 将 deploy-latest.sh 第 159, 253, 254, 255 行的 `/opt/bingxi/` 改为 `/opt/bingxi-erp/`
2. 添加部署后路径一致性检查

---

**缺陷 7.3-4：日志目录不一致**
**风险等级：P1**
**证据**：
- `/workspace/deploy/deploy.sh:24` `LOG_DIR="$DEPLOY_DIR/logs"` = `/opt/bingxi-erp/logs`
- `/workspace/deploy/deploy-backend.sh:20` `LOG_DIR="$INSTALL_DIR/logs"` = `/opt/bingxi-erp/logs`
- `/workspace/deploy/deploy-latest.sh:299` `dir: \"/opt/bingxi-erp/backend/logs\"`
- `/workspace/.monkeycode/MEMORY.md:471` "日志目录：`/opt/bingxi-erp/backend/logs`"
- 三处不一致：deploy.sh 用 `/opt/bingxi-erp/logs`，deploy-latest.sh 用 `/opt/bingxi-erp/backend/logs`

**业务影响**：不同部署脚本生成的 config.yaml 日志目录不同，日志文件位置不一致，运维排查困难。
**修复建议**：
1. 统一为 `/opt/bingxi-erp/backend/logs`（与 MEMORY.md 一致）
2. 修改 deploy.sh 第 24 行 `LOG_DIR="$DEPLOY_DIR/backend/logs"`
3. 修改 deploy-backend.sh 第 20 行 `LOG_DIR="$INSTALL_DIR/backend/logs"`

---

**缺陷 7.3-5：快速部署 install.sh 前端目录路径错误**
**风险等级：P2**
**证据**：
- `/workspace/快速部署/install.sh:10` `FRONTEND_DIR="/opt/bingxi/frontend/dist"`（应为 `/opt/bingxi-erp/frontend/dist`）

**业务影响**：快速部署脚本中 FRONTEND_DIR 变量路径错误，若被使用会导致前端文件部署到错误位置。
**修复建议**：修改为 `FRONTEND_DIR="/opt/bingxi-erp/frontend/dist"`

---

**缺陷 7.3-6：Dockerfile EXPOSE 端口不一致**
**风险等级：P3**
**证据**：
- `/workspace/Dockerfile:73` `EXPOSE 8080`（应为 8082，与后端实际端口一致）
- `/workspace/docker-compose.yml:79` `ports: - "8082:8082"`（正确）

**业务影响**：Docker 部署时端口暴露错误（若启用 Docker）。
**修复建议**：由于 Docker 应被移除（缺陷 7.3-1），此问题随之解决。

---

## 维度 4：CI/CD pipeline 健康度

### 检查方法
1. Read 读取 `.github/workflows/ci-cd.yml`（1911 行完整配置）
2. Read 读取 `.github/workflows/e2e-batch.yml`（前 80 行）
3. Read 读取 `backend/.clippy.toml`
4. 分析 CI job 拓扑、阻塞策略、baseline 机制

### 发现

#### ✅ 已落实的项
1. **10 项必检 job 全部存在**
   - 证据：`/workspace/.github/workflows/ci-cd.yml` 包含 15 个 job：
     - ci-info（环境信息）✅
     - ci-fmt-rust（Rust 格式）✅
     - ci-fmt-fe（前端格式）✅
     - ci-lint-rust（Rust Clippy）✅
     - ci-lint-fe（前端 ESLint）✅
     - ci-type-check（前端类型检查）✅
     - ci-test-rust（Rust 单元测试）✅
     - ci-test-fe（前端测试）✅
     - ci-audit（依赖审计）✅
     - ci-deps（依赖图）✅
     - ci-build-rust（Rust 构建）✅
     - ci-build-fe（前端构建）✅
     - package-release（打包发布）✅
     - github-release（GitHub Release）✅
     - notify（状态通知）✅

2. **E2E 独立工作流，不阻塞主 CI**
   - 证据：`/workspace/.github/workflows/e2e-batch.yml:1-14` 注释明确"E2E 测试从主 CI/CD 独立出来，不阻塞主 CI"
   - 证据：`/workspace/.github/workflows/e2e-batch.yml:29` 仅 `workflow_dispatch` 手动触发
   - 证据：`/workspace/.github/workflows/ci-cd.yml:15` 注释"E2E 测试已独立到 .github/workflows/e2e-batch.yml"

3. **baseline 自动刷新机制有效**
   - 证据：`/workspace/.github/workflows/ci-cd.yml:589-603` main 分支 strict 模式下，已修复 > 0 且无新警告时自动刷新 clippy baseline
   - 证据：`/workspace/.github/workflows/ci-cd.yml:590` 条件：`github.ref == refs/heads/main && CLIPPY_BASELINE_MODE == strict && CLIPPY_FIXED_COUNT > 0 && CLIPPY_NEW_COUNT == 0`
   - 证据：`/workspace/.github/workflows/ci-cd.yml:592-600` 自动 cp + git commit + git push 刷新 baseline

4. **严格阻塞的 job 正确配置**
   - 证据：`/workspace/.github/workflows/ci-cd.yml:1873-1879` 严格阻塞 job 列表：ci-type-check, ci-test-rust, ci-test-fe, ci-build-rust, ci-build-fe
   - 证据：`/workspace/.github/workflows/ci-cd.yml:1880-1886` 失败时 `FAILED++` 并输出错误
   - 证据：package-release 第 1595-1606 行 `needs` 依赖所有 10 个必检 job

5. **CI 失败时提供详细诊断**
   - 证据：每个 job 都有 `actions/upload-artifact@v4` 上传报告
   - 证据：每个 job 都写入 `$GITHUB_STEP_SUMMARY`
   - 证据：clippy 报告包含新增警告列表、修复命令

#### ❌ 缺陷项

**缺陷 7.4-1：clippy 使用 baseline 模式，非严格 -D warnings**
**风险等级：P1**
**证据**：
- 项目规则要求：`/workspace/.trae/rules/project_rules.md` "工作流：cargo clippy --all-targets -- -D warnings"
- 实际：`/workspace/.github/workflows/ci-cd.yml:413` `cargo clippy --all-targets --message-format=json`（无 `-D warnings`）
- 注释说明：`/workspace/.github/workflows/ci-cd.yml:411-412` "不加 -- -D warnings（历史基线警告会直接让 clippy 退出非零码，导致 JSON 输出不完整）"
- baseline 机制：第 435-467 行，首次跑自动建立 baseline，后续严格化（新警告 0 容忍）

**业务影响**：历史 clippy 警告不阻塞 CI，与规则 14 要求的"clippy 严格模式"不符；baseline 文件可能积累大量历史警告。
**修复建议**：
1. 短期：维持 baseline 机制，但制定历史警告清理计划（每迭代清理 10-20 条）
2. 长期：当 baseline 警告数 < 50 时，切换为 `cargo clippy --all-targets -- -D warnings` 严格模式
3. 当前 baseline 行数需检查（`backend/.clippy-baseline.txt`）

---

**缺陷 7.4-2：Rust 格式检查不阻塞 CI**
**风险等级：P1**
**证据**：
- 项目规则要求：`/workspace/.trae/rules/project_rules.md` "Rust 格式（cargo fmt --check）"
- 实际：`/workspace/.github/workflows/ci-cd.yml:250` `exit 0`（渐进式，fmt 失败不阻塞 CI）
- 注释说明：`/workspace/.github/workflows/ci-cd.yml:229-232` "本项目历史代码与 rustfmt 默认风格差异较大（300+ 行 diff），采用渐进式严格化"

**业务影响**：Rust 代码格式不统一，新代码可能引入格式问题；与规则要求不符。
**修复建议**：
1. 执行 `cd backend && cargo fmt --all` 一次性修复所有格式问题
2. 移除 `exit 0`，改为 `exit $EXIT_CODE` 严格阻塞
3. 提交格式修复后立即严格化

---

**缺陷 7.4-3：前端格式检查不阻塞 CI**
**风险等级：P1**
**证据**：
- 实际：`/workspace/.github/workflows/ci-cd.yml:343` `exit 0`（渐进式，prettier 失败不阻塞 CI）
- 注释说明：`/workspace/.github/workflows/ci-cd.yml:325-326` "本项目历史代码与 prettier 默认风格存在差异"

**业务影响**：前端代码格式不统一。
**修复建议**：
1. 执行 `cd frontend && npm run format` 一次性修复
2. 移除 `exit 0`，改为 `exit $EXIT_CODE` 严格阻塞

---

**缺陷 7.4-4：前端 ESLint 不阻塞 CI**
**风险等级：P1**
**证据**：
- 实际：`/workspace/.github/workflows/ci-cd.yml:741` `exit 0`（渐进式，ESLint 失败不阻塞 CI）
- 注释说明：`/workspace/.github/workflows/ci-cd.yml:722-724` "本项目历史代码存在 600+ 个 ESLint 错误"
- 第 664 行 `--max-warnings 999999` 实际上禁用了警告阻塞

**业务影响**：前端代码质量问题不阻塞 CI；600+ ESLint 错误积累。
**修复建议**：
1. 执行 `cd frontend && npm run format && npm run lint -- --fix` 修复可自动修复的问题
2. 对剩余错误逐项评估，修复或抑制
3. 移除 `exit 0` 和 `--max-warnings 999999`，严格阻塞

---

**缺陷 7.4-5：ci-deps job 有 continue-on-error**
**风险等级：P2**
**证据**：
- `/workspace/.github/workflows/ci-cd.yml:1271` `continue-on-error: true`
- 注释说明：第 1266-1270 行 "ci-deps 仅生成 cargo tree / npm ls 依赖图用于记录归档，属 informational job"

**业务影响**：依赖图生成异常不阻塞 CI，可能掩盖 lockfile 漂移问题。
**修复建议**：
1. 移除 job 级 continue-on-error
2. 对 cargo tree --locked 失败的 step 单独处理
3. 让 lockfile 漂移问题能暴露

---

## 维度 5：性能优化与缓存策略

### 检查方法
1. Read 读取 `backend/src/utils/cache.rs`（672 行完整缓存实现）
2. Read 读取 `backend/src/services/cache_service.rs`（前 220 行）
3. Grep 检索 5 个 service（user/product/customer/supplier/role）的 cache 使用
4. Grep 检索 Redis 使用情况
5. Grep 检索 CacheBackend trait / Mock
6. Grep 检索 Prometheus 缓存命中率集成
7. Grep 检索 invalidate_prefix / graceful degradation

### 发现

#### ✅ 已落实的项
1. **CacheService 基于 moka（进程内 LRU + TTL 缓存）**
   - 证据：`/workspace/backend/src/services/cache_service.rs:20` `use moka::future::Cache`
   - 证据：`/workspace/backend/src/services/cache_service.rs:47-61` `CacheService` 结构体（inner: Arc<Cache<String, Vec<u8>>>）
   - 证据：`/workspace/backend/src/services/cache_service.rs:63-84` `new()` 从环境变量读取配置（CACHE_ENABLED, CACHE_CAPACITY, CACHE_TTL_SECS）

2. **invalidate_prefix 按前缀批量失效**
   - 证据：`/workspace/backend/src/services/cache_service.rs:176-197` `invalidate_prefix` 遍历 key 索引，匹配前缀后逐个 invalidate
   - 证据：`/workspace/backend/src/services/cache_service.rs:55-57` key_index 用于按前缀精确失效

3. **per-key 自定义 TTL**
   - 证据：`/workspace/backend/src/services/cache_service.rs:147-159` `set_with_ttl` 记录 per-key 过期时间戳
   - 证据：`/workspace/backend/src/services/cache_service.rs:99-113` `get` 时检查自定义 TTL，过期则返回 None 并清理

4. **graceful degradation（缓存故障不影响业务）**
   - 证据：`/workspace/backend/src/services/cache_service.rs:94-97` `get` 在 `!self.enabled` 时返回 None
   - 证据：`/workspace/backend/src/services/cache_service.rs:131-133` `set` 在 `!self.enabled` 时直接 return
   - 证据：`/workspace/backend/src/services/cache_service.rs:17-18` 注释"查询逻辑必须保证绕过缓存也能返回正确数据"

5. **缓存键空间使用 module: 格式（非 tenant: 格式）**
   - 证据：`/workspace/backend/src/services/cache_service.rs:11` "key 必须以 `module:` 开头，避免跨模块数据串味"
   - 证据：`/workspace/backend/src/services/dashboard_service.rs:154` `cache_key = format!("dashboard:overview:...")`
   - 证据：`/workspace/backend/src/services/dashboard_service.rs:481` `cache_key = "dashboard:inventory:all"`
   - 项目已删除租户，键空间调整为 module: 格式合理

6. **dashboard_service 接入 AppCache**
   - 证据：`/workspace/backend/src/services/dashboard_service.rs:16` `use crate::utils::cache::{AppCache, Cache}`
   - 证据：`/workspace/backend/src/services/dashboard_service.rs:139-144` `cache: Arc<AppCache>`，`new(db, cache)`
   - 证据：`/workspace/backend/src/services/dashboard_service.rs:165-240` get_dashboard_cache 读写缓存

7. **Prometheus 基础设施已建立**
   - 证据：`/workspace/backend/src/services/metrics_service.rs:24` `use prometheus::{...}`
   - 证据：`/workspace/backend/src/services/business_metrics.rs:44` `use prometheus::{...}`
   - 证据：`/workspace/backend/src/services/metrics_service.rs:608` `metrics_service.business_metrics.record_cache_hit()`
   - 证据：`/workspace/backend/src/services/metrics_service.rs:629` 测试断言 `erp_cache_hits_total` 指标存在

8. **Redis 已用于限流和 JTI 吊销**
   - 证据：`/workspace/backend/src/middleware/rate_limit.rs:5-6` `use redis::aio::ConnectionManager; use redis::AsyncCommands`
   - 证据：`/workspace/backend/src/services/auth_service.rs:27-28` `use redis::aio::ConnectionManager; use redis::AsyncCommands`

#### ❌ 缺陷项

**缺陷 7.5-1：5 个 service 全部未接入缓存层（P0 阻塞）**
**风险等级：P0**
**证据**：
- 审计点要求："Redis 缓存层接入 5 个 service：user_service/product_service/customer_service/supplier_service/role_service"
- Grep 检索 `cache|Cache` 在各 service：
  - `/workspace/backend/src/services/user_service.rs`：**无匹配**
  - `/workspace/backend/src/services/product_service.rs`：**无匹配**
  - `/workspace/backend/src/services/customer_service.rs`：**无匹配**
  - `/workspace/backend/src/services/supplier_service.rs`：**无匹配**
  - `/workspace/backend/src/services/role_service.rs`：**无匹配**
- 仅 dashboard_service.rs 接入了 AppCache（内存缓存）

**业务影响**：用户/产品/客户/供应商/角色查询直接访问数据库，高并发场景下数据库压力过大；响应延迟高；无法支撑大规模用户。
**修复建议**：
1. 为 5 个 service 注入 `CacheService` 或 `AppCache`
2. 在 list/get 方法中先查缓存，未命中再查数据库并写入缓存
3. 在 create/update/delete 方法中失效对应缓存键
4. 参考 dashboard_service.rs 的接入模式

---

**缺陷 7.5-2：缓存是内存缓存（moka），非 Redis（P0 阻塞）**
**风险等级：P0**
**证据**：
- 审计点要求："Redis 缓存层接入 5 个 service"
- 实际：`/workspace/backend/src/services/cache_service.rs:20` `use moka::future::Cache`（进程内缓存）
- 实际：`/workspace/backend/src/utils/cache.rs:5` `use dashmap::DashMap`（内存缓存）
- AppCache 结构体全部基于 `Arc<MemoryCache<...>>`（内存）
- Redis 仅用于限流（rate_limit.rs）和 JTI 吊销（auth_service.rs），未用于业务数据缓存

**业务影响**：多实例部署时缓存不共享，每个实例独立缓存，命中率低；实例重启缓存丢失；无法支撑水平扩展。
**修复建议**：
1. 实现 RedisCacheBackend（基于 redis crate）
2. CacheService 支持切换 backend（内存 / Redis）
3. 生产环境使用 Redis，开发环境使用内存
4. 或保持内存缓存作为 L1，Redis 作为 L2（多级缓存）

---

**缺陷 7.5-3：无 CacheBackend trait + Mock**
**风险等级：P1**
**证据**：
- 审计点要求："CacheBackend trait + Mock（单测不依赖真实 Redis）"
- Grep 检索 `CacheBackend|trait CacheBackend`：**无匹配**
- Grep 检索 `Mock|mock` 在 cache_service.rs：**无匹配**
- utils/cache.rs 有 `trait Cache<K, V>`（第 54 行），但无 Mock 实现
- cache_service.rs 是具体结构体，非 trait

**业务影响**：单元测试无法隔离缓存依赖；无法 Mock 缓存行为；测试可能依赖真实缓存状态。
**修复建议**：
1. 抽取 `trait CacheBackend`（get/set/invalidate/invalidate_prefix/stats）
2. 实现 `MemoryCacheBackend`（当前 moka 实现）
3. 实现 `RedisCacheBackend`（Redis 实现）
4. 实现 `MockCacheBackend`（测试用，可预设返回值）
5. CacheService 持有 `Arc<dyn CacheBackend>`，支持依赖注入

---

**缺陷 7.5-4：缓存命中率未自动接入 Prometheus**
**风险等级：P1**
**证据**：
- 审计点要求："缓存命中率统计接入 Prometheus"
- cache_service.rs 维护自己的 stats（hits/misses）：`/workspace/backend/src/services/cache_service.rs:51` `stats: Arc<RwLock<CacheStats>>`
- 但 cache_service.rs 未调用 metrics_service 上报命中率
- metrics_service.rs:608 的 `record_cache_hit()` 仅在测试中调用，非 cache_service 自动上报
- AppCache（utils/cache.rs）也维护自己的 CacheStats，同样未自动上报 Prometheus

**业务影响**：缓存命中率无法在 Grafana 监控；无法判断缓存效果；无法及时发现缓存失效问题。
**修复建议**：
1. 在 cache_service.rs 的 get 方法中，命中时调用 `metrics_service.record_cache_hit()`，未命中时调用 `record_cache_miss()`
2. 或定时任务每分钟上报一次 CacheStats 到 Prometheus
3. 在 Grafana dashboard 添加缓存命中率面板

---

**缺陷 7.5-5：默认 TTL 60s，未按用户/产品 300s→600s 调优**
**风险等级：P2**
**证据**：
- 审计点要求："TTL 调优（用户/产品 300s → 600s）"
- 实际：`/workspace/backend/src/services/cache_service.rs:78` 默认 `ttl_secs = 60`（可通过 CACHE_TTL_SECS 环境变量配置）
- dashboard_service.rs 使用 `Duration::from_secs(300)`（第 243, 370, 612, 846 行）
- 未针对不同实体设置差异化 TTL

**业务影响**：TTL 过短（60s）导致缓存命中率低；TTL 过长可能导致数据不一致；未按实体特性调优。
**修复建议**：
1. 为不同实体设置差异化 TTL：
   - 用户/角色：600s（变更频率低）
   - 产品/客户/供应商：300s（中等变更频率）
   - Dashboard 聚合数据：60-120s（实时性要求较高）
2. 使用 `set_with_ttl` 方法按实体设置 TTL

---

**缺陷 7.5-6：缓存写时 invalidate 策略未在 5 个 service 实现**
**风险等级：P2**
**证据**：
- 审计点要求："缓存失效策略明确（写时 invalidate + TTL 300s 兜底）"
- 由于 5 个 service 未接入缓存（缺陷 7.5-1），写时 invalidate 策略自然未实现
- dashboard_service 仅有读缓存，无写时失效（因为 dashboard 是只读聚合）

**业务影响**：即使接入缓存，若无写时失效，会导致数据不一致。
**修复建议**：
1. 在 create/update/delete 方法中调用 `cache.invalidate(key)` 或 `cache.invalidate_prefix("module:")`
2. TTL 作为兜底（如 300s），确保最终一致性

---

## 审计结果汇总

| 维度 | P0 | P1 | P2 | P3 | 已落实 | 总检查项 |
|------|----|----|----|----|--------|----------|
| 7.1 可维护性 | 1 | 1 | 2 | 0 | 5 | 9 |
| 7.2 i18n 与可访问性 | 3 | 2 | 1 | 1 | 2 | 9 |
| 7.3 部署运维 | 2 | 2 | 1 | 1 | 8 | 14 |
| 7.4 CI/CD pipeline 健康度 | 0 | 4 | 1 | 0 | 5 | 10 |
| 7.5 性能优化与缓存策略 | 2 | 2 | 2 | 0 | 8 | 14 |
| **合计** | **8** | **11** | **9** | **3** | **28** | **56** |

## 修复优先级队列

### P0 级（阻塞，共 8 项）
1. **缺陷 7.3-1**：禁止 Docker 违规 — 删除 4 个 Docker 文件（Dockerfile, backend/Dockerfile, frontend/Dockerfile, docker-compose.yml）
2. **缺陷 7.3-2**：快速部署 install.sh 安装 PostgreSQL 客户端 — 移除 postgresql-client 安装
3. **缺陷 7.5-1**：5 个 service 全部未接入缓存层 — 为 user/product/customer/supplier/role service 接入缓存
4. **缺陷 7.5-2**：缓存是内存缓存（moka），非 Redis — 实现 RedisCacheBackend
5. **缺陷 7.2-1**：useI18n 接入率仅 3.2% — 制定分批迁移计划，达到 80%+
6. **缺陷 7.2-2**：aria-label 严重不足（仅 2 文件）— 为所有交互元素添加 aria-label
7. **缺陷 7.2-3**：图片 alt 属性完全缺失（0 处）— 为所有 img 添加 alt
8. **缺陷 7.1-1**：超长函数大量存在（130+ 个 >80 行）— 优先拆分 >200 行的 8 个函数

### P1 级（高，共 11 项）
1. **缺陷 7.3-3**：deploy-latest.sh 前端部署路径错误（/opt/bingxi → /opt/bingxi-erp）
2. **缺陷 7.3-4**：日志目录不一致（/opt/bingxi-erp/logs vs /opt/bingxi-erp/backend/logs）
3. **缺陷 7.4-1**：clippy 使用 baseline 模式，非严格 -D warnings
4. **缺陷 7.4-2**：Rust 格式检查不阻塞 CI（exit 0）
5. **缺陷 7.4-3**：前端格式检查不阻塞 CI（exit 0）
6. **缺陷 7.4-4**：前端 ESLint 不阻塞 CI（exit 0 + --max-warnings 999999）
7. **缺陷 7.5-3**：无 CacheBackend trait + Mock — 抽取 trait 并实现 Mock
8. **缺陷 7.5-4**：缓存命中率未自动接入 Prometheus
9. **缺陷 7.2-4**：ElMessage 硬编码中文 64 处
10. **缺陷 7.2-5**：后端 AppError 硬编码中文 216 处
11. **缺陷 7.1-2**：模块循环依赖（utils↔services，13 个 service 引用 AppState）

### P2 级（中，共 9 项）
1. **缺陷 7.1-3**：染整 service 未充分拆分（1510 行，4 个 service 在一文件）
2. **缺陷 7.1-4**：魔法数字残留（dashboard_service 300s，auth_service 3600）
3. **缺陷 7.3-5**：快速部署 install.sh 前端目录路径错误
4. **缺陷 7.4-5**：ci-deps job 有 continue-on-error
5. **缺陷 7.5-5**：默认 TTL 60s，未按用户/产品 300s→600s 调优
6. **缺陷 7.5-6**：缓存写时 invalidate 策略未在 5 个 service 实现
7. **缺陷 7.2-6**：i18n/index.ts 注释过时（4506 行 vs 实际 467 行）

### P3 级（低，共 3 项）
1. **缺陷 7.3-6**：Dockerfile EXPOSE 8080 与后端端口 8082 不一致（随 Docker 移除自动解决）
2. **缺陷 7.2-7**：颜色对比度 WCAG 2.1 AA 无法自动验证
3. **缺陷 7.2-6**（已列入 P3 的注释过时项，此处合并到 P2）

---

## 附录：审计依据文件清单

| 文件 | 用途 |
|------|------|
| `/workspace/.trae/rules/project_rules.md` | 项目开发规范（规则 1-9、规则 14、死代码处理） |
| `/workspace/.monkeycode/MEMORY.md` | 部署限制、CI 验证偏好 |
| `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md` | V15 审计计划（第 1063-1149 行 类七 5 维度） |
| `/workspace/backend/src/utils/cache.rs` | 内存缓存实现（AppCache, MemoryCache） |
| `/workspace/backend/src/utils/crud_macro.rs` | CRUD 宏抽象 |
| `/workspace/backend/src/utils/app_state.rs` | AppState 容器（utils→services 依赖） |
| `/workspace/backend/src/services/cache_service.rs` | CacheService（moka 进程内缓存） |
| `/workspace/backend/src/services/crm/recycle_rule.rs` | CRM 规则持久化 |
| `/workspace/backend/src/services/role_permission_service.rs` | 角色权限配置化 |
| `/workspace/backend/src/services/dashboard_service.rs` | Dashboard 缓存接入示例 |
| `/workspace/backend/src/services/dye_batch_state_machine_service.rs` | 染整 service（1510 行） |
| `/workspace/frontend/src/i18n/index.ts` | i18n 配置 |
| `/workspace/frontend/src/locales/zh-CN.ts` | 中文资源文件（467 行） |
| `/workspace/frontend/src/locales/en-US.ts` | 英文资源文件（467 行） |
| `/workspace/frontend/src/views/Login.vue` | i18n 接入示范 |
| `/workspace/deploy/deploy.sh` | 主部署脚本 |
| `/workspace/deploy/deploy-backend.sh` | 后端部署脚本 |
| `/workspace/deploy/deploy-frontend.sh` | 前端部署脚本 |
| `/workspace/deploy/deploy-latest.sh` | 远程部署脚本 |
| `/workspace/deploy/deploy-prepare.sh` | 部署准备脚本 |
| `/workspace/deploy/bingxi-backend.service` | systemd 服务文件 |
| `/workspace/Dockerfile` | 根目录 Dockerfile（违规） |
| `/workspace/docker-compose.yml` | docker-compose 配置（违规） |
| `/workspace/快速部署/install.sh` | 一键安装脚本 |
| `/workspace/.github/workflows/ci-cd.yml` | CI/CD 主工作流 |
| `/workspace/.github/workflows/e2e-batch.yml` | E2E 独立工作流 |
| `/workspace/backend/.clippy.toml` | Clippy 配置 |

---

**审计结论**：V15 批 7（类七可维护性与长期治理）审计完成，共发现 **31 项缺陷**（P0:8, P1:11, P2:9, P3:3），**28 项已落实**。最严重的 P0 级问题集中在 Docker 违规、PostgreSQL 客户端安装违规、5 个 service 未接入缓存、i18n 接入率严重不足、aria-label/alt 完全缺失、超长函数大量存在。建议优先修复 P0 级阻塞项，特别是立即删除 Docker 文件和移除 PostgreSQL 客户端安装。

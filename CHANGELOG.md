# 更新日志

本项目的所有显著变更都将记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/) 规范。

---

## [Unreleased] - 2026-06-14

### 已规划（16 任务总规划 - 阶段一）

#### 项目管理（P0）
- 完成项目深度评估报告（17 万行代码，751 子功能，评分 8.0/10）
- 完成 16 任务总规划（5 P0 + 6 P1 + 4 P2 + 4 P3 = 19 项）
- 建立多子代理并行 + 复查子代理 + 总代理审批工作流
- 归档规划文档：[规划-16tasks-2026-06-14.md](file:///workspace/.monkeycode/docs/规划-16tasks-2026-06-14.md)
- 更新用户记忆（MEMORY.md）：[16 任务总规划] 条目

#### 工作流设计
- **4 类执行子代理**：A 业务实现 / B 前端实现 / C 基础设施 / D 架构演进
- **6 波推荐批次**：每波 4-6 任务，约 5 周完成
- **资源限制**：同时运行子代理数 ≤ 6
- **Git 分支策略**：`feature/{task-id}` 独立分支
- **强制报告模板**：子代理必须输出工作报告（改动/决策/测试/风险/自评）
- **复查清单**：10 项（代码规范/dead_code/clippy/eslint/tsc/租户隔离/敏感信息/错误处理/文档/CHANGELOG）

#### 待启动 Wave 1（5 任务）
- P0-5 修复 MaterialShortageAlert 事件定义（C）
- P1-1 补齐 generate-no 端点（4 页面）（A）
- P1-2 注册未挂载页面路由（sales-analysis/security）（B）
- P2-3 修复 CI 测试编译错误（C）
- 创建 logger 工具（C）

#### 项目管理（阶段二）
- 完成项目进度评估（实时代码扫描）
- **重大发现**：原 19 任务中 5 个已完成（P0-1/3/4/5、P1-2）
- 业务流已通过事件驱动架构实现（event_bus.rs:121-123 InventoryFinanceBridgeService.start_listener）
- 实际未完成任务修正为 **13 个**
- 重新规划文档：[规划-重新规划-13tasks-2026-06-14.md](file:///workspace/.monkeycode/docs/规划-重新规划-13tasks-2026-06-14.md)
- 5 波调度：Wave 1（4 子代理，1 周）→ Wave 2（6 子代理，2 周）→ Wave 3（2 子代理，1 周）→ Wave 4（4 子代理，4 周）→ Wave 5 复查
- 总资源：13 执行子代理 + 1 复查；同时运行峰值 6；总周期约 8 周
- 更新用户记忆（MEMORY.md）：[13 任务重新规划] 条目

#### 修订后 13 任务清单
- 业务流：P0-2 销售发货→AR（60%→100%）
- 基础设施：P2-3 rustc 升级（CI 编译失败修复）
- 前端重构：P1-3 拆分 52 大 .vue、P1-4 完成 10 Tab、P1-5 完成 2 TODO、P2-1 虚拟列表、P2-2 console 替换
- 端点：P1-1 generate-no 4 端点
- AI：P2-4 工艺优化 + 质量预测
- 长期：P3-1 微服务、P3-2 WebSocket、P3-3 React Native、P3-4 BI

### Wave 1 执行结果（2026-06-15）

派发 4 子代理并行执行 Wave 1 任务，全部通过总代理审阅。

#### A1 P0-2 销售发货→AR 应收账款（已完成 100%）
- Commit：`b191398 feat(sales): P0-2 销售发货自动生成 AR 应收账款`
- 文件：[backend/src/services/ar/inv.rs](file:///workspace/backend/src/services/ar/inv.rs)
- 新增 `create_receivable` 方法 92 行 + 6 单元测试 130 行
- 关键发现：[backend/src/services/so/delivery.rs:192-224](file:///workspace/backend/src/services/so/delivery.rs#L192-L224) `ship_order` 已实现 AR 集成调用，本次为"补全缺失方法"
- 剩余风险：R3 voucher 凭证未实现；R2 与 ar_invoice_service 双入口隐患

#### C1 P2-3 编译验证（颠覆性发现）
- CICD Run：[https://github.com/57231307/1/actions/runs/27522504353](https://github.com/57231307/1/actions/runs/27522504353)
- **✅ 已验证通过，无代码修改**：当前 main 分支在 Rust 1.94.1 编译完全通过，P2-3 实际已完成
- 6 个 CICD Job 全绿（test / 前端 test / build-backend 12:29 / vite build / release / notify）
- ~~仅 2 个 .clippy.toml 配置提示警告（`std::println` / `std::eprintln` 宏路径）~~ **已修复**：移除 `std::` 前缀（宏不是方法），2026-06-15
- GitHub Release [v2026.615.1138](https://github.com/57231307/1/releases/tag/v2026.615.1138) 已自动发布

#### B1 P1-1 generate-no 4 端点（已完成 100%）
- Commit：`fe91dc9 feat(generate-no): P1-1 补齐 4 端点 generate-no`
- 4 端点 + 4 前端 API + 4 单测，共 9 文件 +255 行
- 前缀：IC（inventoryCount）/ RK（purchaseReceipt）/ IA（inventoryAdjustment）/ IT（inventoryTransfer）
- 路径风格沿用 RESTful 嵌套（`/api/v1/erp/{domain}/{resource}/generate-no`）

#### B2 P1-5 完成 2 TODO（已完成 100%）
- Commit：`a3b18ca fix(frontend): P1-5 入库单明细 API 类型强化`
- 已推送 `feature/P1-5-completed-2-todos` 等 CICD
- 重大发现：`ca0ca48` 提交已完整实现 2 处 TODO，本次仅做类型补强（消除 `as` 强转）

#### 状态汇总
- Wave 1 进度：4/4 完成 ✅
- 待用户操作：推送 `feature/p0-2-sales-ar`（A1）、`feature/p1-1-generate-no`（B1）触发 CICD
- B2 已推送待 CICD 结果
- 更新用户记忆（MEMORY.md）：[Wave 1 执行结果]、[沙箱与CICD验证策略] 条目

---

## [2026.614.1353] - 2026-06-14

### 已修复（项目全方位校验、整理与清理 - 第二轮）

#### 代码质量（P1）
- 后端 `backend/src/services/inventory_count_service.rs` 已拆分为子模块（`inventory_count/`）并完成对外公开 API 兼容
- 在 `backend/src/services/mod.rs` 新增 `pub mod inventory_count` 声明

#### 前端重构（P1）
- 完善 `views/system/tabs/RoleTab.vue`：从骨架升级为完整可工作组件（包含 CRUD、权限配置对话框，共 267 行）
- 修复角色编辑时"角色名称"和"角色编码"在编辑模式下禁用的问题

### 已修复（项目全方位校验、整理与清理）

#### 安全（P0）
- 删除未使用 CI 备份文件 `.github/workflows/ci-cd.yml.backup`
- 统一 SQL 迁移目录：删除两个无引用的重复迁移目录（`backend/database/migration/` 26 文件、`backend/src/database/migration/` 9 文件），归档至 `docs/database/legacy-migration-snapshots/`
- 修复 `backend/src/cli/migrate.rs` 中错误的迁移目录注释（指向不存在的 `src/database/migration/`）

#### 重复资源（P1）
- 合并三套密码哈希工具：删除 `backend/hasher_tool/` 和 `backend/Cargo.toml.hash`，保留主项目 `backend/src/bin/hash_password.rs`
- 清理 `backend/src/services/mod.rs` 中 7 个旧路径兼容层（purchase_order_service、sales_service、crm_service、inventory_transfer_service、ar_reconciliation_service、ai_analysis_service、report_engine_service）
- 批量迁移 21 个文件中的 31 处 `crate::services::<alias>::` 引用到新路径（`po::order`、`so::order`、`crm::cust`、`inv`、`ar`、`ai`、`report`）

#### 前端重构（P1）
- 拆分 1478 行的 `views/system/index.vue`：
  - 抽出 `views/system/tabs/UserTab.vue`（完整可工作，275 行）
  - 创建 11 个 Tab 骨架（RoleTab/DepartmentTab/PermissionTab/DataPermissionTab/FieldPermissionTab/NotificationTab/AuditTab/WebhookTab/SystemUpdateTab/TenantTab/CompanyTab）
  - 在 `system/index.vue` 顶部添加拆分指引注释
  - 详细拆分计划见 `docs/refactoring/frontend-vue-splitting-plan.md`

#### 依赖升级（P1）
- 前端依赖升级：
  - `vite`: `^6.4.2` → `^6.4.3`（修复 dev server SSRF 相关依赖）
  - `vitest`: `^1.2.0` → `^2.1.0`（缓解 esbuild 嵌套漏洞）
  - `esbuild`: `^0.25.0` → `^0.25.12`（由 `npm audit fix` 自动升级）
- 完整 npm audit 报告：`.audit-reports/npm-audit.json`（含 2 critical + 3 moderate 漏洞记录与升级路径）

#### 文档与规范（P2）
- 创建 `CHANGELOG.md`（本文件）
- 创建 `docs/database/legacy-migration-snapshots/README.md`（归档说明）
- 创建 `docs/refactoring/frontend-vue-splitting-plan.md`（47 个 Vue 组件拆分计划）
- 创建 `.audit-reports/` 目录（保存审计报告）
- 补充 `frontend/.env.production.example`（生产环境模板）
- 迁移根目录散落运维文档至 `docs/reports/historical/`
- 迁移前端调试脚本至 `frontend/scripts/`
- 补充 LICENSE 第三方组件许可声明

### 已知遗留问题
- `views/system/index.vue` 还有 10 个 Tab 仍为骨架，需前端工程师按 UserTab 模板完成数据加载与表单逻辑（详见 `docs/refactoring/frontend-vue-splitting-plan.md`）
- 其他 46 个超过 500 行的 .vue 文件（sales-ext、purchase-ext、sales、ap、trading 等）仍待拆分
- `inventory_count_service.rs`（949 行）建议拆为 query/writer/reporter 子模块
- 前端虚拟列表化（vue-virtual-scroller 或 Element Plus `el-table-v2`）尚未引入

---

## [2026.522.2] - 2026-05-22

### 新增
- 资金管理模块
- 销售/采购合同模块
- 多币种与汇率模块
- 工作流引擎 BPM

### 修复
- 库存调整审批流
- 销售订单状态机

---

## [2026.1.0] - 2026-01-15

### 新增
- 核心业务模块：采购、销售、库存、生产、财务、CRM
- AI 智能分析（销售预测、库存优化、异常检测）
- 报表引擎（Excel/PDF 导出）
- 多租户 SaaS 架构
- 消息通知（站内信、邮件、短信）
- 移动端响应式支持

### 技术栈
- **后端**：Rust 1.75+ / Axum 0.7 / SeaORM 1.0 / PostgreSQL 15
- **前端**：Vue 3.4 / Vite 5.0 / Element Plus 2.4 / Pinia 2.1
- **基础设施**：Redis 7 / gRPC（Tonic）/ GitHub Actions / Prometheus / Grafana

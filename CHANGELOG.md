# 更新日志

本项目的所有显著变更都将记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/) 规范。

---

## [Unreleased] - 2026-06-05

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

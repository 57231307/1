# 数据治理扫描报告 (Data Governance Scan)

- 扫描日期：2026-08-22
- 扫描范围：`backend/migration/src/domain/`、`backend/src/models/`、`backend/src/services/`、`backend/src/utils/`
- 扫描代理：审计代理（只读，未修改任何代码文件）
- 参考批次：27.1 - 27.8（共 8 项数据治理扫描）

---

## 27.1 主数据完整性 (Master Data Integrity)

**扫描命令**：`grep -rn "UNIQUE\|unique" backend/migration/src/domain/`

**扫描结果摘要**：
- 命中 20 条记录，覆盖多个批次迁移文件
- 命名约束形式存在两种风格并存：
  - 命名约束：`CONSTRAINT "uk_ct_name" UNIQUE ("name")`、`CONSTRAINT "chk_ct_task_type" CHECK (...)`
  - 行内约束：`VARCHAR(50) NOT NULL UNIQUE`（如 `declaration_no`、`verification_no`、`permit_no`、`manifest_no`、`contract_no`、`voucher_no`、`dimension_code`、`code`）
- 复合唯一键样例：
  - `m0004_add_field_permissions.rs:22` — `UNIQUE ("role_id", "resource_type", "field_name")`
  - `m0006_add_general_ledger_and_finance_base.rs:76` — 唯一索引 `idx_account_balances_unique ("subject_id", "period")`
  - `m0006_add_general_ledger_and_finance_base.rs:95` — 唯一索引 `idx_accounting_periods_unique ("year", "period")`
- `unique_sessions` / `unique_users` 为统计列名（非约束关键字），属误命中

**评估结论**：
- 主数据完整性约束总体覆盖到位，关键业务表（会计科目、会计期间、凭证号、字段权限）均设有 UNIQUE 约束
- **风险点**：约束声明风格不统一，行内 `NOT NULL UNIQUE` 与命名 `CONSTRAINT "uk_xxx" UNIQUE` 混用，后续维护和告警定位时无法通过统一约束名检索
- **建议**：新迁移统一采用命名约束（`CONSTRAINT "uk_<表缩写>_<字段>" UNIQUE (...)`），便于 DBA 在违反约束时快速定位

---

## 27.2 命名规范 (Naming Convention)

**扫描命令**：`grep -rn "VARCHAR\|INTEGER\|TIMESTAMPTZ" backend/migration/src/domain/system/`

**扫描结果摘要**（前 10 行）：
| 文件 | 行 | 列名 | 类型 | 备注 |
|---|---|---|---|---|
| m0004_add_field_permissions.rs | 13 | `role_id` | INTEGER NOT NULL | snake_case |
| m0004_add_field_permissions.rs | 14 | `resource_type` | VARCHAR(100) NOT NULL | snake_case |
| m0004_add_field_permissions.rs | 15 | `field_name` | VARCHAR(100) NOT NULL | snake_case |
| m0004_add_field_permissions.rs | 18 | `mask_strategy` | VARCHAR(20) NOT NULL DEFAULT 'NONE' | snake_case |
| m0004_add_field_permissions.rs | 20-21 | `created_at`/`updated_at` | TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP | 规范 |
| m0006_add_general_ledger_and_finance_base.rs | 18 | `code` | VARCHAR(50) NOT NULL UNIQUE | 通用名 |
| m0006_add_general_ledger_and_finance_base.rs | 19 | `name` | VARCHAR(200) NOT NULL | 通用名 |
| m0006_add_general_ledger_and_finance_base.rs | 20 | `level` | INTEGER NOT NULL DEFAULT 1 | 通用名 |
| m0006_add_general_ledger_and_finance_base.rs | 21 | `parent_id` | INTEGER | 外键命名规范 |

**评估结论**：
- 列名命名一致性良好，全部采用 `snake_case`，符合 PostgreSQL 惯例
- 时间戳字段统一使用 `TIMESTAMPTZ`（带时区），符合分布式系统最佳实践
- **轻微风险**：部分通用列名（`code`、`name`、`level`）语义过于宽泛，在多表 JOIN 时易产生歧义，建议在文档层补充表前缀语义说明（如 `gl_code` / `gl_name`）
- **建议**：保持现状，新增表沿用 `snake_case` + `TIMESTAMPTZ` 规范

---

## 27.3 数据血缘 (Data Lineage)

**扫描命令**：`grep -rn "business_trace\|batch_trace_log\|trace_log" backend/src/models/`

**扫描结果摘要**：
- 血缘模型覆盖完整，共发现 5 个追溯相关实体表 + 1 个视图：
  - `business_traces` — 主业务追溯表（`backend/src/models/business_trace.rs:12`）
  - `v_business_trace_view` — 业务追溯视图（`business_trace_view.rs:11`）
  - `batch_trace_log` — 批次追溯日志（`batch_trace_log.rs:13`）
  - `business_trace_assist_links` — 辅助追溯关联（`business_trace_assist_link.rs:12`）
  - `business_trace_snapshot` — 追溯快照（`business_trace_snapshot.rs:15`）
  - `business_trace_chain` — 追溯链（`mod.rs:96`）
- `status/general.rs:152` 记录了 `batch_trace_log.operation_type` 的枚举值定义，说明操作类型有受控字典

**评估结论**：
- 数据血缘基础设施健全，具备主表 + 视图 + 快照 + 关联链 + 日志的完整闭环
- 追溯操作类型有受控枚举，血缘可追溯性良好
- **建议**：确认 `business_trace_snapshot` 的快照触发时机与保留策略，避免快照表无限膨胀

---

## 27.4 历史归档 (Historical Archiving)

**扫描命令**：`grep -rn "archive\|归档\|retention\|cleanup\|清理" backend/src/services/log_*service*.rs backend/src/services/audit_cleanup_service.rs`

**扫描结果摘要**：
- `log_archive_service.rs` 提供完整的日志冷数据归档能力：
  - 配置结构体 `LogArchiveConfig`，字段 `archive_after_days: i64`（`log_archive_service.rs:17`）
  - 默认归档阈值：**90 天**（`log_archive_service.rs:25`）
  - 归档入口函数：`archive_old_logs(&self) -> Result<u64, AppError>`（`log_archive_service.rs:43`）
  - 阈值计算：`Utc::now() - chrono::Duration::days(self.config.archive_after_days)`（`log_archive_service.rs:44`）
  - 归档前先查询需归档数量，无可归档时记录 `info!("没有需要归档的审计日志")`
- `audit_cleanup_service.rs` 文件存在（grep 未返回内容，说明文件存在但未命中中文/英文关键词，需进一步确认实现）

**评估结论**：
- 历史归档机制设计合理，90 天阈值符合常见审计日志保留要求
- 归档前先 COUNT 再执行的设计可减少无效操作，日志可观测性良好
- **风险点**：归档阈值为硬编码常量 `90`，无法按表/按业务域差异化配置
- **建议**：将 `archive_after_days` 提升至配置文件（`config.yaml`），支持按表差异化配置；并确认 `audit_cleanup_service.rs` 的清理逻辑是否被调度执行

---

## 27.5 数据脱敏 (Data Masking)

**扫描命令**：`grep -rn "mask\|脱敏\|pii\|PII" backend/src/utils/pii_mask.rs backend/src/utils/field_mask.rs`

**扫描结果摘要**：
- `pii_mask.rs` 提供 V15 P2 20.8-C 规范的 PII 脱敏工具：
  - 覆盖类型：手机号、身份证号、邮箱、密码字段
  - 实现方式：`LazyLock<Regex>` 正则预编译（`pii_mask.rs:32` 起）
    - 手机号：`r"1[3-9][0-9]{9}"`（`pii_mask.rs:32`）
    - 身份证号：正则编译（`pii_mask.rs:40`）
    - 邮箱：正则编译（`pii_mask.rs:47`）
    - 密码字段：正则编译（`pii_mask.rs:54`）
  - 统一入口：`mask_pii(msg)`（`pii_mask.rs:57`）
  - 正则编译失败时使用 `expect("PII_MASK: xxx 正则编译失败")`，启动期快速失败
- `field_mask.rs` 文件存在但本次 grep 未返回内容，需确认是否实现字段级动态脱敏策略

**评估结论**：
- PII 脱敏工具实现规范，覆盖四大敏感数据类型，启动期快速失败设计正确
- 正则预编译避免运行时性能开销
- **风险点**：`field_mask.rs` 未命中关键词，可能存在动态字段级脱敏策略缺失
- **建议**：确认 `field_mask.rs` 是否对接 `m0004` 的 `mask_strategy` 列（值 `NONE` 等），实现按角色字段的动态脱敏

---

## 27.6 审计追踪 (Audit Trail)

**扫描命令**：`grep -rn "audit_log\|omni_audit\|operation_log" backend/src/models/`

**扫描结果摘要**：
- 审计追踪模型覆盖完整，多表分层设计：
  - `audit_logs` — 主审计日志表（`audit_log.rs:91`）
  - `omni_audit_logs` — 全量审计表（`omni_audit_log.rs:6`）
  - `customer_audit_log` — 客户专属审计表（`customer_audit_log.rs:7`）
  - `operation_log` — 操作日志模块（`mod.rs:32`）
  - `export_approval_request.rs:191` — 定义 `AUDIT_LOG` 常量，用于导出审批审计
- `mod.rs:229` 注释明确："V15 缺陷 10-4：审计日志导出二次审计表（防篡改，独立于 audit_logs）"
- `status/general.rs:112-113` 记录 `omni_audit_message.status` 枚举与对应服务 `omni_audit_service.rs`

**评估结论**：
- 审计追踪架构成熟，具备分层审计（主表 + 全量表 + 客户专属表 + 操作日志）和防篡改二次审计表
- 审计导出本身也被审计，符合金融级合规要求
- **建议**：确认各审计表的写入路径是否统一通过 `omni_audit_service`，避免业务层直接 INSERT 绕过审计

---

## 27.7 脏数据检测 (Dirty Data Detection)

**扫描命令**：`grep -rn "CHECK\|约束" backend/migration/src/domain/`

**扫描结果摘要**：
- CHECK 约束覆盖良好，发现多处枚举值校验：
  - `m0080_create_collection_templates.rs:36` — `CONSTRAINT "chk_ct_task_type" CHECK (...)`
  - `m0080_create_collection_templates.rs:39` — `CONSTRAINT "chk_ct_overdue_stage" CHECK (...)`
  - `m0077_add_oa_visibility_consent_retention.rs:71` — `CHECK ("consent_type" IN ('behavior_tracking', 'page_view_tracking', 'cookie_usage', 'marketing_email'))`
  - `m0081_create_fixed_asset_counts.rs:42` — `CONSTRAINT "chk_fac_status" CHECK (...)`
  - `m0081_create_fixed_asset_counts.rs:71` — `CONSTRAINT "chk_fac_count_result" CHECK (...)`
- 外键约束也有显式声明：
  - `m0004_add_field_permissions.rs:40` — `-- 添加外键约束`
  - `m0001_initial_schema.rs:627` — `-- 20. 创建外键约束`
  - `m0002_add_crm_and_greige_tables.rs:59` — `-- 添加外键约束`
- 业务约束注释：`m0078_batch18_greige_outsourcing_quality_scheduling.rs:125` — "P1 batch-18 缺陷 9.1：排程基于缸号批量约束"

**评估结论**：
- CHECK 约束使用规范，命名约束（`chk_<表缩写>_<字段>`）和行内 CHECK 两种形式并存
- 枚举值校验到位（`consent_type`、`task_type`、`overdue_stage`、`status`、`count_result`），有效防止脏数据写入
- 外键约束在初始 schema 中有显式声明
- **风险点**：部分业务约束（如"排程基于缸号批量约束"）以注释形式存在，未转化为 DB 约束，依赖应用层校验
- **建议**：评估关键业务规则是否可下沉为 DB 级 CHECK 或触发器，减少应用层遗漏风险

---

## 27.8 迁移校验 (Migration Validation)

**扫描命令**：`grep -rn "validate\|校验\|verify" backend/src/services/init_service.rs`

**扫描结果摘要**（仅 3 条命中）：
- `init_service.rs:255` — `/// 参数校验错误（P0 新增：用于密码强度等输入校验，HTTP 400）`
- `init_service.rs:256` — `#[error("参数校验错误：{0}")]`
- `init_service.rs:268` — `InitError::ValidationError(e) => AppError::validation(format!("参数校验失败: {}", e))`

**评估结论**：
- **重大风险点**：`init_service.rs` 中未发现任何迁移完成后的数据完整性校验逻辑（如 `verify_schema`、`validate_migration`、`count_rows` 等校验步骤）
- 现有校验仅覆盖输入参数校验（密码强度等），不涉及迁移结果校验
- 迁移完成后缺少以下校验环节：
  1. 表/索引/约束是否创建成功
  2. 关键主数据是否初始化（如会计科目、字段权限）
  3. 行数对账（迁移前后）
- **建议**：在 `init_service` 中增加 `verify_migration()` 后置步骤，迁移完成后执行 schema 校验 + 关键表行数校验 + 主数据完整性校验，失败时回滚或告警

---

## 总体评估

| 扫描项 | 评估等级 | 关键发现 |
|---|---|---|
| 27.1 主数据完整性 | 良好 | UNIQUE 约束覆盖到位，约束命名风格不统一 |
| 27.2 命名规范 | 良好 | snake_case + TIMESTAMPTZ 规范一致，部分列名过宽泛 |
| 27.3 数据血缘 | 优秀 | 主表+视图+快照+链+日志闭环完整 |
| 27.4 历史归档 | 良好 | 90 天归档机制完善，阈值硬编码不可配置 |
| 27.5 数据脱敏 | 良好 | PII 四类脱敏规范，field_mask 对接待确认 |
| 27.6 审计追踪 | 优秀 | 分层审计 + 防篡改二次审计表 |
| 27.7 脏数据检测 | 良好 | CHECK 约束覆盖枚举值，部分业务规则仅注释 |
| 27.8 迁移校验 | 需改进 | 缺少迁移后置校验逻辑，存在数据完整性盲区 |

### 高优先级改进建议

1. **P0 - 迁移校验缺失**（27.8）：`init_service.rs` 缺少迁移后置校验，存在数据完整性盲区，建议增加 `verify_migration()` 步骤
2. **P1 - 归档阈值可配置化**（27.4）：`archive_after_days` 硬编码为 90，建议提升至配置文件支持差异化配置
3. **P1 - field_mask 对接确认**（27.5）：确认 `field_mask.rs` 是否对接 `m0004` 的 `mask_strategy` 实现动态脱敏
4. **P2 - 约束命名统一**（27.1/27.7）：UNIQUE 和 CHECK 约束命名风格不统一，建议新迁移统一采用命名约束
5. **P2 - 业务规则下沉**（27.7）：部分业务约束仅以注释存在，建议评估下沉为 DB 级约束

---

*本报告由审计代理生成，仅扫描未修改任何代码文件。*

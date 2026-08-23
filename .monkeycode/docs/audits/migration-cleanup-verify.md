# 迁移清理验证审计报告

- 审计编号: MIG-CLEANUP-001
- 审计日期: 2026-08-23
- 审计目标: 迁移文件重组后的清理完整性验证（旧子迁移文件、旧数据、旧引用、CI/deploy 脚本）
- 审计模式: 只读审计（不修改代码文件、不创建 PR、不推送）
- 审计范围: `backend/migration/`、`backend/database/`、`backend/src/`、`deploy/`、`快速部署/`、`.github/workflows/`、CI/deploy 脚本

## 总体结论

**部分通过（存在 3 个 P0/P1 级问题 + 多个 P2/P3 级问题）**

迁移目录已从 11 域成功精简为 6 域（system/business/sales_crm/production/finance/v15），旧子迁移文件（m0001~m0118）已从工作区删除。但存在 **未提交的删除操作**、**残留冗余 SQL 文件**、**CI 引用失效路径** 等问题需要处理。

| 严重级别 | 数量 | 说明 |
|---------|------|------|
| P0（阻断） | 1 | rls.sql 内容已迁移至 finance 域但文件未删除，存在重复定义风险 |
| P1（重要） | 2 | CI 引用不存在的 database/migration 路径；迁移连续性检查逻辑与 DeriveMigrationName 命名格式不兼容 |
| P2（次要） | 3 | 注释中残留旧文件名引用；迁移执行顺序与注释描述矛盾；clippy baseline 时间戳过期 |
| P3（提示） | 3 | .clippy-baseline.txt 无迁移文件路径；migration Cargo.toml 缺 [lints.clippy]；v15 域未纳入 git 跟踪 |

---

## 问题清单

### P0：阻断级问题

#### P0-1：`backend/database/rls.sql` 内容已迁移但文件未删除

- **位置**: `backend/database/rls.sql`（156 行）
- **现象**: rls.sql 定义了 5 张表的 RLS 策略（customers/suppliers/sales_orders/crm_lead/crm_opportunity），而 `backend/migration/src/domain/finance/mod.rs` 第 123-205 行已完整包含相同的 5 张表的 `ENABLE ROW LEVEL SECURITY` + `CREATE POLICY` 语句，内容完全一致。
- **引用情况**:
  - `backend/src/middleware/rls_context.rs:8` 注释引用 `rls.sql`
  - `backend/src/middleware/rls_context.rs:11` 注释引用 `rls.sql`
  - `backend/tests/rls_context_test.rs:18` 注释引用 `backend/database/rls.sql`
- **加载机制**: rls.sql **没有被任何代码或迁移通过 `include_str!` 或 `psql -f` 加载**，纯属冗余文件。
- **风险**: 文件内容与 finance 域迁移重复，后续维护时若只改一方会导致策略不一致；注释引用残留文件会误导开发者认为 rls.sql 仍被加载。
- **建议**: 删除 `backend/database/rls.sql`，更新 `rls_context.rs` 和 `rls_context_test.rs` 中的注释，指向 `finance` 域迁移。

---

### P1：重要级问题

#### P1-1：CI 工作流引用不存在的 `database/migration` 路径

- **位置**: `.github/workflows/ci-cd.yml:2229-2231`
- **现象**:
  ```yaml
  if [ -d "database/migration" ]; then
    mkdir -p release/bingxi-erp/database
    cp -r database/migration release/bingxi-erp/database/
  fi
  ```
- **问题**: 根目录 `database/migration` 目录**不存在**（迁移文件位于 `backend/migration/`，编译进后端二进制）。此代码块因 `[ -d ]` 判断为 false 而静默跳过，不会报错，但属于死代码。
- **对比**: 第 2225-2226 行的 `backend/database` 路径判断是正确的（指向 `backend/database/rls.sql`）。
- **建议**: 删除 2229-2231 行的 `database/migration` 拷贝逻辑（迁移已编译进二进制，无需随发布包分发 SQL 文件）。

#### P1-2：`check_migration_continuity` 与 DeriveMigrationName 命名格式不兼容

- **位置**: `backend/src/bootstrap/service_bootstrap.rs:276-310`
- **现象**: `check_migration_continuity` 函数解析 `seaql_migrations` 表中的迁移名，使用 `name.strip_prefix('m')` 提取编号，期望格式为 `m0001_xxx`。
- **实际命名**: 6 个域迁移使用 `#[derive(DeriveMigrationName)]`，SeaORM 2.0.2 的 `DeriveMigrationName` 基于 `module_path!()` 生成名称，格式为 `migration::domain::system::Migration`（不含 `m` 前缀和数字编号）。
- **后果**:
  - `strip_prefix('m')` 对新格式返回 `None`，所有迁移名被跳过
  - `migration_numbers` 列表为空
  - `gaps` 为空，输出 "迁移连续性检查通过（0 个迁移）" 的误导信息
  - 连续性检查功能**完全失效**
- **建议**: 更新 `check_migration_continuity` 的解析逻辑以适配 `module_path` 格式，或移除该检查（6 域聚合后已无编号连续性需求）。

---

### P2：次要级问题

#### P2-1：迁移注释中残留旧文件名引用

- **位置及内容**:

| 文件 | 行号 | 残留引用 |
|------|------|---------|
| `system/mod.rs` | 67 | `m0001_initial_schema.rs` |
| `production/mod.rs` | 15 | `m0001 创建` |
| `production/mod.rs` | 29 | `m0001 创建以来` |
| `production/mod.rs` | 165 | `m0044 整合迁移`、`m0029` |
| `sales_crm/mod.rs` | 171 | `production_quality 域的 m0044` |
| `sales_crm/mod.rs` | 172 | `m0044` |
| `sales_crm/mod.rs` | 174 | `m0044` |
| `lib.rs` | 4 | 注释 `system → business → sales_crm → production → finance → v15`（顺序正确，无旧引用） |

- **影响**: 纯注释，不影响编译和迁移执行。但会误导开发者查找不存在的旧文件。
- **建议**: 更新注释，移除旧文件名引用，改为描述当前域的功能。

#### P2-2：sales_crm 域执行顺序与注释描述矛盾

- **位置**: `backend/migration/src/domain/sales_crm/mod.rs:171-174`
- **现象**: 注释称 "custom_orders 表由 production_quality 域的 m0044 创建，本迁移位于 sales_crm 域，执行早于 m0044"，实际执行顺序为：
  ```
  system → business → sales_crm → production → finance → v15
  ```
  `custom_orders` 表在 `production/mod.rs:242` 创建，**production 在 sales_crm 之后执行**。
- **兜底机制**: 代码使用 `IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'custom_orders')` 进行条件判断（第 178 行），表不存在时跳过 ALTER，不会中断迁移。production 域创建表时已声明 `notes` 列，所以此 ALTER 实际上是幂等兜底。
- **风险**: 逻辑上正确（有 information_schema 保护），但注释描述与实际执行顺序不符，且此 ALTER 操作实际上永远不会执行（表在 sales_crm 执行时尚不存在，production 创建时已包含 notes 列）。
- **建议**: 更新注释说明实际执行顺序，或将此兜底 ALTER 移至 production 域。

#### P2-3：`.clippy-baseline.txt` 时间戳过期

- **位置**: `backend/.clippy-baseline.txt`
- **现象**:
  - baseline 修改时间: 2026-08-20 11:16:43
  - 迁移文件修改时间: 2026-08-23 01:30:19（lib.rs）
  - 域 mod.rs 修改时间: 2026-08-23 03:21（v15/mod.rs）
- **问题**: baseline 比迁移文件旧 3 天，若 CI 使用 baseline 比对模式，可能漏报迁移文件新增的 lint 警告。
- **缓解**: baseline 中未包含任何 migration 相关条目（仅含 `migration_jump_detector.rs`，属应用代码非迁移代码），实际影响有限。
- **建议**: 迁移重组完成后重新生成 baseline。

---

### P3：提示级问题

#### P3-1：migration Cargo.toml 缺少 [lints.clippy] 配置

- **位置**: `backend/migration/Cargo.toml`
- **现象**: 只有 `[lints.rust]`（dead_code/unused_imports/unused_variables = warn），缺少 `[lints.clippy]` 配置。
- **对比**: `backend/Cargo.toml` 的注释说明 "workspace lint 不传递给子 crate"，migration crate 需独立配置。
- **影响**: migration crate 的 clippy lint 未显式配置，使用默认行为（部分 clippy lint 可能不触发）。
- **建议**: 视项目 clippy 策略决定是否添加 `[lints.clippy]`（若主 crate 有 clippy lint 列表，建议同步添加）。

#### P3-2：v15 域未纳入 git 跟踪

- **位置**: `backend/migration/src/domain/v15/`
- **现象**: `git status` 显示 `?? backend/migration/src/domain/v15/`（未跟踪）。
- **说明**: v15/mod.rs 文件存在于工作区（145KB），但尚未 `git add`。
- **影响**: 不影响编译和运行，但未纳入版本控制。
- **建议**: 提交时 `git add backend/migration/src/domain/v15/`。

#### P3-3：`backend/database/` 目录仅剩 rls.sql 一个文件

- **位置**: `backend/database/`
- **现象**: 旧数据文件 `init_data.sql` 和 `init_admin_permissions.sql` 已删除（git status 显示 `D`），目录下仅剩 `rls.sql`。
- **关联**: 若按 P0-1 建议删除 rls.sql，则 `backend/database/` 目录将变空，CI 第 2225 行的 `backend/database` 拷贝逻辑也应一并清理。
- **建议**: 删除 rls.sql 后，同步清理 CI 中 `backend/database` 的拷贝逻辑。

---

## 检查项逐项验证

### 1. 旧迁移文件是否全部删除

| 检查项 | 结果 | 说明 |
|--------|------|------|
| `find ... -name "m0*.rs"` | 通过 | 工作区无旧子迁移文件（m0001~m0118 已删除） |
| `ls backend/migration/src/domain/` | 通过 | 仅 6 个域目录 + mod.rs |
| 旧域目录（fixes/v15_core/v15_batch18/v15_batch19/v15_extensions/v15_final） | 通过 | 均已删除 |
| 域目录结构 | 通过 | 每个域仅含 mod.rs（system/business/sales_crm/production/finance/v15） |

**注**: git status 显示大量 `D`（已删除）状态的旧文件，这些删除操作**尚未提交**。

### 2. 旧迁移数据是否删除

| 检查项 | 结果 | 说明 |
|--------|------|------|
| `init_data.sql` | 通过 | 已删除（git status: `D`） |
| `init_admin_permissions.sql` | 通过 | 已删除（git status: `D`） |
| `backend/database/rls.sql` | **未删除** | 156 行，内容已迁移至 finance 域（见 P0-1） |
| `backend/scripts/p2-2-slow-query.sql` | 不相关 | 运维诊断脚本，非迁移数据 |

### 3. 旧迁移引用是否清理

| 检查项 | 结果 | 说明 |
|--------|------|------|
| `m0001/m0002/m0044/m0029` 文件名引用 | **残留** | 6 处注释引用（见 P2-1） |
| `v15_core/v15_batch/v15_extensions/v15_final/fixes` 域引用 | 通过 | 代码中无引用（仅注释中 lib.rs 第 4 行列出当前 6 域顺序，无旧域名） |
| `production_quality/finance_compliance/core_schema` 等旧域名 | 残留 1 处 | `sales_crm/mod.rs:171` 引用 `production_quality`（见 P2-1） |

### 4. Cargo.toml 和 lib.rs 引用

| 检查项 | 结果 | 说明 |
|--------|------|------|
| `Cargo.toml [lib] path` | 通过 | `path = "src/lib.rs"` 正确 |
| `lib.rs` 域注册数量 | 通过 | 6 个域全部注册 |
| `lib.rs` 注册格式 | 通过 | 全部 `domain::xxx::Migration` |
| `domain/mod.rs` pub mod 数量 | 通过 | 6 个 pub mod（system/business/sales_crm/production/finance/v15） |
| `Cargo.toml [lints]` | 部分 | 有 `[lints.rust]`，缺 `[lints.clippy]`（见 P3-1） |

### 5. 代码中是否引用旧迁移路径

| 检查项 | 结果 | 说明 |
|--------|------|------|
| `database/migration`、`database/rls`、`database/init` 引用 | 通过 | 应用代码无 SQL 文件路径引用 |
| `migrate run` 命令 | 通过 | `backend/src/cli/migrate.rs` 正确使用 `Migrator::up` |
| `seaql_migrations` 引用 | **兼容性问题** | `check_migration_continuity` 解析逻辑与新命名格式不兼容（见 P1-2） |

### 6. deploy 脚本是否引用旧迁移

| 检查项 | 结果 | 说明 |
|--------|------|------|
| `deploy/*.sh` 中 `.sql`/`psql -f` 引用 | 通过 | 无 SQL 文件直接引用 |
| `deploy/*.sh` 中 `migration` 引用 | 通过 | 均为 `bingxi migrate run`（后端二进制内置迁移） |
| `快速部署/install.sh` | 通过 | 同上，使用 `bingxi migrate run` |
| rls.sql 加载 | 通过 | deploy 脚本不加载 rls.sql |

### 7. CI 工作流是否引用旧迁移

| 检查项 | 结果 | 说明 |
|--------|------|------|
| `database/migration` 引用 | **失效** | 引用不存在的根目录路径（见 P1-1） |
| `backend/database` 引用 | 有效但冗余 | 路径存在（含 rls.sql），但 rls.sql 无加载机制 |
| `.sql` 文件引用 | 通过 | CI 无 psql/sql 执行 |

### 8. 细节问题

| 检查项 | 结果 | 说明 |
|--------|------|------|
| `.clippy-baseline.txt` 时间戳 | 过期 | baseline 比迁移文件旧（见 P2-3） |
| `.clippy-baseline.txt` 迁移路径 | 无 | baseline 不含 migration crate 条目 |
| `Cargo.toml [lints]` | 部分 | 有 rust lint，缺 clippy lint（见 P3-1） |
| `.gitignore` 忽略 migration | 通过 | 未忽略 migration/database 目录 |
| v15 域 git 跟踪 | 未跟踪 | 需 `git add`（见 P3-2） |

---

## 迁移命名机制说明

### DeriveMigrationName 行为

6 个域迁移均使用 `#[derive(DeriveMigrationName)]`，无自定义 `name()` 实现。SeaORM 2.0.2 的 `DeriveMigrationName` 基于 `module_path!()` 生成迁移名，格式为：

```
migration::domain::system::Migration
migration::domain::business::Migration
migration::domain::sales_crm::Migration
migration::domain::production::Migration
migration::domain::finance::Migration
migration::domain::v15::Migration
```

**影响**: 迁移名不再以 `m` 开头、不含数字编号，导致 `check_migration_continuity` 的 `strip_prefix('m')` 解析逻辑失效（见 P1-2）。

### 迁移执行顺序

lib.rs 中的注册顺序即执行顺序：

```
1. system     → 核心表（用户/角色/部门/产品/供应商/客户/库存/财务基础）
2. business   → 业务表（依赖 system）
3. sales_crm  → 销售报价（依赖 business 的 product_color_prices）
4. production → 生产/质量（创建 custom_orders，sales_crm 域有兜底 ALTER）
5. finance    → 合规/RLS（依赖 system 的 customers.owner_id）
6. v15        → V15 各批次扩展
```

---

## 建议处理优先级

1. **P0-1**: 删除 `backend/database/rls.sql`，更新 `rls_context.rs` 和 `rls_context_test.rs` 注释
2. **P1-1**: 清理 CI 中 `database/migration` 死代码（2229-2231 行）
3. **P1-2**: 修复或移除 `check_migration_continuity` 函数
4. **P2-1**: 清理迁移注释中的旧文件名引用
5. **P2-2**: 修正 sales_crm 域注释中的执行顺序描述
6. **P3-2**: `git add` v15 域目录
7. 提交所有删除操作（git status 中大量 `D` 状态文件）

---

## 附：已有相关审计报告

- `.monkeycode/docs/audits/migration-regroup-verification.md`（2026-08-22）- 基于旧的 11 域结构，内容已过时
- `.monkeycode/docs/migration-domain-regroup-plan.md`（2026-08-22）- 域重组计划

# 业务与部署专项审计报告

- 审计范围：面料 SKU 编码 / 色卡面料关联 / 供应商认证 / 前端测试覆盖率 / 蓝绿部署 / 部署回滚
- 审计代理：MonkeyCode AI Audit Agent
- 审计日期：2026-08-22
- 仓库路径：`/workspace`
- 审计规则：只读审计，不修改业务代码、不创建 PR、不推送

---

## 16.16 面料 SKU 编码（产品编码）

### 证据

- `backend/src/models/product.rs:17-18`：字段 `code: String`，注释标注「产品编码（唯一）」，但 Rust 实体层未声明 `unique` 约束（SeaORM 实体也不强制 DB 约束）。
- `backend/src/services/product_ops/crud.rs:36-44`：`generate_product_code` 调用 `DocumentNumberGenerator::generate_no(&*self.db, "PRD", product::Entity, product::Column::Code)`，前缀 `PRD`。
- `backend/src/services/product_ops/import_export.rs:65,90,155,266-279`：CSV 导入字段「产品编码」必填，`FieldValidator::required` + `max_length(code, "产品编码", 50)`，但未见导入时做编码重复校验。
- 数据库 DDL `001_consolidated_schema.sql:160`：`COMMENT ON COLUMN products.code IS '产品编码（唯一）'`（仅注释）。
- 数据库 DDL `001_consolidated_schema.sql:163`：`CREATE INDEX IF NOT EXISTS idx_products_code ON products(code);`（**普通索引，非 UNIQUE 索引**）。
- 全库 `grep safe_add_constraint('products'` 无任何命中 —— **products 表未通过 `safe_add_constraint` 添加 UNIQUE 约束**。
- `018_performance_indexes.sql:72-73`：另建 `idx_products_product_code ON products(product_code)`，但 product 表实际字段名为 `code` 而非 `product_code`，该迁移指向了不存在的列（疑似历史遗留脏迁移）。

### 评估结论

**不达标（高风险）**。「产品编码（唯一）」仅存在于代码注释和数据库 COMMENT，数据库层缺少 `UNIQUE` 约束保护，多线程并发导入或绕过 `DocumentNumberGenerator` 的手工录入会产生重复 `code`，业务语义"唯一"无法保障。同时 `018` 迁移引用了不存在的 `product_code` 列，需核查该迁移是否在干净库上会失败。

### 建议（不在本次审计执行范围）

1. 在 `products` 表为 `code` 列补 `UNIQUE` 约束（迁移脚本：`ALTER TABLE products ADD CONSTRAINT uk_products_code UNIQUE (code);`）。
2. `import_export.rs` 的 `validate_code` 增加查重步骤。
3. 核查 `018_performance_indexes.sql:72` 的 `product_code` 列名是否应改为 `code`。

---

## 16.17 色卡面料关联

### 证据

- `backend/src/models/color_card_item.rs:13,31`：色卡明细 `color_card_items` 表持有 `color_card_id: i64`（指向色卡主表）和 `product_color_price_id: Option<i64>`（指向 `product_color_prices`）。
- `backend/src/models/color_card_item.rs:52-57`：`Relation::ColorPrice` 声明 `belongs_to product_color_price::Entity`，色卡明细 → 产品色号价格。
- `backend/src/models/product_color_price.rs:45-47`：`product_color_prices` 表 `belongs_to product_color::Entity`，色号价格 → 产品色号。
- `backend/src/models/product_color.rs:10`：`product_colors` 表，色号实体。
- `backend/src/models/color_card_item_dto.rs:59`、`color_card_response_dto.rs:81`：DTO 同步携带 `product_color_price_id`。
- 链路：`color_card_items.product_color_price_id → product_color_prices.id → product_color_prices.product_color_id → product_colors.id → product_colors.product_id → products.id`。

### 评估结论

**达标**。色卡与面料的关联通过「色卡明细 → 产品色号价格 → 产品色号 → 产品」四级 belongs_to 链路完整建立，SeaORM Relation 定义齐全，外键方向正确。关联字段为 `Option<i64>`，允许色卡明细独立存在（不强制绑定面料），符合"色卡可先建后绑"的业务场景。

### 风险提示

- `color_card_items.product_color_price_id` 为可空，存在"悬挂色卡明细"（无对应价格记录）的可能，建议在色卡发放业务中校验非空。
- 链路跨 4 张表，色卡查询若未做 JOIN 优化可能产生 N+1。

---

## 16.18 供应商认证 / 资质

### 证据

- `backend/src/models/supplier_qualification.rs:6-39`：`supplier_qualifications` 表，字段齐全：
  - `qualification_name`、`qualification_type`、`qualification_no`、`issuing_authority`
  - `issue_date`、`valid_until`、`need_annual_check`、`annual_check_record`
  - `is_expired: bool`、`attachment_path: Option<String>`
- `backend/src/models/supplier_qualification.rs:44-49`：`belongs_to supplier::Entity`，一对多关系正确。
- 未发现独立的「准入」状态字段或「认证」审核流程模型（grep `认证|准入` 在 supplier*.rs 仅命中资质表）。

### 评估结论

**部分达标**。资质信息建模完整（证书名/编号/发证机构/有效期/年检/附件），覆盖供应商资质档案管理需求。但以下两点缺失：

1. **无准入流程**：未见供应商准入审核状态机（如 `approval_status: pending/approved/rejected`），无法区分"待审/准入/淘汰"。
2. **`is_expired` 为静态布尔字段**：依赖手工或定时任务更新，未见基于 `valid_until` 的自动过期计算逻辑（需进一步核查 service 层）。

### 建议

- 评估是否需在 `suppliers` 表增加 `approval_status` 字段支撑准入流程。
- 增加定时任务或查询时计算 `valid_until < now()` 动态判断过期。

---

## 25.10 前端测试覆盖率

### 证据

- `frontend/vitest.config.ts:20-40`：coverage 配置存在，provider `v8`，reporter `text/json/html`，include `src/**/*.{ts,vue}`。
- 门槛 `thresholds`：lines/functions/branches/statements 均为 **1**（注释：「当前 1.78%，逐步提升至 70%」「2026-08-11: 保持 1%，后续通过补充测试逐步提升」）。
- `frontend/tests/` 下测试文件统计：
  - `tests/components/`：1 个（v2-table）
  - `tests/composables/`：1 个（use-table-columns）
  - `tests/unit/`：10 个（utils、v2-table、password-strength-meter、inventory-store、storage、slow-query、audit-log、user-store、request、login）
  - 合计 **12 个 `.test.ts` 文件**。
- `tests/fixtures/`：14 个 fixture 文件（fabric、color_card、inventory、dashboard、auth-mock、user、colorCardIssue、i18n-mock、production_order、sales、dyeing、v2-table 等），覆盖多个业务域。

### 评估结论

**不达标（覆盖率严重不足）**。配置规范但门槛仅设 1%，实际覆盖率约 1.78%，与目标 70% 差距巨大。12 个测试文件主要集中在 utils/store/table 组件等基础设施层，业务页面组件（如色卡、面料、订单、库存等业务模块）的测试基本缺失。Fixture 准备较充分（14 个），但未被对应的业务组件测试消费。

### 建议

1. 每个业务模块至少补 1 个组件测试，优先覆盖色卡发放、面料导入、订单创建等高风险流程。
2. 将 thresholds 按月度阶梯提升（如每月 +5%），避免一次性跳到 70% 导致 CI 红灯。
3. 将 fixtures 与对应业务组件测试配对，避免 fixture 沉淀无人使用。

---

## 26.4 蓝绿部署

### 证据

- `deploy/nginx-upstream-blue.conf`：
  ```
  upstream bingxi_backend {
      server 127.0.0.1:8082;
      server 127.0.0.1:8083 backup;
  }
  ```
- `deploy/nginx-upstream-green.conf`：
  ```
  upstream bingxi_backend {
      server 127.0.0.1:8083;
      server 127.0.0.1:8083 backup;
  }
  ```
- 两份配置文件头注释一致：「P0-D15：nginx upstream 活跃实例配置」「用法：`ln -sf /etc/nginx/bingxi-upstream-{blue|green}.conf /etc/nginx/bingxi-upstream.active.conf`」「切换后执行 `nginx -s reload` 实现零停机流量切换」。
- 另存在 `nginx-upstream-canary-10.conf`、`nginx-upstream-canary-50.conf` 两份金丝雀配置。

### 评估结论

**基本达标，但有配置缺陷**。蓝绿部署机制已建立：双 upstream 配置 + 软链切换 + nginx reload，方向正确。但存在以下问题：

1. **blue 配置中 backup 指向 green 端口（8083）**：当 8082 宕机时流量会 fallback 到 8083，这与"蓝绿隔离"的意图相悖 —— 蓝环境故障会自动切到绿环境，破坏蓝绿独立性。
2. **未发现健康检查配置**：upstream 块未配置 `health_check` 或主动探测，nginx 仅靠被动失败（连接拒绝）才剔除节点。
3. **未见自动化切换脚本**：切换依赖手工 `ln -sf` + `nginx -s reload`，无编排脚本，存在误操作风险。

### 建议

1. 明确蓝绿语义：若希望严格隔离，blue 的 backup 不应指向 green 的 8083；若希望互为容灾，则应在文档中明确说明。
2. 补充 nginx 主动健康检查（`health_check interval=5s fails=3`）或使用 nginx-plus / consul-template 动态 upstream。
3. 编写 `switch-blue-green.sh` 脚本封装切换 + reload + 健康验证。

---

## 26.9 部署回滚

### 证据

- `backend/src/services/system_update_service.rs:15`：facade 注释声明 `backup` 子模块含「备份创建 + 回滚 + 旧备份清理（5 方法）」。
- `backend/src/services/system_update_ops/backup.rs`：
  - `create_backup(version)`（L19-45）：备份 `backend/frontend/config/VERSION` 到 `backups/v{version}_{timestamp}/`。
  - `rollback(backup_path)`（L67-92）：删除当前 `backend/frontend/config` 后从备份拷回，恢复 `VERSION`。
  - `rollback_to_version(version)`（L94-104）：按版本号前缀匹配备份目录并调用 `rollback`。
  - `cleanup_old_backups()`（L106-116）：保留最近 3 个备份，更旧的删除。
- `backend/src/services/system_update_ops/apply.rs:72-89`：更新应用失败时自动调用 `rollback(&backup_path)`，回滚失败仅 `tracing::warn` 记录不中断。
- `backend/src/services/system_update_ops/status.rs:160`：`list_backup_versions()` 列出可用备份版本。
- **未见 health check 与 deploy monitor**：grep `health.*check|deploy.*monitor` 在 system_update*.rs 无任何命中。

### 评估结论

**部分达标**。回滚机制具备基础能力：版本备份、按路径回滚、按版本号回滚、自动失败回滚、旧备份清理（保留 3 份）。但存在以下风险：

1. **回滚是文件级覆盖，非数据库级**：`rollback` 仅还原 `backend/frontend/config/VERSION` 文件，**不回滚数据库 migration**。若更新包包含破坏性 DDL，文件回滚后代码与数据库 schema 不匹配，会导致服务启动失败。
2. **回滚失败被吞错**：`apply.rs:74-75` 回滚失败仅 `tracing::warn`，不向上抛错，调用方收到的是「已回滚」的成功语义，实际可能处于半残状态。
3. **无健康检查**：回滚后未对服务做存活/就绪探测，无法确认回滚是否真正生效。
4. **无部署监控**：缺少部署后指标采集（错误率、延迟、QPS）用于判断是否需要触发回滚。
5. `rollback` 中 `fs::remove_dir_all(&dst)` 先删后拷，若拷贝中途失败会导致目录被清空且无内容，属于非原子操作。

### 建议

1. 回滚流程增加数据库 migration 反向脚本支持，或在回滚前校验 schema 兼容性。
2. 回滚失败必须向上抛错并标记 `is_updating=false` + 告警，禁止吞错。
3. 回滚采用「先拷到临时目录、成功后原子 rename」的方式，避免半完成状态。
4. 回滚后增加 HTTP 健康检查（`GET /health`），失败则告警。
5. 引入部署后观察窗口（如 5 分钟错误率监控），异常自动触发回滚。

---

## 汇总表

| 编号 | 审计项 | 结论 | 风险等级 |
|------|--------|------|----------|
| 16.16 | 面料 SKU 编码 | 不达标 | 高 |
| 16.17 | 色卡面料关联 | 达标 | 低 |
| 16.18 | 供应商认证 | 部分达标 | 中 |
| 25.10 | 前端测试覆盖率 | 不达标 | 高 |
| 26.4 | 蓝绿部署 | 基本达标（有缺陷） | 中 |
| 26.9 | 部署回滚 | 部分达标 | 高 |

---

## 审计约束声明

本次审计严格遵循只读原则：
- 未修改任何业务代码文件
- 未创建 PR、未执行 git push
- 仅在 `.monkeycode/docs/audits/business-deploy-scan.md` 写入审计报告
- 所有结论基于扫描时刻的代码快照

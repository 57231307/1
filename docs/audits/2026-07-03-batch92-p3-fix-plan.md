# 批次 92：P3 修复规划（15 项）

**生成时间**：2026-07-03
**关联复审**：`docs/audits/2026-07-03-reaudit-v3.md` 第五节 P3 问题
**修复目标**：清理占位代码、死代码、panic! 宏、静默吞错、占位 user_id；完善固定资产折旧逻辑与 m0034 表结构

## 修复项清单

| # | 问题 | 文件 | 类型 | 复杂度 |
|---|------|------|------|--------|
| 1 | custom_order_handler 第 537 行无意义占位 `let _ = Entity::find()` | handlers/custom_order_handler.rs | 删除 | 低 |
| 2 | middleware/validation.rs 占位空文件 | middleware/validation.rs | 删除文件 | 低 |
| 3 | print_handler inventory_count_print_html dead_code | handlers/print_handler.rs | 删除函数 | 低 |
| 4 | failover_handler init_global_metrics 死代码 | handlers/failover_handler.rs | 删除函数 | 低 |
| 5 | greige_fabric_handler StockOutRequest dead_code 标注过时 | handlers/greige_fabric_handler.rs | 删除标注 | 低 |
| 6 | models 文件级 #![allow(dead_code)] | models/*.rs | 保留（规范例外） | - |
| 7 | 生产代码 panic!（omni_audit_service / failover_service） | services/*.rs | 改为 Result/日志 | 中 |
| 8 | cli/util 下 let _ = run_cmd 静默吞错（36 处） | cli/util/*.rs | 加日志/关键路径 return | 中 |
| 9 | 占位 user_id（recon 6 处 + color_card 3 处） | services/ar/recon.rs、color_card_crud_service.rs | 签名补 user_id + handler 注入 | 高 |
| 10 | calculate_monthly_depreciation 事务外重复读资产 | services/fixed_asset_service.rs | 拆出纯计算函数 | 中 |
| 11 | batch_calculate_depreciation .all() 缺 LIMIT | services/fixed_asset_service.rs | 加长度校验 + 分页 | 中 |
| 12 | m0034 asset_id 外键无 ON DELETE 策略 | 新增 m0037 迁移 | ALTER TABLE 加 ON DELETE RESTRICT | 中 |
| 13 | idx_fa_depreciation_records_asset 索引冗余 | 同 m0037 迁移 | DROP INDEX | 低 |
| 14 | 折旧未做"已足额折旧"短路 | services/fixed_asset_service.rs | 加短路 + 剩余额封顶 | 中 |
| 15 | data_permission.rs / validation.rs 占位文件清理 | middleware/*.rs | 删两文件 + 移除 mod 引用 | 低 |

**注**：
- 项 6：models 文件级 `#![allow(dead_code)]` 99 个文件属于 SeaORM 自动生成模型，按项目规则六.5 例外保留
- 项 15：与项 2 部分重叠（validation.rs），但 data_permission.rs 仍在 mod.rs 声明，需一并清理
- 项 6.2：event_kafka.rs 第 458 行 panic! 在测试函数内，属合理断言，不修改

## 修复分批

**批次 92（本批次，一次性合并 15 项）**：
- 子批 A：死代码/占位清理（项 1, 2, 3, 4, 5, 15）
- 子批 B：panic/吞错修复（项 7, 8）
- 子批 C：业务逻辑修复（项 9, 10, 11, 12, 13, 14）
- 项 6：保留（规范例外）

## 详细修复方案

### 项 1：custom_order_handler.rs 删除占位代码
- 文件：`backend/src/handlers/custom_order_handler.rs` 第 536-537 行
- 动作：删除 `// 避免未使用变量警告` + `let _ = after_sales_model::Entity::find();`
- 验证：检查 after_sales_model 的 use 是否仍被使用，若仅此处使用需一并删除

### 项 2 + 15：删除占位中间件文件
- 文件：`backend/src/middleware/validation.rs`（仅 2 行注释）
- 文件：`backend/src/middleware/data_permission.rs`（仅 2 行注释）
- 文件：`backend/src/middleware/mod.rs` 第 6 行 `pub mod data_permission;` 删除
- 验证：validation.rs 已不在 mod.rs 中，data_permission.rs 在 mod.rs 但无引用

### 项 3：删除 inventory_count_print_html 函数
- 文件：`backend/src/handlers/print_handler.rs` 第 54-62 行
- 动作：删除整个函数 + 第 61-62 行注释
- 验证：routes/inventory.rs:148 已注释，删除后无破坏

### 项 4：删除 init_global_metrics 死代码
- 文件：`backend/src/handlers/failover_handler.rs` 第 130-134 行
- 动作：删除整个函数（含 doc 注释和 #[allow(dead_code)]）
- 验证：get_global_metrics 已实现懒加载，无需显式 init

### 项 5：删除 StockOutRequest 过时标注
- 文件：`backend/src/handlers/greige_fabric_handler.rs` 第 90 行
- 动作：删除 `#[allow(dead_code)] // TODO(tech-debt): ...` 整行
- 验证：stock_out 路由已在 routes/production.rs:79 挂载，StockOutRequest 已被使用

### 项 7：panic! 替换为 Result/日志
- 文件：`backend/src/services/omni_audit_service.rs` 第 68 行
  - panic!("未设置 AUDIT_SECRET_KEY...") → return Err("...".to_string())
- 文件：`backend/src/services/failover_service.rs` 第 130, 134 行
  - mk_counter/mk_gauge 内 panic → 改用 tracing::error! + unwrap_or_default fallback
  - 注：第 136 行 `Self::new().unwrap_or_else(|_| Self {...})` 自相矛盾，回退分支不应再 panic

### 项 8：cli/util 静默吞错修复（36 处）
- 文件：`backend/src/cli/util/backup.rs`（10 处）
- 文件：`backend/src/cli/util/service.rs`（2 处）
- 文件：`backend/src/cli/util/misc.rs`（4 处）
- 文件：`backend/src/cli/util/upgrade.rs`（20 处）
- 修复模式：
  - 关键路径（systemctl stop/reload、mv 覆盖、mkdir 备份目录）：if let Err(e) = ... { eprintln!("[ERROR] ..."); return; }
  - 清理路径（rm -rf temp、rm -f download）：if let Err(e) = ... { eprintln!("[WARN] ..."); }

### 项 9：占位 user_id 修复（9 处）
- 文件：`backend/src/services/ar/recon.rs`：update/delete/send/dispute/close/update_status 共 6 处
- 文件：`backend/src/services/color_card_crud_service.rs`：update/archive/mark_lost 共 3 处
- 修复：
  1. service 方法签名补 `user_id: i32` 参数
  2. `Some(0)` → `Some(user_id)`
  3. 删除 TODO 注释
  4. handler 层从 AuthContext 注入 user_id
- 注：mark_lost 未接入路由（带 #[allow(dead_code)]），签名补全但 user_id 暂用占位 0

### 项 10：拆出纯计算函数避免事务外重复读
- 文件：`backend/src/services/fixed_asset_service.rs`
- 动作：
  1. 新增私有 `fn calc_monthly_depreciation_for(asset: &fixed_asset::Model) -> Result<Decimal, AppError>`
  2. `calculate_monthly_depreciation(asset_id)` 改为：先 get_by_id 再调用纯计算版
  3. `depreciate` 中第 223 行改为 `Self::calc_monthly_depreciation_for(&asset)?`
- 收益：消除 TOCTOU 风险，行锁覆盖完整

### 项 11：batch_calculate_depreciation 加 LIMIT 保护
- 文件：`backend/src/services/fixed_asset_service.rs` 第 440-482 行
- 动作：
  1. 入口校验：`if asset_ids.len() > 10_000 { return Err(...); }`
  2. 查询改用 `.paginate(&*self.db, 1000)` 流式

### 项 12 + 13：新增 m0037 迁移
- 文件：`backend/migrations/20260703000006_alter_fa_depreciation_records_fk/up.sql`
  ```sql
  BEGIN;
  ALTER TABLE "fixed_asset_depreciation_records"
      DROP CONSTRAINT IF EXISTS "fixed_asset_depreciation_records_asset_id_fkey";
  ALTER TABLE "fixed_asset_depreciation_records"
      ADD CONSTRAINT "fixed_asset_depreciation_records_asset_id_fkey"
      FOREIGN KEY ("asset_id") REFERENCES "fixed_assets"("id")
      ON DELETE RESTRICT;
  DROP INDEX IF EXISTS "idx_fa_depreciation_records_asset";
  COMMIT;
  ```
- 文件：`backend/migrations/20260703000006_alter_fa_depreciation_records_fk/down.sql`
  ```sql
  -- 反向恢复：重建单列索引，外键回到默认 NO ACTION
  CREATE INDEX IF NOT EXISTS "idx_fa_depreciation_records_asset"
      ON "fixed_asset_depreciation_records"("asset_id");
  -- 外键 ON DELETE 策略变更不可逆，保持 RESTRICT
  ```
- 文件：`backend/migration/src/m0037_alter_fa_depreciation_records_fk.rs`（参考 m0034 模板）
- 文件：`backend/migration/src/lib.rs` 注册 m0037

### 项 14：折旧"已足额折旧"短路
- 文件：`backend/src/services/fixed_asset_service.rs` `depreciate` 函数
- 动作：
  1. 行 215（状态校验后）追加：判断 accumulated_depreciation >= original_value - residual_value 则 return Ok(())
  2. 行 223（计算后）追加：monthly_depreciation = monthly_depreciation.min(remaining)
  3. 行 225（零值校验）改为：if monthly_depreciation <= Decimal::ZERO { return Ok(()); }

## CI 验证策略

- 编译错误修复后 push → CI 全绿（13 项检查，E2E continue-on-error 非阻塞）→ squash merge
- 关键检查：Rust 后端构建、Rust Clippy、Rust 单元测试、Rust 格式检查

## 进度跟踪

| 子批 | 项 | 状态 |
|------|----|----|
| A | 1, 2, 3, 4, 5, 15 | ✅ 代码完成 |
| B | 7, 8 | ✅ 代码完成 |
| C | 9, 10, 11, 12, 13, 14 | ✅ 代码完成 |
| 提交 | PR + CI + 合并 | ⬜ 待执行 |

## 实施记录

### 子批 A 死代码/占位清理（已完成）
- 项 1：`custom_order_handler.rs` 删除占位 `let _ = Entity::find()` + 未使用的 use
- 项 2+15：删除 `middleware/validation.rs`、`middleware/data_permission.rs` 占位文件 + 移除 mod.rs 引用
- 项 3：`print_handler.rs` 删除 `inventory_count_print_html` 函数 + `routes/inventory.rs` 删除已注释路由块
- 项 4：`failover_handler.rs` 删除 `init_global_metrics` 死代码（懒加载已实现）
- 项 5：`greige_fabric_handler.rs` 删除过时 `#[allow(dead_code)]` 标注，并在 stock_out 中实际使用 `req.remarks`

### 子批 B panic/吞错修复（已完成）
- 项 7：
  - `omni_audit_service.rs`：panic! 改为 `match` 表达式 + `return Err`（闭包 return 陷阱修复）
  - `failover_service.rs`：panic 前加 `tracing::error!` 日志，`unwrap_or_else` 修复错误变量捕获
- 项 8：`cli/util/{backup,service,misc,upgrade}.rs` 共 36 处 `let _ = run_cmd(...)` 改为带日志的错误处理
  - 关键路径（mkdir、stop/reload nginx、tar 解压、mv 覆盖）：失败时 `return` 中止
  - 清理路径（rm -rf temp、find/rm）：失败仅告警

### 子批 C 业务逻辑修复（已完成）
- 项 9：占位 user_id 全部替换为真实注入
  - `ar/recon.rs`：6 个方法签名补 `user_id: i32`，删除 `Some(0)` 占位
  - `color_card_crud_service.rs`：3 个方法签名补 `user_id: i32`
  - `ar_reconciliation_handler.rs`：4 处调用注入 `auth.user_id`
  - `color_card/crud.rs`：2 处调用注入 `auth.user_id`
- 项 10：`fixed_asset_service.rs` 拆出纯计算函数
  - 新增 `fn calc_monthly_depreciation_for(asset: &Model) -> Result<Decimal, AppError>`
  - `calculate_monthly_depreciation(asset_id)` 改为异步包装（handler 入口保留）
  - `depreciate` 改用 `Self::calc_monthly_depreciation_for(&asset)?`，消除事务外重复读
- 项 11：`batch_calculate_depreciation` 加保护
  - 入口校验：空列表返回空，>10_000 拒绝
  - 查询改用 `.paginate(&*self.db, 1000)` 流式拉取
- 项 12+13：新建 m0037 迁移
  - `migrations/20260703000006_alter_fa_depreciation_records_fk/up.sql`：外键 ON DELETE RESTRICT + DROP 冗余索引
  - `down.sql`：恢复默认外键 + 恢复冗余索引
  - `migration/src/m0037_alter_fa_depreciation_records_fk.rs`：迁移实现
  - `migration/src/lib.rs`：注册 m0037
- 项 14：折旧"已足额折旧"短路
  - `depreciate` 函数加 `accumulated_depreciation >= depreciable_cap` 短路
  - `new_accumulated` 封顶到 `depreciable_cap`（最后一期可能小于月折旧额）
  - 折旧记录 `depreciation_amount` 改用 `actual_depreciation`（封顶后实际计提额）
  - 零值校验改为 `return Ok(())`（已足额折旧或使用寿命为 0 时幂等返回）


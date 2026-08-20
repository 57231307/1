# 迁移体系稳健版重构 + CI 修复 实施计划

> 范围：方案 A 稳健版。保留 11 个聚合模块的域划分与编号，删空/重复迁移，修顺序 bug，吸收 parse_bullets。
> 前提：项目在测试阶段，无生产部署，但 SeaORM `seaql_migrations` 版本号机制仍需谨慎（删迁移对全新库安全，对已应用库需 drop 重建）。

## 已确认事实（规划依据）

- 128 个 Rust 迁移（m0001-m0116）是当前**唯一权威、活跃执行**的迁移；`legacy-migration-snapshots` 的 35 个 SQL 已删除归档，无需处理。
- 真空迁移仅 1 个：`m0025`（已是保留空实现，注释说明为保持迁移历史顺序，**不删**——删了有版本号风险，无收益）。
- 26 张表被重复 CREATE（用 IF NOT EXISTS 幂等），重点重复迁移：m0017（重复 m0008/m0012/m0013 的 11 张表）、m0018（重复 m0012 的 9 张表）、m0044（重复 m0021/m0022/m0023）。
- m0017/m0018 经核实**纯 CREATE 无 ALTER**，但列定义可能与原表不同，需逐表对比。
- E2E 失败根因：m0032 在 m0044（创建 custom_orders）之前执行 → 表不存在 → 迁移中断；m0029 也在 m0044 之前（顺序 bug，与 m0044 注释矛盾）。
- CI Release 失败根因：`generate-release-notes.py:75` 调用未定义的 `parse_bullets`。

- [ ] 1. 修复 CI Release 脚本（parse_bullets NameError）
  - [ ] 1.1 在 `.github/scripts/generate-release-notes.py` 实现解析 conventional commit bullet points 的函数，返回 `[{"type","scope","desc"}]`
  - [ ] 1.2 本地构造含 `- feat(scope): desc` 的 commit 端到端验证 release_notes.md 正确分类
  - [ ]* 1.3 验证空 body / None 输入返回 `[]` 不抛异常

- [ ] 2. 修复 E2E 迁移链中断（m0032/m0029/m0044 顺序与表存在性）
  - [ ] 2.1 `m0032`：用 `DO $$ IF EXISTS(information_schema.tables)` 包裹 ALTER，custom_orders 表不存在时安全跳过
  - [ ] 2.2 `m0044`：CREATE TABLE custom_orders 补 `notes` 列（与 m0032 幂等对齐，避免 notes 列缺失）
  - [ ] 2.3 `production_quality.rs`：调整 up 调用顺序，m0044 在 m0029 之前（符合 m0044 设计意图"注册在 m0028 之后、m0029 之前"），down 保持逆序

- [ ] 3. 核实重复迁移的列定义差异（删重复的安全性前提）
  - [ ] 3.1 逐表对比 m0017 vs m0008/m0012/m0013 重复表（crm_lead/crm_opportunity/supplier_*/purchase_contracts/sales_contracts/customer_credit_ratings）的列定义，确认 m0017 是否纯重复还是有额外列
  - [ ] 3.2 逐表对比 m0018 vs m0012 重复表（ap_invoice/ar_invoices/budget_plans/fixed_assets/fund_accounts 等 9 张）的列定义
  - [ ] 3.3 对比 m0044 vs m0021/m0022/m0023（sales_quotations/items/terms）的列定义，确认 m0044 是否补列（m0044 已知补了 notes 列，需确认其余）
  - [ ] 3.4 对比 m0020 vs m0005（log_login/omni_audit_logs）、m0078 vs m0008（piece_mapping）、m0069 vs m0017（supplier_evaluation_records）

- [ ] 4. 检查点 - 确认哪些重复迁移可安全删除
  - 基于步骤 3 对比结果，列出"纯重复（列定义一致）可删"与"补字段（不可删）"两类。纯重复的迁移可从对应聚合模块的 .rs 中移除调用并删除文件；补字段的保留。确保所有疑问已澄清。

- [ ] 5. 删除确认为纯重复的迁移
  - [ ] 5.1 从对应聚合模块 .rs 的 up/down 调用中移除纯重复迁移的调用
  - [ ] 5.2 删除纯重复迁移的 .rs 文件及其 mod 声明
  - [ ] 5.3 确保删除后各聚合模块 up/down 顺序仍自洽

- [ ] 6. 建分支提交并验证
  - [ ] 6.1 从最新 main 建 `fix/migration-refactor-stable` 分支
  - [ ] 6.2 提交所有改动（parse_bullets + 顺序修复 + 删重复迁移）
  - [ ] 6.3 推送并创建 PR（按 `.github/PULL_REQUEST_TEMPLATE.md` 模板填写描述）
  - [ ] 6.4 监控 CI：等待 Rust 构建/Clippy/30 测试分区全绿，失败拉 annotations 修复
  - [ ]* 6.5 CI 全绿后触发 E2E 批次工作流验证迁移链能完整执行（可选，需 workflow_dispatch）

- [ ] 7. 检查点 - CI 全绿后合并
  - 确保 CI 必检全绿后 squash 合并 PR，清理本地与远程分支

## 风险与约束

- **不删 m0025**（唯一空迁移，删有版本号风险无收益）
- **删重复迁移前必须完成步骤 3 逐表列对比**，纯重复才删，补字段的保留
- 全新测试库迁移链必须能完整执行（m0032→跳过、m0044→建表+notes、m0029→DROP COLUMN 安全）
- SeaORM 迁移版本号：删迁移对全新库安全；如存在已应用库需 `bingxi migrate fresh` 重建
- 不重排 169 张表依赖拓扑序（稳健版核心：不引入新顺序风险）

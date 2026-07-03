# 全项目复审 v2 修复规划（批次 85-87）

**复审报告**：[2026-07-03-reaudit-v2.md](file:///workspace/docs/audits/2026-07-03-reaudit-v2.md)
**审计基线**：main HEAD `c40dd511`（批次 78-84 全部修复完成后）
**问题总数**：56 项（P1×9 / P2×19 / P3×28）
**规划批次**：3 批（批次 85-87）

## 修复策略

延续 v1 批次修复流程：
1. 按 P1 → P2 → P3 优先级分批，每批 CI 全绿后 squash merge 到 main
2. 复用 v1 已建立的标准修复模式（lock_exclusive 串行化 / round_dp(2) 精度校验 / startsWith 前缀匹配等）
3. P1 集中在维度 1（事务边界 TOCTOU）和维度 5（并发竞态），是本轮核心
4. P2 维度 1 的"有 txn 但无 lock_exclusive"9 项与 P1 同类问题，作为加固项合并到批次 85 后段或单独批次 86 处理
5. 全部完成后启动 v3 第三轮复审

## 批次规划

### 批次 85：P1 修复 — 事务边界 TOCTOU + 并发竞态（9 项）

**主题**：8 处 update/approve/check+insert 事务边界 TOCTOU + 1 处 submit/self_approve 状态门+锁组合修复
**级别**：P1
**项数**：9
**修复分支**：`fix/v19-batch85-tx-boundary-p1`

| # | 文件 | 方法 | 问题 | 修复 |
|---|------|------|------|------|
| P1-1 | custom_order_crud_service.rs:168-213 | update | 状态门在 self.db，update 也在 self.db，无 txn 无 lock | begin txn + find_by_id(id).lock_exclusive().one(&txn) + 状态门 + update_with_audit(&txn) + commit |
| P1-2 | quotation_service.rs:211-363 | update | 状态门在 txn 外，line 360 update 在 txn commit 后 | 将状态门移入 txn + lock_exclusive，update 移到 commit 前 |
| P1-3 | cost_collection_service.rs:388-417 | approve | 全程无 txn 无 lock_exclusive | 用 txn 包裹 find_by_id + lock_exclusive + 状态门 + update + commit |
| P1-4 | fixed_asset_service.rs:188-242 | depreciate | 状态门在 txn 外 | 状态门移入 txn + lock_exclusive |
| P1-5 | fixed_asset_service.rs:245-304 | dispose | 状态门在 txn 外 | 状态门移入 txn + lock_exclusive |
| P1-6 | customer_service.rs:92-154 | create_customer | 检查编码存在 + insert 无 txn | begin txn + 检查存在（lock_exclusive 串行化） + insert + commit |
| P1-7 | field_permission_service.rs:117-164 | create_field_permission | 检查存在 + insert 无 txn | 同上模式 |
| P1-8 | data_permission_service.rs:124-165 | set_data_permission | find + update/insert 无 txn 无锁 | begin txn + lock_exclusive + upsert + commit |
| P1-9 | quotation_approval_service.rs:96-153 | submit + self_approve | submit 状态门在 self.db 无 lock；self_approve 有 lock 但无状态检查 | submit：状态门移入 txn + lock_exclusive；self_approve：在 lock 后加状态门检查 |

### 批次 86：P2 修复（19 项）

**主题**：9 处 update/delete 加 lock_exclusive + 金额精度校验补齐 + 全表加载 LIMIT + 前端 any 清理 + v-permission 编辑/删除按钮 + TraceLayer IP 修复
**级别**：P2
**项数**：19
**修复分支**：`fix/v19-batch86-p2-hardening`

| 维度 | # | 文件 | 修复 |
|------|---|------|------|
| 1 事务边界 | P2-1~P2-9 | role_service / ar_invoice_service / ap_invoice_service / ap_payment_request_service / customer_credit_limit / fixed_asset_service delete | 已有 txn 内 find_by_id 后追加 .lock_exclusive() 串行化 |
| 2 输入验证 | P2-10 | ar_invoice_service.rs:298-302 update | invoice_amount 加 round_dp(2) 精度校验（补 v1 批次 84 P2-4 遗漏） |
| 2 输入验证 | P2-11 | sales_fabric_order_handler.rs:175-184 | f64 → Decimal + 非负校验 + round_dp(2) |
| 6 N+1 | P2-12 | ai/rec.rs:170-171 | get_abc_classification 加 LIMIT 兜底 |
| 6 N+1 | P2-13 | ai/rec.rs:617-618 | generate_price_recommendations 加 LIMIT 兜底 |
| 9 前端类型 | P2-14 | bi.ts:160-193 | 5 处 BiResponseData<any> → 显式接口 |
| 9 前端类型 | P2-15 | ar.ts:175-187 | 4 处 ApiResponse<any> → 显式接口 |
| 9 前端类型 | P2-16 | report-enhanced/print-templates/api-gateway/product/sales-analysis | 6 处 ApiResponse<any> → 显式接口 |
| 10 路由权限 | P2-17 | 17+ .vue 文件 | 编辑/删除按钮批量补齐 v-permission |
| 11 测试质量 | P2-18 | inventory-store.test.ts | 6 处 `as any` → 显式类型断言 |
| 12 安全性 | P2-19 | main.rs:513-519 | TraceLayer IP 提取优先级对齐 audit_context（X-Real-IP → X-Forwarded-For → ConnectInfo） |

### 批次 87：P3 修复（28 项）

**主题**：低优先级清理 — 错误处理规范化 + 金额计算归一化 + LIMIT 兜底补齐 + TODO 注释 + 测试命名 + IP 提取统一
**级别**：P3
**项数**：28
**修复分支**：`fix/v19-batch87-p3-cleanup`

| 维度 | 项数 | 修复要点 |
|------|------|---------|
| 3 错误处理 | 5 | expect/unwrap 改为 ok_or_else / map_err / unwrap_or_default |
| 4 业务逻辑 | 7 | 金额计算统一 round_dp(2)，状态字符串提取常量 |
| 6 N+1 | 3 | 全表加载补 LIMIT 兜底 |
| 8 死代码 | 2 | reason 属性补 TODO 注释 |
| 9 前端 API | 2 | 索引签名 any → unknown / 显式接口 |
| 10 路由权限 | 2 | v-permission 遗漏补齐 |
| 11 测试质量 | 3 | 测试文件命名风格统一 |
| 12 安全性 | 2 | IP 提取 helper 统一复用 |

**延后项**（按 v1 经验，部分 P3 项可能延后到下一迭代，需在 PR 描述中明确标注）

### 批次 88：占位符功能实现（批次 85 评估发现，3 项）

**主题**：批次 85 P1 修复过程中发现的占位符/未完善功能实现
**级别**：功能完善（需 schema 变更）
**项数**：3
**修复分支**：`fix/v19-batch88-placeholder-impl`

| # | 文件 | 占位符 | 评估 | 实现方案 |
|---|------|--------|------|----------|
| PH-1 | custom_order_crud_service.rs:218-220 | `if let Some(v) = dto.notes { let _ = v; }` 注释"暂存到 yarn_spec 字段（无 notes 字段；如有需要扩展 schema）" | custom_order 模型无 notes 字段，DTO 有 notes 但被丢弃 | 新增 migration 添加 notes 列 + 修改 ActiveModel + update 方法接入 notes 字段 |
| PH-2 | fixed_asset_service.rs:191 depreciate | `period: &str` 参数只用于日志，未按期间记录折旧 | 无折旧期间记录表，无法跟踪每个期间的折旧 | 新增 fixed_asset_depreciation_records 表 + migration + 折旧时插入期间记录 |
| PH-3 | fixed_asset_service.rs:287 dispose | `let _disposal_gain_loss = req.disposal_value - net_book_value;` 计算后未使用 | fixed_asset_disposal 模型无 gain_loss 字段 | 新增 migration 添加 gain_loss 列 + 修改 ActiveModel + dispose 方法写入 gain_loss |

**说明**：这 3 项占位符实现都需要 schema 变更（migration），超出 P1 事务边界修复范围，安排到专门批次以降低 CI 失败风险。

## 进度跟踪

| 批次 | 主题 | 级别 | 项数 | 状态 |
|------|------|------|------|------|
| 85 | 事务边界 TOCTOU + 并发竞态 | P1 | 9 | 🔄 修复中 |
| 86 | P2 加固（lock_exclusive + 精度 + N+1 + 前端 + 安全） | P2 | 19 | ⬜ 待启动 |
| 87 | P3 清理（错误处理 + 金额 + LIMIT + 测试） | P3 | 28 | ⬜ 待启动 |
| 88 | 占位符功能实现（schema 变更） | 功能完善 | 3 | ⬜ 待启动 |

**全部完成后**：v3 第三轮复审，循环直到无问题

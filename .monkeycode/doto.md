# 任务与历史

> 本文件记录**当前任务**与**历史任务索引**。
> 详细历史请查阅 [`.monkeycode/docs/archives/`](file:///workspace/.monkeycode/docs/archives/)。

---

## 当前活跃任务（2026-06-24）

### 🔴 安全审计：周期性漏洞评估

**状态**：审计完成，P0/P1/P2 漏洞待修复
**详细记录**：[bug.md](file:///workspace/.monkeycode/bug.md)
**审计时间**：2026-06-24
**审计方法**：人工审计，聚焦 4 大高风险攻击面（认证/注入/外部交互/敏感数据）

#### 漏洞汇总

| 编号 | 等级 | 漏洞 | 位置 | 状态 |
|------|------|------|------|------|
| #1 | 🔴 P0 | 静态资源路径遍历 | `routes/static.rs:22-57` | 待修复 |
| #2 | 🔴 P0 | WebSocket 认证绕过 | `websocket/notifications.rs:198-222` | 待修复 |
| #3 | 🟠 P1 | 初始化接口匿名访问 | `middleware/public_routes.rs:9` | 待修复 |
| #4 | 🟡 P2 | 错误信息泄露内部细节 | `utils/error.rs:297-315` | 待修复 |
| #5 | 🟡 P2 | API Key 撤销后仍可冒用 | `services/api_key_service.rs:69-85` | 待修复 |
| #6 | 🟡 P2 | 内存限流器多实例失效 | `middleware/rate_limit.rs:76-79` | 待修复 |
| #7 | 🟡 P2 | 弱密码黑名单策略不严 | `utils/password_validator.rs:84-100` | 待修复 |
| #8 | 🟡 P2 | 调试模式错误响应泄露 | `utils/error.rs:88` | 待修复 |

#### 关键修复要点

- **#1 路径遍历**：`std::fs::canonicalize()` + 目录范围校验，过滤 `..` 和 `//`
- **#2 WebSocket 认证**：复用 `validate_token_static()`，删除占位 `verify_jwt_token`
- **#3 初始化接口**：增加 `INIT_TOKEN` 环境变量校验
- **#4/#8 错误响应**：强制 `APP_ENV=production`，脱敏 `error_type`/`detail`
- **#5 API Key 黑名单**：Redis 存储已撤销 key_hash
- **#6 分布式限流**：使用 Redis `INCR` + `EXPIRE` 原子操作
- **#7 弱密码策略**：COMMON_PASSWORDS 命中硬性拒绝而非扣分

#### 修复任务分解

- [ ] P0：#1 静态资源路径遍历（1-2 天）
- [ ] P0：#2 WebSocket 认证绕过（1-2 天）
- [ ] P1：#3 初始化接口匿名访问（3-5 天）
- [ ] P2：#4-#8 批量修复（2-3 周）

---

### ✅ CI 错误修复（PR #248）

**状态**：已合并
**PR**：[#248](https://github.com/57231307/1/pull/248)
**合并 commit**：`cd7f6b5e`
**分支**：`fix/ci-clippy-activevalue-error-2026-06-24`
**CI 结果**：✅ 15 个 job 全绿

#### 问题
`backend/tests/color_price_crud_test.rs:90` 错误调用 `active.is_active.is_ok()`：
- `active.is_active` 类型是 `sea_orm::ActiveValue<bool>`，**不是** `Result`
- 没有 `is_ok()` 方法
- 原代码 `assert!(active.is_active.is_ok() || true);` 中 `|| true` 是恒真式，掩盖了编译错误

#### 影响
- CI 编译失败 `error[E0599]` → cargo clippy 无法完成 → 误报 884-1178 个"新警告"

#### 修复
1. 改用 `match` 模式匹配 `ActiveValue::Set(v)` 变体
2. 删除损坏的 `backend/.clippy-baseline.txt`，让 CI 重建基线

#### 关键经验
- **`|| true` 反模式**：恒真式断言掩盖编译错误
- **Clippy Baseline 脆弱性**：`sort -u` 处理多行 `rendered` 字段失效
- **TODO 改进**：CI 改用 `jq` 提取结构化标识符（`code` + `message` + `span`）

---

### ✅ 批次 C dead_code 清理（PR #247）

**状态**：已合并
**PR**：[#247](https://github.com/57231307/1/pull/248)
**合并 commit**：`f524dad7`
**分支**：`fix/clippy-deadcode-batch-c-2026-06-24`
**CI 结果**：✅ 15 个 job 全绿

#### 范围
- 40 个低频 dead_code 警告后端文件（8 轮 × 5 个子代理）
- 修复 12 个集成测试文件的 `use crate::` 错误导入（共 20 处）
- 删除并重建 `backend/.clippy-baseline.txt`

#### 集成测试导入修复清单
- tests/bi_analysis_test.rs
- tests/color_card_borrow_test.rs
- tests/color_card_crud_test.rs
- tests/color_card_e2e_test.rs
- tests/color_card_item_test.rs
- tests/color_card_scan_test.rs
- tests/custom_order_e2e_test.rs（2 处）
- tests/custom_order_process_test.rs
- tests/custom_order_state_test.rs
- tests/quotation_e2e_test.rs（4 处）
- tests/quotation_handler_test.rs（5 处）
- tests/websocket_test.rs

#### 关键决策
1. 集成测试 `crate` 语义：`tests/` 目录下的 `crate` 指测试二进制；引用 lib 模块用 `use bingxi_backend::`
2. 损坏的 clippy baseline（970 个"新警告"误报）→ 删除让 CI 重建
3. 8 轮 × 5 子代理并行处理结构

---

## 下一步计划

### 批次 D：跨文件清理与基线更新
- 范围：剩余 dead_code 警告 + clippy baseline 重建（已自动完成）
- 关键：处理 PR #248 后未涉及的 4-7 个高价值清理
- 预计时间：1-2 天

### CI 脚本改进（TODO）
- `backend/.clippy-baseline.txt` 生成改用结构化标识符（`jq` 提取 `code` + `message` + `span`）
- 原因：当前 `sort -u` 处理多行 `rendered` 字段失效
- 文件位置：`.github/workflows/ci-cd.yml:405-416`

### 中期任务
- 完成 clippy dead_code 清理全量覆盖（高频/中频/低频已完成首轮）
- 安全漏洞 4 waves 全部修复完成
- 持续监控新警告增量

---

## 历史任务索引

### 2026-06-24
- [PR #245 批次 A 清理](file:///workspace/.monkeycode/docs/archives/doto-2026-06-24-pre-optimization.md) - 20 个高频 dead_code 文件
- [PR #246 批次 B 清理](file:///workspace/.monkeycode/docs/archives/doto-2026-06-24-pre-optimization.md) - 30 个中频 dead_code 文件
- PR #247 批次 C 清理 - 40 个低频 dead_code + 12 测试导入（见上）
- PR #248 CI 错误修复（见上）

### 2026-06-23
- [批次 A/B/C 整体规划与启动](file:///workspace/.monkeycode/docs/archives/doto-2026-06-24-pre-optimization.md)
- 修复 CI 误报问题

### 2026-06-22
- [项目真实运行问题检测](file:///workspace/.monkeycode/docs/archives/doto-2026-06-24-pre-optimization.md) - 80/100

### 2026-06-19
- [路由/API 审计](file:///workspace/.monkeycode/docs/archives/doto-2026-06-24-pre-optimization.md)
- [现代代码质量审计 73/100](file:///workspace/.monkeycode/docs/audits/2026-06-19-modern-code-audit.md)
- [Clippy 死代码深度预判](file:///workspace/.monkeycode/docs/audits/2026-06-19-clippy-deep-prediction.md)

### 2026-06-16
- [API 100% 完整度报告](file:///workspace/.monkeycode/docs/archives/doto-2026-06-24-pre-optimization.md)

### 2026-06-07
- [日志诊断技能自动触发](file:///workspace/.monkeycode/docs/archives/doto-2026-06-24-pre-optimization.md)

### 2026-05-29
- [部署限制规范](file:///workspace/.monkeycode/docs/archives/doto-2026-06-24-pre-optimization.md) - 不安装 PG/Redis/Docker

### 2026-05-27
- [服务器环境信息](file:///workspace/.monkeycode/docs/archives/doto-2026-06-24-pre-optimization.md) - bingxi-backend systemd
- [工作角色定位](file:///workspace/.monkeycode/docs/archives/doto-2026-06-24-pre-optimization.md) - 主代理/子代理分工

---

## 详细归档

完整历史任务与原始记录：

- 完整 doto：`.monkeycode/docs/archives/doto-2026-06-24-pre-optimization.md`
- 完整 MEMORY：`.monkeycode/docs/archives/MEMORY-2026-06-24-pre-optimization.md`
- 完整 CHANGELOG：`.monkeycode/docs/archives/CHANGELOG-2026-06-24-pre-optimization.md`

# 任务与历史

> 本文件记录**当前任务**与**历史任务索引**。
> 详细历史请查阅 [`.monkeycode/docs/archives/`](file:///workspace/.monkeycode/docs/archives/)。

---

## 当前活跃任务（2026-06-24）

### ✅ bug.md 8 个安全漏洞全部修复（PR #250）

**状态**：已合并
**PR**：[#250](https://github.com/57231307/1/pull/250)
**合并 commit**：`1e6ba7da`（squash merge）
**分支**：`fix/security-p0-2026-06-24`
**CI 结果**：✅ 12 个 job 全绿（clippy + build + test + 依赖审计 + 前端）
**bug.md**：已简化为空占位文件（5 行）

#### 8 个漏洞修复明细
| # | 等级 | 漏洞 | 关键修复 | 关联 commit |
|---|------|------|----------|-------------|
| #1 | P0 | 路径遍历 | 文件下载路径校验 + 沙箱化 | `ee5fda48` |
| #2 | P0 | WebSocket 认证绕过 | ws 握手 + JWT 校验 | `ee5fda48` |
| #3 | P1 | init_token 缺失 | 新增 init_token 中间件（subtle::ConstantTimeEq） | `373e132e` |
| #4 | P2 | 错误响应信息泄漏 | 错误响应脱敏（移除 error_type/detail） | `b47c4108` |
| #5 | P2 | API Key 撤销失效 | 撤销写黑名单 + is_api_key_revoked 检查 | `3d193937` / `2419a8bc` / `82909402` |
| #6 | P2 | 分布式限流缺失 | Redis INCR+EXPIRE + 内存回退 | `62efbc5f` |
| #7 | P2 | 弱密码接受 | Top 100 黑名单 + l33t 归一 + 键盘序列 | `8390380c` |
| #8 | P2 | 错误响应类型泄漏 | 与 #4 同步脱敏 | `b47c4108` |

#### 12 个 commit 累计修复
1. `ee5fda48` #1 #2 P0 修复
2. `9ebaef5a` ESLint vue/no-mutating-props
3. `373e132e` #3 init_token 中间件
4. `b47c4108` #4 #8 错误脱敏
5. `3d193937` #5 API Key 黑名单
6. `62efbc5f` #6 分布式限流
7. `8390380c` #7 弱密码严格化
8. `e1988f74` docs 记录
9. `2419a8bc` #5 修复补充（Cache trait import）
10. `82909402` #5 修复补充（移除错误 .copied()）
11. `ebf4ada7` CI 失败修复（3 个：rate_limit 回退 / GanttItemDto 字段 / 未用导入）
12. `ab9c4396` 删除损坏 clippy baseline
**`1e6ba7da` squash merge**

#### 关键文件变更
- `backend/src/middleware/init_token.rs` (新增)
- `backend/src/middleware/rate_limit.rs` (分布式限流 + 内存回退)
- `backend/src/services/api_key_service.rs` (黑名单机制)
- `backend/src/utils/error.rs` (响应脱敏统一化)
- `backend/src/utils/password_validator.rs` (黑名单扩展)
- `backend/src/handlers/api_key_handler.rs` (传入 cache)
- `backend/tests/test_scheduling.rs` (补全字段)
- `backend/tests/ai_extend_test.rs` (清理未用导入)
- `backend/.clippy-baseline.txt` (删除 - 损坏)

#### 关键经验教训（详见 MEMORY.md / CHANGELOG.md）
- **分布式限流回退逻辑必须真正回退**：`check_redis_rate_limit` 返回 `Ok(None)` 与 `Err(_)` 等价，都回退内存限流
- **clippy baseline 脆弱性**：`sort -u` 对多行 `rendered` 字段去重错误；删除损坏 baseline 让 CI 重建
- **`Cache::get()` 返回 `Option<V>`（已 Clone）**：不能调用 `.copied()`（仅 Option<&T> 或迭代器支持）
- **`Clippy --release` 才会暴露**某些 dev build 不触发的编译错误（如 `.copied()` on owned Option）

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

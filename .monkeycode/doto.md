# 未完成任务

> 本文件只记录**未完成的任务**（任务队列、待修复项）。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。

---

## 🔄 当前任务：v14 深度调研报告修复（高风险 6/6 完成，中风险 14/25 完成）

> **v14 深度调研报告**（2026-07-09，[bug.md](file:///workspace/.monkeycode/bug.md)）：12 维度全量扫描，15 高/25 中/74 低风险，共 114 个问题。
> v13 后端 P0/P1 全部完成（批次 229-236），v13 剩余 P2 任务合并到 v14 队列。
> 修复策略：按优先级（高→中→低）+ 影响范围（核心路径→边缘功能）排序，每批 1 commit，CI 全绿后合并 main。

### v14 修复任务队列

#### 🔴 高风险修复队列（6 项，全部完成 ✅）

| 批次 | 编号 | 问题 | 状态 |
|------|------|------|------|
| 237 | P0-1 | 并发-async 阻塞（spawn_blocking 包装 Argon2id） | ✅ PR #414 |
| 238 | P0-2 | 性能-全表扫描（ar_service SQL 聚合） | ✅ PR #415 |
| 239 | P0-3 | 空实现-业务失效（handleView 只读模式） | ✅ PR #416 |
| 240 | P0-4 | 测试覆盖-安全核心（permission.rs 23 测试） | ✅ PR #417 |
| 241 | P0-5 | API 文档缺失（恢复 docs.rs + 删 openapi.rs） | ✅ PR #418 |
| 242 | P0-6 | 简化阉割-RFM 分布真实计算 | ✅ PR #419 |

#### 🟡 中风险修复队列（25 项，已完成 14/25 🔄）

**待修复项（11 项 ⏳）**：

- **测试覆盖（7 项 ⏳ 待修复）**：
  - handlers 100+ 文件覆盖率 10%
  - services 107 个无测试
  - frontend api 4.4%
  - ai 算法零测试
  - store 覆盖率低
  - middleware 覆盖率低
  - 其他模块测试补测
- **重复实现（2 项 🔄 进行中）**：
  - service 分页接入 paginate_with_total（累计 8/35 完成，剩余 27/35 ⏳；批次 257 第三批 PR #434 进行中）
  - 30+ view 表格逻辑接入 useTableApi（⏳ 待修复）

**已完成项（13 项 ✅）**：
- 空实现（4 项 ✅）：批次 246 handleViewVersion + 批次 252 bi_analysis unreachable! + 批次 253 AdvancedFilter handleLogicChange
- 简化阉割（3 项 ✅）：批次 249 capacity + 批次 250 budget + 批次 251 webhook retry
- 死代码（1 项 ✅）：批次 254 composable eslint-disable any 清理
- 重复实现 service 分页首批（1 项 ✅）：批次 255 + 批次 256（累计 8/35）
- 项目规则符合性（1 项 ✅）：批次 247 CLI 硬编码 URL
- 性能问题（5 项 ✅）：批次 244 ar 报表 + 批次 245 ap 报表 + 批次 248 缓存接入
- 安全漏洞（2 项 ✅）：批次 243 XSS + 输入验证

#### 🟢 低风险修复队列（74 项 ⏳ 后续迭代）

- 占位符/Mock 存根（21 项）
- 项目规则符合性（11 项）
- 死代码（8 项）
- 其他（34 项）

#### 📋 合并到 v14 的历史遗留任务（⏳ 待修复）

- ⏳ v13 前端 P2：FE-P2-1/2/3 → 合并到中风险
- ⏳ v13 后端 P2：P2-1/2/3 → 合并到中风险
- ⏳ FE-P2-3：i18n 覆盖率（200+ 视图，后续迭代）
- ⏳ FE-P2-6：大列表虚拟化（966 处 el-table，后续迭代）
- ⏳ P2-8 剩余 143 个无测试 service（后续迭代）
- ⏳ E2E 失败排查（已知问题，待规则 5 节点）

---

## 🔄 进行中批次

### 批次 257：4 个 service 分页逻辑接入 paginate_with_total 第三批（PR #434，CI 进行中 🔄）

**状态**：代码已提交推送（分支 `fix/batch257-v14-pagination-part3`，commit 54f1947），PR #434 已创建，CI run #29062023389 进行中。

**待修复文件**（4 个 service）：
- `backend/src/services/customer_service.rs`：list 方法
- `backend/src/services/supplier_contact_service.rs`：list 方法
- `backend/src/services/user_service.rs`：list_users 方法
- `backend/src/services/notification_service.rs`：list 方法

**修复模式**（与批次 255/256 一致）：
- 删除独立 `select.clone().count()` 查询
- 复用 paginator 的 `num_items()`
- 统一补充 `page.clamp(1, 1000)` 防 DoS
- `PaginatorTrait` 导入保留

---

## 📋 下一个批次计划

### 批次 258（待启动）：4 个 service 分页逻辑接入 paginate_with_total 第四批

**候选文件**（从剩余 27/35 中选取 4 个）：
- `backend/src/services/quotation_service.rs`（需解决 ServiceError 转换）
- `backend/src/services/inventory_service.rs`
- `backend/src/services/sales_order_service.rs`
- `backend/src/services/purchase_order_service.rs`

---

## 规则节点提醒

- **规则 5（每 10 批次 E2E）**：批次 260 触发（上次批次 250）
- **规则 10（每 15 批次记忆整理）**：批次 270 触发（上次批次 255）

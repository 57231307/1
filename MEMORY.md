# 项目记忆文档

> 本文档记录项目关键信息、约束规则、当前状态与历史决策。
> 每次任务关键进展必须实时更新本文档。

---

## 一、项目基础

| 项目 | 内容 |
|------|------|
| 项目名称 | 冰西 ERP（Bingxi ERP） |
| 后端技术栈 | Rust 1.94.1 + Axum + SeaORM + PostgreSQL |
| 前端技术栈 | Vue 3.4 + TypeScript 5.4 + Element Plus + Vite |
| 主分支 | main |
| Git 平台 | GitHub |
| CI/CD | `.github/workflows/ci-cd.yml`（4 job 并行：build-backend / build-frontend / test / test-frontend） |

---

## 二、硬性约束（不可违反）

### 1. 构建验证规则（2026-06-15 用户新规）
- **项目全程只在 CI/CD 构建验证，本地不允许构建**
- 禁止本地运行 `cargo build` / `cargo clippy` / `cargo test` / `npm run build` / `vue-tsc` 等命令
- 所有验证必须通过 git push 触发 GitHub Actions CI 流水线
- 工作流配置：`.github/workflows/ci-cd.yml`
  - 后端：cargo clippy --all-targets -- -D warnings + cargo build --release
  - 前端：npm ci + npx vite build
  - 后端测试：cargo fmt --check + cargo test --lib --jobs 1
  - 前端测试：npm run test:run + npm run lint
- 验证流程：本地编码 → git commit → git push → 等待 CI 全绿 → 创建 PR → squash merge → 清理远端分支

### 2. Git 工作流
- 功能开发使用 feature/ 分支
- 修复 bug 使用 fix/ 分支
- PR 必须 squash merge 到 main
- 合并后立即删除远端 feature/ 分支
- 保持 main 分支为最新稳定源代码

### 3. 命名规范
- 名称不超过 9 个英文字符
- 使用有意义、描述性名称
- 避免缩写和单字母变量（除约定俗成的如 i）

### 4. 编码规范
- 项目禁止硬编码，所有文本使用中文
- 代码注释使用中文
- 文件名限 9 字符内、描述性

### 5. 安全规范
- 严禁 `auth.tenant_id.unwrap_or(0)`，必须用 `extract_tenant_id(&auth)?`
- 禁止硬编码敏感信息（密码、密钥、令牌等）
- 用户输入必须验证和清理
- 参数化查询防 SQL 注入

### 6. 死代码处理
- 禁止文件级 `#![allow(dead_code)]` 全局抑制
- 禁止 crate 级 `#![allow(unused_imports)]` 等
- 例外：SeaORM 自动生成模型（backend/src/models/）可保留文件级抑制
- 项级抑制：必须加 TODO(tech-debt) 注释

### 7. 思考模式规范（2026-06-16 用户新规）
- **每次开始思考都必须使用第一性原理**
- **禁止假设用户非常清楚自己想要什么和该怎么得到**
- **保持审慎**：从原始需求和问题出发，不要跳跃到方案
- **动机和目标不清晰时**：立即停下，使用 `AskUserQuestion` 工具与用户讨论
- **目标清晰但路径不是最短时**：明确告诉用户，并建议更好的方法
- **应用步骤**：
  1. 先识别"用户原始问题"（表面诉求）
  2. 推断"用户真实动机"（深层目的）
  3. 验证"目标是否清晰"（模糊则停下讨论）
  4. 评估"当前路径是否最短"（非最短则建议替代方案）
  5. 实施前再次确认（避免执行即返工）
- **禁止行为**：
  - 假设用户已经知道所有细节
  - 默认走最熟悉/最安全的路径
  - 跳过澄清问题直接实施
  - 把"我的理解"当作"用户的需求"

---

## 三、当前状态（2026-06-17）

### 已完成

| 任务 | 提交 | 状态 |
|------|------|------|
| B7：console.* 清理（112 处 → logger.*）| fee7507 | ✅ 完成 |
| Wave 3 v2 评估 | 3c9ca64 | ✅ 完成 |
| Wave 3 收尾综合 spec | 9ca478b | ✅ 完成 |
| B7 完成报告 | a8a1d1a | ✅ 完成 |
| CHANGELOG B7 更新 | 4658d37 | ✅ 完成 |
| **B 任务：清理 32 个 type-check 错误 → 0** | 7de8b0d | ✅ 完成（4 批 4 PR：#95-#98） |
| **A2-1 工艺优化**（recipe_opt）| f157f56 | ✅ **完成**（PR #99 squash merge，CI 4 job 全绿，11 文件实施，4 单测全过）|
| **A2-2 质量预测**（quality_pred）| dd9faa4 | ✅ **完成**（PR #100 squash merge，CI 4 job 全绿，8 文件实施，4 单测全过，自动发布 v2026.615.2350）|
| **P11 批 1：3 个高风险任务** | 0b1c9ac | ✅ **完成**（PR #173-#175 全部 squash merge，CI 4 job 全绿） |
| **P11 收尾 PR #176** | 382e522 | ✅ **完成**（H3 死字段修复 + 27 个新 dead_code 扩展标注 + CHANGELOG/MEMORY 同步，squash merge，CI 4 job 全绿）|

### P11 批 1 详情（2026-06-17）

- **H1 CSRF 防护**（PR #173，commit 475e79b）
  - 后端中间件 7 文件 + 13 单元/集成测试
  - 前端 axios 自动注入 `X-CSRF-Token` + localStorage 保存
  - 公开路径白名单 + Token rotation 模式
- **H2 Kafka 真实集成**（PR #174，commit 3e87b81）
  - 引入 rskafka 0.5（纯 Rust）
  - EventBus 重构为 `EventBackend` trait + Broadcast/Kafka 双后端
  - 默认 `enabled=false`，Kafka 不可达 5s 超时自动降级
- **H3 dead_code 全面清理**（PR #175，commit 0b1c9ac）
  - `#[allow(dead_code)]` 从 116 → 30（-74%）
  - 删除 1 整文件（scheduler_service.rs 336 行）+ 24 死函数 + 15+ 未用 import
  - 30 项保留全部补 `TODO(tech-debt)` 注释
  - 完成报告：`docs/superpowers/plans/2026-06-17-p11-h3-deadcode-cleanup-report.md`

### P11 收尾关键经验（用于 P12 改进）

- ✅ **串行派发策略** 用户已确认正确（避免子代理间文件冲突）
- ✅ **CI 反馈循环** 子代理 1 次提交 + 主代理 1 次修复 = 2 个 commit，符合项目工作流
- ⚠️ **子代理 import 误删** 教训：H3 子代理删除 15 处实际被用的 import，主代理通过 CI 错误精确定位并恢复
- ⚠️ **子代理 git 工作流** H3 子代理未完成 git commit/push 流程，主代理接手完成
- ⚠️ **本地 cargo fmt 必要之恶** 当子代理 fmt 不通过时，**主代理**可临时使用 `cargo fmt`（仅此例外），仍禁止 build/clippy/test
- ⚠️ **子代理死代码清理不彻底** H3 子代理在 pub 字段/方法/struct 上漏加 `#[allow(dead_code)]`，主代理通过 2 轮 CI 反馈补充 27+ 个标注（PR #176）

### 进行中

- 无（P11 批 1 收尾完成，P12 待启动）

### 待启动

- **P12 批 1**：详见综合路线图 [2026-06-17-roadmap.md](docs/superpowers/plans/2026-06-17-roadmap.md) **v0.3**
  - **v0.3 范围调整**（2026-06-17）：加入 P0 销售报价单 port（test 独有资产），**总 10 PR / 4 子代理并行**
  - 推荐范围：
    - P2-1 PR-2~5（4 PR 改写 4 .vue）
    - B-type-check（CI 5 job + vue-tsc 集成）
    - P2-2 性能优化（DB N+1 + Redis 缓存）
    - **P0 port 销售报价单**（4 PR 串行：数据层 → DTO+Service → Handler+路由 → 审批+转换+测试）
  - **v0.3 P0 port 关键约束**：test 与 main 无共同祖先，所有代码重新实现；stub pricing（不引入 product_color_price）
  - 详细 plan：[2026-06-17-p12-batch1-quotation-port-plan.md](docs/superpowers/plans/2026-06-17-p12-batch1-quotation-port-plan.md)
  - **v0.3 已移除**（已完成/基本完成项）：B5 POC / B6 清理 / B-PR 模板 / 复查 / 收尾 / 部署 / .monkeycode 归档 4 阶段
  - 派发策略：4 个独立子代理并行（参照 P11 批 1 验证通过的模式）
  - **v0.2 实际状态核实关键发现**：P2-1 PR-1（V2Table 组件 + useTableApi composable）已落地但被 v0.1 误标为"未启动"
  - **v0.3 关键发现**：test 分支与 main 完全分叉，1154 独有 commit / 29 迁移文件 / 7 handler；P0 销售报价单与 P0 产品色价为高价值资产
  - 用户决策：3 关键点已确认（命名/旧文件/范围）+ P0 port 范围确认 + v0.3 合并策略确认

### Wave 3 收尾关键产出

- 6 PR：#95-#100 全部 squash merge
- 4 CI run：4 job 全绿
- 8 新单测：recipe_opt 4 + quality_pred 4
- 1 自动发版：v2026.615.2350
- 6 远端分支清理
- 1 收尾报告：（已整合入 [2026-06-17-roadmap.md](docs/superpowers/plans/2026-06-17-roadmap.md)，旧版按用户决策删除）

### Wave 1-3 综合评估

- 评估报告：（已整合入 [2026-06-17-roadmap.md](docs/superpowers/plans/2026-06-17-roadmap.md)，旧版按用户决策删除）
- 评估 commit：1981888（PR #103 squash merge）
- 关键数据：894 commits / 101 PR 全部合并 / 190 tags / 235044 行代码 / 108 .vue / 493 .rs
- AI 模块：5 个（pred/detect/rec/recipe_opt/quality_pred）
- Wave 1：4 PR 100% 合并（4 子代理并行）
- Wave 2：原计划 9 子任务，实际 0 PR（主代理转向 Wave 3）
- Wave 3：11 PR 100% 合并（4+4 串行 + 2 单代理 + 1 收尾）
- Wave 4 启动推荐：P2-1 el-table-v2 真实数据验证（首选）/ P2-2 性能优化 / P3-1 安全加固

---

## 四、关键文档位置

| 文档 | 路径 |
|------|------|
| **综合路线图** | `/workspace/docs/superpowers/plans/2026-06-17-roadmap.md`（2026-06-17 新建，整合所有未完成任务）|
| Wave 4 P2-1 详细计划 | `/workspace/docs/superpowers/plans/2026-06-16-wave4-p2-1-plan.md` |
| Wave 4 P2-1 设计稿 | `/workspace/docs/superpowers/specs/2026-06-16-wave4-p2-1-design.md` |
| P11 H3 死代码清理报告 | `/workspace/docs/superpowers/plans/2026-06-17-p11-h3-deadcode-cleanup-report.md` |
| CHANGELOG | `/workspace/CHANGELOG.md` |
| CI/CD | `/workspace/.github/workflows/ci-cd.yml` |
| Clippy 规则 | `/workspace/backend/.clippy.toml` |

> **注**：2026-06-15 / 2026-06-13 / 2026-06-03 的旧 plans/specs 已按用户决策删除，关键内容整合入 [roadmap.md](docs/superpowers/plans/2026-06-17-roadmap.md)。

---

## 五、AI 智能分析服务框架

### 模块结构
```
backend/src/services/ai/
├── mod.rs         # 共享 DTO + Service 结构 + 工具函数
├── pred.rs        # 销售预测（移动平均 + 指数平滑 + 季节性因子）
├── detect.rs      # 异常检测（Z-score / IQR）
└── rec.rs         # 智能推荐（补货 / 关联 / 趋势 / 价格）
```

### 共享 DTO
- `SalesForecast`：产品 + 预测日期 + 预测量 + 置信度 + 趋势
- `InventorySuggestion`：产品 + 库存 + 建议 + 再订货点
- `AnomalyDetection`：实体类型/ID + 异常类型 + 严重度
- `SmartRecommendation`：推荐类型 + 目标 + 评分 + 原因
- `AbcClassification`：产品 + A/B/C + 销售额 + 累计比
- `InventoryTurnover`：产品 + 周转率 + 平均库存 + 出库

### Service 结构
```rust
pub struct AiAnalysisService {
    pub(crate) db: Arc<DatabaseConnection>,
}
```

### 共享工具函数
- `mean(&[f64]) -> f64`
- `std_deviation(&[f64]) -> f64`
- `iqr_quartiles(&[f64]) -> (f64, f64)`

### Handler 子模块
```
backend/src/handlers/advanced/
├── mod.rs          # pub use 所有子模块
├── analytics.rs    # 报表分析
├── decide.rs       # 异常检测 / 销售合同 / 销售价格 / 租户
├── forecast.rs     # 销售预测 / 库存优化
├── rec.rs          # 智能推荐
├── reorder.rs      # 采购合同 / 采购价格 / 销售退货
├── recipe_opt.rs   # 🆕 工艺优化（PR #99）
└── quality_pred.rs # 🆕 质量预测（PR #100）
```

### 路由
- `/advanced/ai/sales-forecast` POST
- `/advanced/ai/inventory-optimization` POST
- `/advanced/ai/anomaly-detection` POST
- `/advanced/ai/recommendations` POST
- `/advanced/reports/templates` GET
- `/advanced/reports/execute` POST
- `/advanced/reports/export` POST
- `/advanced/tenants` GET/POST
- `/advanced/tenants/:id` GET/PUT

### 前端 Advanced 页面
- 文件：`frontend/src/views/advanced/index.vue`
- API：`frontend/src/api/advanced.ts`
- 当前 Tab（5 个）：AI 分析 / 报表引擎 / 多租户管理 / 工艺优化 🆕 / 质量预测 🆕

---

## 六、数据模型关键表

| 表名 | 关键字段 | 用途 |
|------|----------|------|
| dye_recipe | 11 工艺参数（temperature / time_minutes / ph_value / liquor_ratio / fabric_type / color_no / color_name / dye_type / chemical_formula / auxiliaries / version）| A2-1 工艺优化数据源 |
| dye_batch | 外键关联 dye_recipe | 批次追溯 |
| quality_inspection_records | 13 创建 | A2-2 质量预测数据源 |
| production_orders | 004 创建 | 销售预测关联 |
| inventory_stock / inventory_transaction | - | 库存优化 |
| sales_order_item | - | 关联推荐 / 趋势推荐 |

---

## 七、关键经验（已踩坑）

### TypeScript
- 对象字面量 excess property check 每次只报告第一个未知属性
- `String(e)` 转换是 unknown → string 的标准模式
- 命名 `_rule` 触发 TS6133 命名豁免
- `vue-tsc` 不要带 `-b`（与 noEmit 冲突），用 `npx --no-install vue-tsc` 强制本地版本

### Rust
- 项级 `#[allow(dead_code)]` + TODO(tech-debt) 是合规做法
- SeaORM 自动生成模型保留文件级抑制
- 子代理串行调度避免云端卡死

### Git
- worktree 占用导致本地分支无法删除：先 `git checkout main` 切到 main，再 `git branch -D`
- GitHub squash merge 后远端分支自动删除

### CI/CD（2026-06-15 新增）
- 所有验证通过 `.github/workflows/ci-cd.yml`
- 后端 4 检查：clippy / build / fmt / test
- 前端 3 检查：build / test / lint
- 推送后等 CI 全绿（绿色 ✓）才算成功

---

## 八、对话语言

- 用户消息：中文（简体）
- 助手回复：中文（简体）
- 技术术语（Git 命令、文件名、错误码、英文错误消息）：保留英文原文

---

## 九、最后更新

- 2026-06-17 18:xx (Asia/Shanghai) - Roadmap v0.3：加入 P0 销售报价单 port 计划（test 独有资产 reverse-port），P12 批 1 总 PR 数从 6 升至 10，4 子代理并行派发
- 2026-06-17 17:xx (Asia/Shanghai) - Roadmap v0.2 状态更新：P2-1 PR-1 确认已完成（V2Table + useTableApi），B5/B6/B-PR 模板/部署/.monkeycode 等任务标注实际状态，P12 批 1 范围从 7 PR 调整为 6 PR
- 2026-06-16 01:55 (Asia/Shanghai) - 新增思考模式规范（第一性原理 + 不假设 + 路径求最短 + 目标不清停下讨论）

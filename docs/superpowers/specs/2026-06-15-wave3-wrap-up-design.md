# Wave 3 收尾综合 Spec (B + A2) - 2026-06-15

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 清理基线 32 个预存 type-check 错误（B） + 完成 AI 深化（A2），Wave 3 收尾
**Architecture:** 单子代理串行（避免云端卡死），B 先 A2 后，按文件细粒度分批
**Tech Stack:** Vue 3.4 / TypeScript 5.4 / Rust 1.94.1 / SeaORM / Axum / PostgreSQL

---

## 0. 总体执行策略

### 0.1 顺序：B 先 A2 后

**理由**：
- B 快速（5 批 ~2-3 天），清理后 A2 在干净基线上推进
- A2 涉及大量新代码，type-check 错误会影响子代理
- 严格验证（vue-tsc 0 错误 + cargo build + 集成测试）需要干净基线

### 0.2 派发模式

- **单子代理串行**（峰值 1）
- 每批完成 → 严格验证 → squash merge → 下一批
- 总周期：B 5 批 2-3 天 + A2 2 批 5-7 天 = 7-10 天

### 0.3 严格验证清单

每批合并前必须通过：
- [ ] `pnpm build`（vue-tsc -b + vite build）0 错误
- [ ] `cargo build` 0 错误
- [ ] `pnpm test` 现有单测全过
- [ ] `cargo test` 现有单测全过
- [ ] 错误数对比：基线 32 → 当前 ≤ 基线（无新增）

---

## 1. B 任务：清理 32 个预存 type-check 错误

### 1.1 错误分类与批次划分

| 批次 | 文件数 | 错误数 | 错误类型 | 修复策略 |
|------|--------|--------|----------|----------|
| **B-批 1** | 2 | 4 | TS2724 + TS2308 | cost API 引用修复 + index.ts 重导出 |
| **B-批 2** | 10 | 11 | TS2339 (ApiResponse.total) | 全局 ApiResponse 类型扩展 |
| **B-批 3** | 1 | 3 | TS2322 | dataPermission 类型守卫 |
| **B-批 4** | 1 | 8 | TS2353 + TS2339 | fiveDimension 类型定义扩展 |
| **B-批 5** | 3 | 3 | TS6133 + TS2345 | warehouse + user-profile + 杂项 |
| **B-批 6** | 1 | 3 | 综合复查 | 整体 vue-tsc 0 错误验证 |

### 1.2 详细错误分布

#### 批 1（2 文件 4 错误）

**src/api/index.ts(49,1)**: TS2308 模块 './financial-analysis' 已导出 'ReportData'
- 修复：移除或显式 re-export

**src/views/cost/tabs/CostCollectionTab.vue(200-202)**: TS2724 缺 listCollections/createCollection/updateCollection
- 修复：将代码改为 `listCostCollections` / `createCostCollection` / `updateCostCollection`（B6 已重命名）

#### 批 2（10 文件 11 错误）

ApiResponse 泛型未声明 `.total` 字段：
- api-gateway/index.vue: 494, 650, 763
- data-import/index.vue: 342, 379
- dye-batch/index.vue: 290
- dye-recipe/index.vue: 279
- print-templates/index.vue: 274
- quality-standards/index.vue: 214
- report-templates/index.vue: 218
- system-update/index.vue: 427, 507, 572

**修复策略**（两选一）：

**方案 A（推荐）**：在 ApiResponse 类型上添加 `total?: number` 可选字段
```typescript
// src/types/api.ts 或类似位置
export interface ApiResponse<T> {
  data: T
  total?: number  // 分页总数，可选
  code: number
  message: string
}
```

**方案 B**：在每个使用处加 `(response as any).total`
- 不推荐：污染类型

#### 批 3（1 文件 3 错误）

**src/views/dataPermission/index.vue(262-264)**: TS2322 string | undefined 不匹配 CustomCondition/AllowedFields/HiddenFields
- 修复：添加类型守卫或 null check
```typescript
// 类型守卫
if (typeof value === 'string' && value) { ... }
```

#### 批 4（1 文件 8 错误）

**src/views/fiveDimension/index.vue**:
- 75: `page` 不在 StatsQueryParams
- 118: `five_dimension_id` 不在 FiveDimensionStats
- 153: `keyword` 不在 SearchQueryParams
- 305-315: `product_id/product_name/batch_no/color_no/dye_lot_no/grade` 6 个字段缺失

**修复策略**：
- 扩展 StatsQueryParams 添加 `page?: number`
- 扩展 SearchQueryParams 添加 `keyword?: string`
- 扩展 FiveDimensionStats 添加 `five_dimension_id?: number`
- 在组件内对 stats 对象做类型断言或扩展内部类型

#### 批 5（3 文件 3 错误）

- **src/views/user-profile/index.vue(170)**: TS6133 'rule' 已声明未使用 → 删除
- **src/views/warehouse/index.vue(368, 389)**: TS2345 unknown 不能赋给 string → `String(value)` 包装

### 1.3 验收标准

- [ ] `vue-tsc` 错误数：32 → **0**
- [ ] `cargo build` 无新增警告
- [ ] 现有单测全过
- [ ] 5 批 PR 全部 squash merge
- [ ] 远端分支全部清理
- [ ] 业务逻辑未改动（仅类型修复）

---

## 2. A2 任务：AI 深化（工艺优化 + 质量预测）

### 2.1 范围

| 任务 | 后端 service | 后端 handler | 前端 API | 前端 Tab | 端点 |
|------|--------------|--------------|----------|----------|------|
| **A2-1** 工艺优化 | `recipe_opt.rs` (250-300 行) | `recipe_opt.rs` (120-150 行) | `optimizeRecipe` | "工艺优化" | `POST /advanced/ai/recipe-optimization` |
| **A2-2** 质量预测 | `quality_pred.rs` (250-300 行) | `quality_pred.rs` (120-150 行) | `predictQuality` | "质量预测" | `POST /advanced/ai/quality-prediction` |

### 2.2 数据源（已确认就绪）

- **dye_recipe** (027 创建 / 026 修复列)：11 工艺参数（recipe_no, color_no, formula, temperature, time_minutes, ph_value, liquor_ratio, color_name, fabric_type, dye_type, chemical_formula）
- **dye_batch** (028 创建)：dye_recipe_id 外键关联
- **quality_inspection_records** (013 外键)：product_id, inspection_date, result

### 2.3 算法选择

**工艺优化（recipe_opt.rs）**：
- **主方案**：k-NN 匹配历史配方（基于 color_no + fabric_type 找相似案例，按 temperature/time_minutes/ph_value 加权推荐）
- **退化方案**：基于颜色 + 布料类型的典型参数表（硬编码兜底）
- **冷启动**：历史数据 < 10 条时返回"需更多数据"提示

**质量预测（quality_pred.rs）**：
- **主方案**：基于"工艺指纹"相似度（k-NN 找历史相似 case）→ 返回历史平均合格率
- **退化方案**：参数合法性检查（温度 30-100°C, pH 4-9, 时间 10-120 min）+ 越界警告
- **冷启动**：同工艺优化

### 2.4 验收标准

#### A2-1 工艺优化

- [ ] `POST /advanced/ai/recipe-optimization` 接收 `{ color_no, fabric_type, color_name? }`
- [ ] 返回 `{ recommended_params: { temperature, time_minutes, ph_value, liquor_ratio }, similar_cases: number, confidence: number }`
- [ ] 后端 service 4 个单测全过
- [ ] 前端 Tab 渲染：表单 + 推荐参数 + 相似案例数
- [ ] type-check / build / 集成测试全过

#### A2-2 质量预测

- [ ] `POST /advanced/ai/quality-prediction` 接收 `{ recipe_no?, temperature, time_minutes, ph_value, liquor_ratio }`
- [ ] 返回 `{ predicted_pass_rate: number, similar_cases: number, warnings: string[] }`
- [ ] 后端 service 4 个单测全过
- [ ] 前端 Tab 渲染：表单 + 预测结果 + 警告列表
- [ ] type-check / build / 集成测试全过

### 2.5 风险与应对

| 风险 | 等级 | 应对 |
|------|------|------|
| 历史数据稀疏 | 🟡 中 | 退化方案：典型参数表 + 合法性检查 |
| 工艺参数非线性 | 🟡 中 | 简化模型：k-NN 相似度匹配 |
| 质量预测准确度 | 🟠 高 | 明确指标：返回"历史相似案例合格率"而非硬预测 |
| 业务领域知识 | 🟠 高 | 算法基于通用统计，业务调优留给后续迭代 |

---

## 3. 子代理任务描述

### 3.1 B 任务子代理输入规范

每批派发时提供：
1. 基线 commit + 错误清单（文件:行:错误码）
2. 修复策略（按 1.2 表）
3. 验证命令（vue-tsc / cargo build / pnpm test）
4. 输出要求：before/after 错误数 + 改动文件 + commit SHA

### 3.2 A2 任务子代理输入规范

派发 A2-1 时提供：
1. 完整 spec（含 DTO 定义、算法伪代码、路由注册位置、前端集成位置）
2. 现有 AI 框架参考文件（mod.rs / pred.rs / rec.rs / forecast.rs / decider.rs）
3. 数据源 SQL 表结构（dye_recipe / dye_batch）
4. 单测要求 + 验证命令
5. 退化方案硬编码表（颜色 → 典型温度/时间/pH）

派发 A2-2 时类似。

### 3.3 派发顺序

```
B-批 1 (cost + index.ts)
   ↓ squash merge
B-批 2 (ApiResponse.total 全局)
   ↓ squash merge
B-批 3 (dataPermission)
   ↓ squash merge
B-批 4 (fiveDimension)
   ↓ squash merge
B-批 5 (warehouse + user-profile)
   ↓ squash merge
B-批 6 (复查验证)
   ↓ squash merge
A2-1 (工艺优化)
   ↓ squash merge
A2-2 (质量预测)
   ↓ squash merge
Wave 3 收尾报告
```

---

## 4. 验收总标准

### 4.1 B 任务 DoD

- [ ] vue-tsc 错误数：32 → **0**
- [ ] cargo build 无新增警告
- [ ] 现有单测全过
- [ ] 5-6 个 PR 全部 squash merge 到 main
- [ ] 远端分支全部清理

### 4.2 A2 任务 DoD

- [ ] 2 个后端 service 编译通过
- [ ] 2 个后端 handler 路由注册 + 集成测试通过
- [ ] 2 个前端 API 函数 type-check 0 错误
- [ ] 2 个前端 Tab 渲染正常
- [ ] 8 个单测全过
- [ ] 2 个 PR 全部 squash merge

### 4.3 Wave 3 收尾 DoD

- [ ] B + A2 全部完成
- [ ] CHANGELOG 更新
- [ ] MEMORY 更新
- [ ] 收尾报告写入 docs/superpowers/plans/

---

## 5. 立即可执行

派发 B-批 1 子代理（cost + index.ts 修复，4 个错误）：
- 范围：src/api/index.ts + src/views/cost/tabs/CostCollectionTab.vue
- 修复：移除 ReportData 重导出冲突 + 改为 B6 重命名后的 API 名
- 验证：vue-tsc 错误 32 → 28

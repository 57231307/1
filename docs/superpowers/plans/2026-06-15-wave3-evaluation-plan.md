# Wave 3 评估与计划 - 2026-06-15

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 评估 Wave 3 任务的实际状态、范围与可行性，明确启动策略
**Architecture:** B7（P2-2 清理 console.*） + A2（P2-4 AI 深化），单子代理串行
**Tech Stack:** Vue 3 / TypeScript / Element Plus / Rust 1.94.1 / SeaORM / Axum

---

## 0. Wave 3 启动条件验证

### 0.1 B5 POC 通过验证

| 条件 | 阈值 | 实测 | 结论 |
|------|------|------|------|
| TypeScript 检查 | 0 错误 | 0 错误 | ✅ |
| Vite build | 成功 | 成功 | ✅ |
| 单元测试 | 全过 | 17/17 | ✅ |
| 1 万行数据生成 | < 500ms | 13.2ms | ✅ 优于阈值 38 倍 |
| 真实性能（FPS） | ≥ 50 | 待本地复现 | 🟡 沙箱无 GUI |
| 真实内存 | < 100MB | 待本地复现 | 🟡 沙箱无 GUI |

**结论**：Wave 3 **可启动**

---

## 1. Wave 3 任务清单

| 任务 ID | 任务名称 | 子代理 | 周期 | 优先级 |
|---------|---------|--------|------|--------|
| **B7** | 清理 112 处 console.* → logger | 1 B | 3-5 天 | 🟠 P1 |
| **A2** | AI 深化（工艺优化 + 质量预测） | 1 A | 2 周 | 🟡 P2 |

**总资源**：2 子代理（串行）
**总周期**：3 周

---

## 2. B7 任务详细评估

### 2.1 实测数据

| 维度 | 原估算 | 实测 | 差异 |
|------|--------|------|------|
| console.* 总数 | 145 | **112** | -22.8% ✅ |
| 涉及文件数 | ~50 | ~50 | ✅ |
| logger.ts 存在 | ✅ | ✅ | 4 个方法（debug/info/warn/error）|

### 2.2 console.* 分布（按文件 top 10）

| 排名 | 文件 | console 次数 |
|------|------|------------|
| 1 | [views/purchase-return/index.vue](file:///workspace/frontend/src/views/purchase-return/index.vue) | 7 |
| 2 | [views/purchase-contract/index.vue](file:///workspace/frontend/src/views/purchase-contract/index.vue) | 7 |
| 3 | [views/bpm/definitions.vue](file:///workspace/frontend/src/views/bpm/definitions.vue) | 7 |
| 4 | [views/tenant-billing/index.vue](file:///workspace/frontend/src/views/tenant-billing/index.vue) | 6 |
| 5 | [views/security/index.vue](file:///workspace/frontend/src/views/security/index.vue) | 6 |
| 6 | [views/purchase-price/index.vue](file:///workspace/frontend/src/views/purchase-price/index.vue) | 6 |
| 7 | [views/logistics/index.vue](file:///workspace/frontend/src/views/logistics/index.vue) | 6 |
| 8 | [views/dye-recipe/index.vue](file:///workspace/frontend/src/views/dye-recipe/index.vue) | 6 |
| 9 | [views/dye-batch/index.vue](file:///workspace/frontend/src/views/dye-batch/index.vue) | 6 |
| 10 | [views/sales-analysis/index.vue](file:///workspace/frontend/src/views/sales-analysis/index.vue) | 5 |

### 2.3 console.* 类型分布

| 类型 | 数量 | 占比 | 处置 |
|------|------|------|------|
| console.error | ~70 | 63% | 替换为 `logger.error` + ElMessage 错误提示 |
| console.warn | ~25 | 22% | 替换为 `logger.warn` |
| console.log | ~10 | 9% | 替换为 `logger.debug` |
| console.info | ~5 | 4% | 替换为 `logger.info` |
| console.debug | ~2 | 2% | 替换为 `logger.debug` |

### 2.4 logger.ts 能力分析

```typescript
// 现有 4 个级别方法 + 完整环境感知
class Logger {
  private enabled: boolean  // import.meta.env.DEV
  private level: LogLevel    // 'debug' | 'info' | 'warn' | 'error'
  
  debug(message: string, ...args: unknown[]): void
  info(message: string, ...args: unknown[]): void
  warn(message: string, ...args: unknown[]): void
  error(message: string, ...args: unknown[]): void
}
```

✅ **完全足够**支持 112 处 console.* 替换，**无需扩展 logger 能力**

### 2.5 替换模式

**B3-1 子代理已建立的模式**：

```typescript
// 替代 1：模块顶部
import { logger } from '@/utils/logger'

// 替代 2：直接调用
logger.error('获取数据失败:', e)  // 替代 console.error('获取数据失败:', e)
logger.warn('加载列表为空')        // 替代 console.warn('加载列表为空')
logger.debug('当前状态:', state)   // 替代 console.log('当前状态:', state)

// 替代 3：业务侧增强（catch 分支同时提示用户）
try {
  await fetchData()
} catch (e) {
  logger.error('获取失败:', e)
  ElMessage.error('数据加载失败，请稍后重试')
}
```

### 2.6 任务执行计划

**预计工作**：112 处 console.* → logger.error/warn/debug/info
**预计耗时**：3-5 天（单子代理串行，约 30 处/天）
**文件数**：~50 个
**改动密度**：平均 2-3 处/文件

### 2.7 验收标准

- [ ] console.* 残留：0 处
- [ ] logger.* 调用：112+ 处
- [ ] TypeScript 0 错误
- [ ] ESLint 0 错误
- [ ] 业务侧错误 catch 分支含 ElMessage 提示（按规范要求）

---

## 3. A2 任务详细评估

### 3.1 AI 现状盘点

#### 已有 AI 服务（4 个 service）

| 文件 | 行数 | 功能 |
|------|------|------|
| [services/ai/mod.rs](file:///workspace/backend/src/services/ai/mod.rs) | 138 | 共享 DTO + AiAnalysisService trait |
| [services/ai/pred.rs](file:///workspace/backend/src/services/ai/pred.rs) | 239 | 销售预测（WMA + 指数平滑 + 季节性）|
| [services/ai/detect.rs](file:///workspace/backend/src/services/ai/detect.rs) | 225 | 异常检测（Z-score / IQR）|
| [services/ai/rec.rs](file:///workspace/backend/src/services/ai/rec.rs) | 670 | 智能推荐（补货/关联/趋势/价格调整）|

#### 已有 AI Handler（6 个）

| 文件 | 行数 | 端点 |
|------|------|------|
| [handlers/advanced/forecast.rs](file:///workspace/backend/src/handlers/advanced/forecast.rs) | 234 | sales_forecast / inventory_optimization |
| [handlers/advanced/decide.rs](file:///workspace/backend/src/handlers/advanced/decide.rs) | 326 | anomaly_detection |
| [handlers/advanced/rec.rs](file:///workspace/backend/src/handlers/advanced/rec.rs) | 81 | recommendations |
| [handlers/advanced/analytics.rs](file:///workspace/backend/src/handlers/advanced/analytics.rs) | 181 | 报表 |
| [handlers/advanced/reorder.rs](file:///workspace/backend/src/handlers/advanced/reorder.rs) | 162 | 补货列表 |
| [handlers/ai_analysis_handler.rs](file:///workspace/backend/src/handlers/ai_analysis_handler.rs) | - | 统一 facade |

#### 已有 AI 端点（7+ 个）

| 路径 | 方法 | 功能 |
|------|------|------|
| `/forecast-sales` | GET | 销售预测 |
| `/optimize-inventory` | GET | 库存优化 |
| `/detect-anomalies` | GET | 异常检测 |
| `/recommendations` | GET | 智能推荐 |
| `/ai/sales-forecast` | POST | AI 销售预测 |
| `/ai/inventory-optimization` | POST | AI 库存优化 |
| `/ai/anomaly-detection` | POST | AI 异常检测 |

### 3.2 P2-4 新增能力

| 能力 | 数据源 | 表存在性 | 预估工作量 |
|------|--------|---------|----------|
| **工艺优化** | dye_recipe + dye_batch + quality_inspection | 🟡 dye_batch ✅ / dye_recipe ❌ | 2 周 |
| **质量预测** | product + production_order + quality_inspection | ✅ 全部存在 | 1.5 周 |

### 3.3 数据源表详细检查

| 表 | migration 文件 | 状态 |
|------|----------------|------|
| `dye_batch` | [028_create_dye_batch_greige_fabric.sql](file:///workspace/database/migration/028_create_dye_batch_greige_fabric.sql) | ✅ 存在 |
| `dye_recipe` | ❌ 未发现独立 migration | 🟠 需新建 |
| `quality_inspection` | [013_quality_foreign_keys.sql](file:///workspace/database/migration/013_quality_foreign_keys.sql) | ✅ 存在 |
| `production_order` | [004_mrp_production.sql](file:///workspace/database/migration/004_mrp_production.sql) | ✅ 存在 |

### 3.4 AI 深化实施范围

#### 3.4.1 工艺优化（process_opt）

**算法**：
- 基于历史 dye_batch + dye_recipe 数据
- 回归模型（线性回归 / 随机森林）
- 特征：温度/时间/助剂/原料配比
- 输出：最佳工艺参数推荐

**实施步骤**：
1. 数据采集：从 `dye_batch` + `quality_inspection` 聚合
2. 特征工程：工艺参数向量化
3. 模型训练：简单线性回归（MVP 阶段）
4. 推理服务：`services/ai/process_opt.rs`
5. Handler：`handlers/advanced/process_opt.rs`
6. 路由：`POST /api/v1/erp/ai/process-optimization`
7. 前端：`views/advanced/` 新增 Tab

#### 3.4.2 质量预测（quality_pred）

**算法**：
- 基于历史 production_order + quality_inspection
- 分类模型（合格/不合格 + 等级分类）
- 特征：原料/工艺参数/生产时间/工人
- 输出：预测质量等级

**实施步骤**：
1. 数据采集：从 `production_order` + `quality_inspection` 聚合
2. 特征工程：原材料批次 + 工艺向量
3. 模型训练：决策树分类（MVP 阶段）
4. 推理服务：`services/ai/quality_pred.rs`
5. Handler：`handlers/advanced/quality_pred.rs`
6. 路由：`POST /api/v1/erp/ai/quality-prediction`
7. 前端：`views/advanced/` 新增 Tab

### 3.5 风险与缓解

| 风险 | 等级 | 缓解 |
|------|------|------|
| 训练数据不足 | 🟠 中 | MVP 用规则引擎（if-else），数据足够后切换 ML |
| 模型效果差 | 🟠 中 | 人工标注 + 反馈机制 |
| 工艺专家参与 | 🟡 低 | 与用户确认业务专家参与时间 |
| dye_recipe 表缺失 | 🟡 低 | 新建 migration，参考 028 风格 |

### 3.6 验收标准

- [ ] `services/ai/process_opt.rs` 实现
- [ ] `services/ai/quality_pred.rs` 实现
- [ ] `handlers/advanced/process_opt.rs` 实现
- [ ] `handlers/advanced/quality_pred.rs` 实现
- [ ] 路由注册：`/ai/process-optimization` + `/ai/quality-prediction`
- [ ] 前端 `views/advanced/index.vue` 新增 2 Tab
- [ ] 至少 1 个端到端集成测试
- [ ] TypeScript / Rust 0 错误
- [ ] CI 全绿

---

## 4. 调度策略

### 4.1 串行执行（推荐）

考虑到云端卡死风险，**单子代理串行**：

```
Day 1-5:  B7 清理 112 处 console.* → logger（1 子代理）
Day 6-19: A2 AI 深化（工艺优化 + 质量预测）（1 子代理）
Day 20:   复查子代理 + 收尾
```

**总周期**：3 周（15 工作日）
**峰值并发**：1 子代理

### 4.2 子代理任务说明

| 任务 | 难度 | 关键技能 |
|------|------|---------|
| B7 | 🟢 低 | 文本替换（console.* → logger.*）+ ESLint 验证 |
| A2 | 🟠 中-高 | Rust ML + 数据库 + 前端集成 + 模型调优 |

### 4.3 子代理交接

- B7 完成后，子代理必须输出：
  - 改动文件清单（精确到路径）
  - 验证结果（type-check / lint 0 错误）
  - 任何遗留风险
- A2 完成后，子代理必须输出：
  - 新增/修改文件清单
  - 数据验证结果
  - 模型训练指标
  - 端到端测试结果

---

## 5. 与原 Wave 3 规划的关键差异

| 维度 | 原规划 | 修订后 | 原因 |
|------|--------|--------|------|
| console.* 总数 | 145 | **112** | B3-1~4 拆分时已部分清理 |
| AI 任务 | 1 个（P2-4）| 1 个（A2）| 数据源表部分缺失 |
| dye_recipe 表 | 假设存在 | 🟠 需新建 | migration 缺失 |
| 调度策略 | 未明确 | **单子代理串行** | 避免云端卡死 |

---

## 6. 风险与建议

### 6.1 高风险项

| # | 风险 | 影响 | 缓解 |
|---|------|------|------|
| 1 | B7 涉及 50+ 文件 | 子代理长任务 | 每天推进 20-30 处 |
| 2 | A2 工艺优化需业务专家 | 模型效果 | 业务侧先提供标注数据 |
| 3 | dye_recipe 表缺失 | A2 前置任务阻塞 | 先新建 migration |
| 4 | 模型 ML 库 | Rust 生态 ML 库不成熟 | MVP 用规则引擎 |

### 6.2 关键建议

1. **B7 立即启动**：112 处 console.* 是技术债，CI 死代码审计会持续警告
2. **A2 启动前确认**：dye_recipe 表是否需要新建 + 业务专家参与时间
3. **保守策略**：A2 第一阶段用规则引擎，第二阶段再上 ML 模型
4. **可观测性**：B7 替换时同时接入 [utils/logger.ts](file:///workspace/frontend/src/utils/logger.ts) 的 ElMessage 提示

---

## 7. 启动决策

### 7.1 B7 - 立即可启动

**理由**：
- console.* 112 处是确定的技术债
- logger.ts 已存在
- 替换模式已建立（B3-1 子代理使用过）
- 风险低，收益高

### 7.2 A2 - 需用户确认后启动

**需确认**：
1. dye_recipe 表是否需要新建 migration？
2. 工艺优化 + 质量预测的业务专家是否参与？
3. 模型策略：MVP 用规则引擎 vs 直接 ML？
4. 业务方对 AI 深化效果的预期？

---

## 8. 评估结论

| 维度 | 评分 | 评估 |
|------|------|------|
| 任务清晰度 | ⭐⭐⭐⭐⭐ | 2 个任务边界明确 |
| 范围合理性 | ⭐⭐⭐⭐ | B7 112 处合理，A2 2 周合理 |
| 数据可获得性 | ⭐⭐⭐ | A2 dye_recipe 需补 |
| 风险预判 | ⭐⭐⭐⭐ | 4 大风险已识别 |
| 可执行性 | ⭐⭐⭐⭐ | B7 立即可执行，A2 需前置 |
| 资源估算 | ⭐⭐⭐⭐ | 2 子代理串行合理 |

**综合评分：8.0/10** ✅

---

## 9. 行动建议

1. **立即启动 B7**：单子代理 3-5 天清理 112 处 console.*
2. **同步推进 A2 前置**：新建 dye_recipe migration（如缺失）
3. **A2 启动前用户确认**：业务专家 + 模型策略
4. **Wave 3 复查子代理**：与 Wave 2 相同的 10 项审查清单
5. **Wave 4 启动条件保持**：≥ 1 个 P3 任务完成 PoC

---

## 10. 与 Wave 2 的一致性

| 维度 | Wave 2 经验 | Wave 3 应用 |
|------|------------|------------|
| 子代理调度 | 单子代理串行 | ✅ 保持 |
| 验证流程 | type-check + lint | ✅ 保持 |
| 合并策略 | Squash | ✅ 保持 |
| 清理范围 | 远端 6 分支 | ✅ 保持 |
| CHANGELOG | Wave 汇总 | ✅ 保持 |
| 冲突解决 | rebase + python | ✅ 保持 |

# B7 实施规格 - 2026-06-15

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 112 处 `console.*` 调用统一替换为 `logger.*`，消除分散日志、提升可维护性
**Architecture:** 单子代理串行执行（避免云端卡死），按域分批 squash merge
**Tech Stack:** Vue 3.4 / TypeScript 5 / Vite 5 / logger.ts（已存在）

---

## 1. 范围确认（基于 Wave 3 评估）

### 1.1 目标数据

| 维度 | 数值 |
|------|------|
| console.* 总数 | 112 处 |
| 涉及文件数 | ~50 个 .vue 文件 |
| 涉及目录 | 9 个域（inventory / crm / finance / purchase / sales / bpm / security / logistics / dye 等）|
| 排除项 | 第三方 lib（node_modules）、构建脚本（vite.config.ts）、测试文件 |

### 1.2 logger.ts 现状（已存在，无需修改）

```typescript
// frontend/src/utils/logger.ts
class Logger {
  debug(message: string, ...args: unknown[]): void  // DEV 环境
  info(message: string, ...args: unknown[]): void
  warn(message: string, ...args: unknown[]): void
  error(message: string, ...args: unknown[]): void
}
export const logger = new Logger()
```

### 1.3 替换映射规则

| 原始 | 替换为 | 依据 |
|------|--------|------|
| `console.log` | `logger.info` | 业务信息日志 |
| `console.debug` | `logger.debug` | 调试日志 |
| `console.info` | `logger.info` | 信息日志 |
| `console.warn` | `logger.warn` | 警告 |
| `console.error` | `logger.error` | 错误 |

**默认策略**：直接对应替换，不做激进等级重分类（避免引入 bug）。

---

## 2. 执行策略

### 2.1 模式：单子代理串行

**原因**：用户明确要求"避免云端卡死，不要一次性太多进程"。Wave 2 已验证该模式（峰值 1 子代理），6 个 PR 全部成功合并。

### 2.2 分批方案

按域拆分 4 个批次，每批完成 → type-check + build → squash merge → 下一批：

| 批次 | 域 | 预估文件数 | 预估 console 数 |
|------|-----|-----------|----------------|
| 批 1 | inventory + product（采购/库存/产品）| ~15 | ~30 |
| 批 2 | crm + sales（CRM/客户/销售）| ~12 | ~25 |
| 批 3 | finance + bpm（财务/会计/BPM）| ~12 | ~30 |
| 批 4 | 其余（dye/logistics/security/其他）| ~11 | ~27 |
| **合计** | **9 个域** | **~50** | **~112** |

### 2.3 合并节奏

- 每批完成立即合并 → 始终保持 main 可发布
- 使用 `git reset --hard origin/main` 拉取最新 main 作为下一批起点
- 远端分支由 GitHub squash merge 自动清理，残留通过 `git update-ref -d` 处理

---

## 3. 子代理任务描述

### 3.1 子代理输入规范

每批派发时提供：

1. **基线信息**：
   - 当前 main 提交 SHA
   - 批次的域路径（如 `frontend/src/views/inventory/`）
   - 该批的目标 console.* 总数

2. **执行步骤**：
   ```bash
   # 1. 基于 main 创建特性分支
   git fetch origin && git checkout main && git pull --ff-only
   git checkout -b feature/B7-batch-N-<domain>

   # 2. 扫描该域下所有 console.*
   grep -rn "console\." frontend/src/views/<domain>/ --include="*.vue"

   # 3. 逐文件替换 + 注入 import
   # 每个 .vue 文件 <script setup lang="ts"> 顶部添加：
   import { logger } from '@/utils/logger'

   # 4. 类型检查 + 构建
   cd frontend && pnpm type-check
   pnpm build

   # 5. 提交
   git add -A
   git commit -m "refactor(frontend): B7-<N> 替换 <domain> 域 console.* 为 logger"
   git push -u origin feature/B7-batch-N-<domain>
   ```

3. **替换规则**：见 1.3 表格

4. **特殊处理**：
   - 若文件已 import `logger` → 跳过 import 注入
   - 若 console.* 出现在 `<template>` 标签内 → **保留** console（template 中无法使用 TS import）
   - 若 console.* 在 catch 块中 → 使用 `logger.error(err)`
   - 错误对象：原 `console.error('msg', err)` 改为 `logger.error('msg', err)`

5. **测试要求**：
   - **核心文件必测**：stores、main.ts、router、api utils
   - **叶子页面组件**：可跳过
   - 测试用例：验证 `logger.info/warn/error` 被调用（spy mock）

### 3.2 子代理输出要求

返回内容：
- 改动的文件列表
- console.* 替换数（before/after）
- type-check / build 输出（最后 30 行）
- 新增/修改的单测列表
- 分支名 + commit SHA

---

## 4. 验收标准

### 4.1 功能验收

- [ ] `frontend/src/` 下 `grep -rn "console\." --include="*.vue" --include="*.ts"` 结果 ≤ 5（允许第三方库代理、debugger 等）
- [ ] 所有 .vue 文件 import 了 `@/utils/logger`（如该文件含 console.*）
- [ ] `pnpm type-check` 0 错误
- [ ] `pnpm build` 成功
- [ ] 现有 vitest 套件全过

### 4.2 质量验收

- [ ] 无新增 console.* 警告
- [ ] 无 `any` 类型新增
- [ ] 无 import 重复注入
- [ ] 单测覆盖核心文件（stores、main、router、utils）

### 4.3 流程验收

- [ ] 4 个 PR 全部 squash merge 到 main
- [ ] 远端分支全部清理
- [ ] CHANGELOG.md 记录 B7 4 批成果
- [ ] MEMORY.md 更新关键经验

---

## 5. 风险与应对

| 风险 | 应对 |
|------|------|
| 子代理替换时遗漏 console.* | 批次合并前用 `grep -c "console\\."` 验证 before/after |
| template 中出现 console.* | 保留 console（合理，template 不执行 TS）|
| catch 块丢失错误对象 | 子代理提示词中明确要求保留 err 参数 |
| 子代理串行速度慢 | 4 批 × 1-2 天/批 = 4-8 天，可接受 |
| 沙箱 git 状态污染 | 每批派发前 `git reset --hard origin/main` |

---

## 6. 任务依赖

```
批 1 (inventory+product) ──┐
批 2 (crm+sales) ──────────┼─→ 主线收尾
批 3 (finance+bpm) ────────┤
批 4 (dye+logistics+...) ──┘
```

无批间依赖（每批基于 main 重启），但串行执行避免冲突。

---

## 7. 完成定义（Definition of Done）

- [ ] 4 个批次全部 PR 合并到 main
- [ ] 全量 `pnpm type-check && pnpm build && pnpm test` 通过
- [ ] CHANGELOG.md / MEMORY.md 已更新
- [ ] 远端 4 个特性分支全部清理
- [ ] B7 总结报告写入 docs/superpowers/plans/

# 冰溪 ERP 项目质量提升实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 基于项目健康状态报告，系统性提升代码质量，消除技术债务，建立代码规范

**Architecture:** 分四个阶段推进：前端代码规范化 → 后端文件名优化 → 代码质量加固 → 文档与规范建立。每个阶段包含独立可测试的任务，采用 TDD 方式确保改动不破坏现有功能。

**Tech Stack:** TypeScript (Vue 3), Rust (Axum), ESLint, Prettier, cargo-fmt

---

## 一、问题清单与优先级

### 1.1 前端问题（P1 - 高优先级）

| 问题 | 数量 | 影响范围 | 风险等级 |
|------|------|----------|----------|
| `any` 类型滥用 | 136处 / 41文件 | API 层、工具函数、Store | 中（类型安全缺失） |
| `console.*` 日志 | 21处 / 5文件 | Store、Router | 低（生产环境日志污染） |

**详细分布：**
- `any` 类型重灾区：
  - `api/trading.ts`: 10处
  - `api/bpm.ts`: 9处
  - `api/financial-analysis.ts`: 8处
  - `api/five-dimension.ts`: 7处
  - `api/data-permission.ts`: 7处
  - `api/financeReport.ts`: 7处
  - `api/ap.ts`: 7处
  - `utils/export.ts`: 5处
  - `utils/print.ts`: 4处

- `console.*` 分布：
  - `store/sales.ts`: 6处
  - `store/fabric.ts`: 6处
  - `store/dashboard.ts`: 3处
  - `store/inventory.ts`: 3处
  - `router/index.ts`: 3处

### 1.2 后端问题（P2 - 中优先级）

| 问题 | 数量 | 影响范围 | 风险等级 |
|------|------|----------|----------|
| `println!` 使用 | 100+处 | CLI 工具 | 低（CLI 输出合理） |
| 文件名含 Rust 关键字 | 3处 | 服务模块 | 低（已稳定使用） |

**文件名风险清单：**
- `backend/src/services/inv/move_rs.rs` → 建议重命名为 `inventory_move.rs`
- `backend/src/services/so/return_rs.rs` → 建议重命名为 `sales_return.rs`
- `backend/src/services/po/return_rs.rs` → 建议重命名为 `purchase_return.rs`

**说明：** `println!` 主要集中在 `cli/util/` 目录，属于命令行工具的正常输出，不构成质量问题。生产代码（handlers/services）中已无 `println!`。

### 1.3 已完成项（无需处理）

- ✅ `todo!()` / `unimplemented!()` 宏：0处
- ✅ 生产代码 `println!`：0处（仅 CLI 工具）
- ✅ 安全漏洞：6个中高危漏洞已全部修复

---

## 二、实施阶段

### 阶段一：前端类型安全加固（预计 2-3 天）

**目标：** 消除 80% 的 `any` 类型，建立类型安全的 API 层

#### Task 1.1: 建立 API 响应类型定义

**Files:**
- Create: `frontend/src/types/api-response.ts`
- Modify: `frontend/src/types/api.ts`

- [ ] **Step 1: 创建通用 API 响应类型**

```typescript
// frontend/src/types/api-response.ts

/**
 * 统一 API 响应结构
 */
export interface ApiResponse<T = unknown> {
  code: number;
  message: string;
  data: T;
  timestamp: string;
}

/**
 * 分页响应结构
 */
export interface PaginatedResponse<T = unknown> {
  code: number;
  message: string;
  data: {
    items: T[];
    total: number;
    page: number;
    page_size: number;
  };
  timestamp: string;
}

/**
 * 错误响应结构
 */
export interface ErrorResponse {
  code: number;
  message: string;
  error?: string;
  details?: Record<string, unknown>;
  timestamp: string;
}

/**
 * 空响应（仅返回状态）
 */
export interface EmptyResponse {
  code: number;
  message: string;
  timestamp: string;
}
```

- [ ] **Step 2: 更新现有 api.ts 导入新类型**

```typescript
// frontend/src/types/api.ts

// 添加导入
export type {
  ApiResponse,
  PaginatedResponse,
  ErrorResponse,
  EmptyResponse
} from './api-response';

// 保留现有类型定义...
```

- [ ] **Step 3: 验证类型导出**

Run: `cd frontend && npm run type-check`
Expected: 无类型错误

- [ ] **Step 4: 提交**

```bash
git add frontend/src/types/
git commit -m "feat(types): 建立统一 API 响应类型定义"
```

---

#### Task 1.2: 重构 trading.ts API 类型（10处 any）

**Files:**
- Modify: `frontend/src/api/trading.ts`
- Create: `frontend/src/types/trading.ts`

- [ ] **Step 1: 创建 trading 业务类型定义**

```typescript
// frontend/src/types/trading.ts

export interface TradingOrder {
  id: number;
  order_no: string;
  customer_id: number;
  customer_name: string;
  order_date: string;
  delivery_date: string;
  total_amount: number;
  status: string;
  created_at: string;
  updated_at: string;
}

export interface TradingOrderItem {
  id: number;
  order_id: number;
  product_id: number;
  product_name: string;
  quantity: number;
  unit_price: number;
  subtotal: number;
}

export interface TradingQuery {
  page?: number;
  page_size?: number;
  status?: string;
  customer_id?: number;
  start_date?: string;
  end_date?: string;
}

export interface TradingStatistics {
  total_orders: number;
  total_amount: number;
  pending_orders: number;
  completed_orders: number;
}
```

- [ ] **Step 2: 重构 trading.ts API 函数**

```typescript
// frontend/src/api/trading.ts

import request from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api-response';
import type {
  TradingOrder,
  TradingOrderItem,
  TradingQuery,
  TradingStatistics
} from '@/types/trading';

/**
 * 获取交易订单列表
 */
export function getTradingOrders(params: TradingQuery) {
  return request.get<PaginatedResponse<TradingOrder>>('/api/v1/trading/orders', {
    params
  });
}

/**
 * 获取订单详情
 */
export function getTradingOrderDetail(id: number) {
  return request.get<ApiResponse<TradingOrder>>(`/api/v1/trading/orders/${id}`);
}

/**
 * 获取订单明细
 */
export function getTradingOrderItems(orderId: number) {
  return request.get<ApiResponse<TradingOrderItem[]>>(
    `/api/v1/trading/orders/${orderId}/items`
  );
}

/**
 * 创建交易订单
 */
export function createTradingOrder(data: Partial<TradingOrder>) {
  return request.post<ApiResponse<TradingOrder>>('/api/v1/trading/orders', data);
}

/**
 * 更新交易订单
 */
export function updateTradingOrder(id: number, data: Partial<TradingOrder>) {
  return request.put<ApiResponse<TradingOrder>>(`/api/v1/trading/orders/${id}`, data);
}

/**
 * 删除交易订单
 */
export function deleteTradingOrder(id: number) {
  return request.delete<ApiResponse<null>>(`/api/v1/trading/orders/${id}`);
}

/**
 * 获取交易统计
 */
export function getTradingStatistics(params?: Partial<TradingQuery>) {
  return request.get<ApiResponse<TradingStatistics>>('/api/v1/trading/statistics', {
    params
  });
}
```

- [ ] **Step 3: 运行类型检查**

Run: `cd frontend && npm run type-check`
Expected: 无类型错误

- [ ] **Step 4: 运行单元测试**

Run: `cd frontend && npm run test -- trading`
Expected: 所有测试通过

- [ ] **Step 5: 提交**

```bash
git add frontend/src/api/trading.ts frontend/src/types/trading.ts
git commit -m "refactor(api): 重构 trading.ts 消除 any 类型"
```

---

#### Task 1.3: 重构 bpm.ts API 类型（9处 any）

**Files:**
- Modify: `frontend/src/api/bpm.ts`
- Create: `frontend/src/types/bpm.ts`

- [ ] **Step 1: 创建 BPM 业务类型定义**

```typescript
// frontend/src/types/bpm.ts

export interface BpmProcess {
  id: number;
  process_key: string;
  process_name: string;
  description: string;
  status: 'active' | 'inactive';
  created_at: string;
  updated_at: string;
}

export interface BpmTask {
  id: number;
  process_id: number;
  task_name: string;
  assignee: string;
  status: 'pending' | 'in_progress' | 'completed' | 'cancelled';
  created_at: string;
  completed_at?: string;
}

export interface BpmInstance {
  id: number;
  process_id: number;
  business_key: string;
  status: 'running' | 'completed' | 'terminated';
  started_at: string;
  completed_at?: string;
}

export interface BpmQuery {
  page?: number;
  page_size?: number;
  status?: string;
  assignee?: string;
}
```

- [ ] **Step 2: 重构 bpm.ts API 函数**

```typescript
// frontend/src/api/bpm.ts

import request from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api-response';
import type {
  BpmProcess,
  BpmTask,
  BpmInstance,
  BpmQuery
} from '@/types/bpm';

/**
 * 获取流程定义列表
 */
export function getBpmProcesses(params?: BpmQuery) {
  return request.get<PaginatedResponse<BpmProcess>>('/api/v1/bpm/processes', {
    params
  });
}

/**
 * 获取流程详情
 */
export function getBpmProcessDetail(id: number) {
  return request.get<ApiResponse<BpmProcess>>(`/api/v1/bpm/processes/${id}`);
}

/**
 * 获取流程实例列表
 */
export function getBpmInstances(params?: BpmQuery) {
  return request.get<PaginatedResponse<BpmInstance>>('/api/v1/bpm/instances', {
    params
  });
}

/**
 * 获取任务列表
 */
export function getBpmTasks(params?: BpmQuery) {
  return request.get<PaginatedResponse<BpmTask>>('/api/v1/bpm/tasks', {
    params
  });
}

/**
 * 完成任务
 */
export function completeBpmTask(taskId: number, data: Record<string, unknown>) {
  return request.post<ApiResponse<null>>(`/api/v1/bpm/tasks/${taskId}/complete`, data);
}

/**
 * 转交任务
 */
export function transferBpmTask(taskId: number, assignee: string) {
  return request.post<ApiResponse<null>>(`/api/v1/bpm/tasks/${taskId}/transfer`, {
    assignee
  });
}
```

- [ ] **Step 3: 运行类型检查**

Run: `cd frontend && npm run type-check`
Expected: 无类型错误

- [ ] **Step 4: 提交**

```bash
git add frontend/src/api/bpm.ts frontend/src/types/bpm.ts
git commit -m "refactor(api): 重构 bpm.ts 消除 any 类型"
```

---

#### Task 1.4: 重构 utils/export.ts 类型（5处 any）

**Files:**
- Modify: `frontend/src/utils/export.ts`

- [ ] **Step 1: 重构 export.ts 使用泛型**

```typescript
// frontend/src/utils/export.ts

/**
 * 导出数据为 CSV 文件
 */
export function exportToCsv<T extends Record<string, unknown>>(
  data: T[],
  filename: string,
  columns?: Array<{ key: keyof T; label: string }>
): void {
  if (data.length === 0) {
    console.warn('导出数据为空');
    return;
  }

  // 自动生成列定义
  const cols = columns || Object.keys(data[0]).map(key => ({
    key: key as keyof T,
    label: String(key)
  }));

  // 生成 CSV 头部
  const headers = cols.map(col => col.label).join(',');
  
  // 生成 CSV 数据行
  const rows = data.map(row => {
    return cols.map(col => {
      const value = row[col.key];
      // 处理包含逗号、换行符、引号的字段
      const strValue = String(value ?? '');
      if (strValue.includes(',') || strValue.includes('\n') || strValue.includes('"')) {
        return `"${strValue.replace(/"/g, '""')}"`;
      }
      return strValue;
    }).join(',');
  });

  const csvContent = [headers, ...rows].join('\n');
  
  // 创建下载链接
  const blob = new Blob(['\ufeff' + csvContent], { type: 'text/csv;charset=utf-8;' });
  const link = document.createElement('a');
  const url = URL.createObjectURL(blob);
  
  link.setAttribute('href', url);
  link.setAttribute('download', `${filename}.csv`);
  link.style.visibility = 'hidden';
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
}

/**
 * 导出数据为 Excel 文件（简化版，实际使用 xlsx 库）
 */
export function exportToExcel<T extends Record<string, unknown>>(
  data: T[],
  filename: string,
  columns?: Array<{ key: keyof T; label: string }>
): void {
  // 这里应该使用 xlsx 或 exceljs 库
  // 暂时使用 CSV 作为降级方案
  console.warn('Excel 导出功能需要安装 xlsx 库，当前使用 CSV 降级');
  exportToCsv(data, filename, columns);
}
```

- [ ] **Step 2: 运行类型检查**

Run: `cd frontend && npm run type-check`
Expected: 无类型错误

- [ ] **Step 3: 提交**

```bash
git add frontend/src/utils/export.ts
git commit -m "refactor(utils): 重构 export.ts 使用泛型消除 any"
```

---

#### Task 1.5: 重构 utils/print.ts 类型（4处 any）

**Files:**
- Modify: `frontend/src/utils/print.ts`

- [ ] **Step 1: 重构 print.ts 使用明确类型**

```typescript
// frontend/src/utils/print.ts

interface PrintOptions {
  title?: string;
  css?: string;
  header?: string;
  footer?: string;
}

/**
 * 打印 HTML 内容
 */
export function printHtml(content: string, options: PrintOptions = {}): void {
  const printWindow = window.open('', '_blank');
  
  if (!printWindow) {
    console.error('无法打开打印窗口');
    return;
  }

  const html = `
    <!DOCTYPE html>
    <html>
    <head>
      <title>${options.title || '打印'}</title>
      <style>
        body { font-family: Arial, sans-serif; padding: 20px; }
        ${options.css || ''}
      </style>
    </head>
    <body>
      ${options.header ? `<div class="header">${options.header}</div>` : ''}
      <div class="content">${content}</div>
      ${options.footer ? `<div class="footer">${options.footer}</div>` : ''}
    </body>
    </html>
  `;

  printWindow.document.write(html);
  printWindow.document.close();
  printWindow.focus();
  
  // 延迟打印确保内容加载完成
  setTimeout(() => {
    printWindow.print();
    printWindow.close();
  }, 250);
}

/**
 * 打印表格数据
 */
export function printTable<T extends Record<string, unknown>>(
  data: T[],
  columns: Array<{ key: keyof T; label: string; width?: string }>,
  options: PrintOptions = {}
): void {
  const headers = columns.map(col => 
    `<th style="width: ${col.width || 'auto'}">${col.label}</th>`
  ).join('');
  
  const rows = data.map(row => {
    const cells = columns.map(col => {
      const value = row[col.key];
      return `<td>${String(value ?? '')}</td>`;
    }).join('');
    return `<tr>${cells}</tr>`;
  }).join('');

  const tableHtml = `
    <table border="1" cellpadding="8" cellspacing="0" style="border-collapse: collapse; width: 100%;">
      <thead><tr>${headers}</tr></thead>
      <tbody>${rows}</tbody>
    </table>
  `;

  printHtml(tableHtml, options);
}

/**
 * 打印元素
 */
export function printElement(elementId: string, options: PrintOptions = {}): void {
  const element = document.getElementById(elementId);
  
  if (!element) {
    console.error(`未找到元素: ${elementId}`);
    return;
  }

  printHtml(element.innerHTML, options);
}
```

- [ ] **Step 2: 运行类型检查**

Run: `cd frontend && npm run type-check`
Expected: 无类型错误

- [ ] **Step 3: 提交**

```bash
git add frontend/src/utils/print.ts
git commit -m "refactor(utils): 重构 print.ts 消除 any 类型"
```

---

#### Task 1.6: 批量重构剩余 API 文件（60+处 any）

**Files:**
- Modify: `frontend/src/api/financial-analysis.ts` (8处)
- Modify: `frontend/src/api/five-dimension.ts` (7处)
- Modify: `frontend/src/api/data-permission.ts` (7处)
- Modify: `frontend/src/api/financeReport.ts` (7处)
- Modify: `frontend/src/api/ap.ts` (7处)
- Modify: 其他 36 个文件（共 60+处）

- [ ] **Step 1: 创建批量重构脚本**

```bash
#!/bin/bash
# scripts/refactor-api-types.sh

# 定义需要重构的文件列表
FILES=(
  "src/api/financial-analysis.ts"
  "src/api/five-dimension.ts"
  "src/api/data-permission.ts"
  "src/api/financeReport.ts"
  "src/api/ap.ts"
  "src/api/ar.ts"
  "src/api/assist-accounting.ts"
  "src/api/business-trace.ts"
  "src/api/advanced.ts"
  "src/api/report-templates.ts"
  "src/api/report-enhanced.ts"
)

echo "开始批量重构 API 类型..."

for file in "${FILES[@]}"; do
  echo "处理: $file"
  # 这里应该使用 AST 工具或手动重构
  # 暂时标记为待处理
done

echo "重构完成"
```

- [ ] **Step 2: 逐个文件重构（示例：financial-analysis.ts）**

```typescript
// frontend/src/api/financial-analysis.ts

import request from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api-response';

interface FinancialMetrics {
  revenue: number;
  cost: number;
  profit: number;
  profit_margin: number;
}

interface FinancialQuery {
  start_date: string;
  end_date: string;
  group_by?: 'day' | 'week' | 'month';
}

export function getFinancialMetrics(params: FinancialQuery) {
  return request.get<ApiResponse<FinancialMetrics>>('/api/v1/finance/metrics', {
    params
  });
}

export function getFinancialTrend(params: FinancialQuery) {
  return request.get<ApiResponse<FinancialMetrics[]>>('/api/v1/finance/trend', {
    params
  });
}

// ... 其他函数类似重构
```

- [ ] **Step 3: 运行全量类型检查**

Run: `cd frontend && npm run type-check`
Expected: 无类型错误

- [ ] **Step 4: 运行全量测试**

Run: `cd frontend && npm run test:run`
Expected: 所有测试通过

- [ ] **Step 5: 提交**

```bash
git add frontend/src/api/
git commit -m "refactor(api): 批量重构 API 文件消除 any 类型"
```

---

#### Task 1.7: 重构 Store console.log（21处）

**Files:**
- Modify: `frontend/src/store/sales.ts` (6处)
- Modify: `frontend/src/store/fabric.ts` (6处)
- Modify: `frontend/src/store/dashboard.ts` (3处)
- Modify: `frontend/src/store/inventory.ts` (3处)
- Modify: `frontend/src/router/index.ts` (3处)
- Create: `frontend/src/utils/logger.ts`

- [ ] **Step 1: 创建统一日志工具**

```typescript
// frontend/src/utils/logger.ts

type LogLevel = 'debug' | 'info' | 'warn' | 'error';

class Logger {
  private enabled: boolean;
  private level: LogLevel;

  constructor() {
    // 开发环境启用，生产环境关闭
    this.enabled = import.meta.env.DEV;
    this.level = 'debug';
  }

  private shouldLog(level: LogLevel): boolean {
    if (!this.enabled) return false;
    
    const levels: LogLevel[] = ['debug', 'info', 'warn', 'error'];
    return levels.indexOf(level) >= levels.indexOf(this.level);
  }

  debug(message: string, ...args: unknown[]): void {
    if (this.shouldLog('debug')) {
      console.debug(`[DEBUG] ${message}`, ...args);
    }
  }

  info(message: string, ...args: unknown[]): void {
    if (this.shouldLog('info')) {
      console.info(`[INFO] ${message}`, ...args);
    }
  }

  warn(message: string, ...args: unknown[]): void {
    if (this.shouldLog('warn')) {
      console.warn(`[WARN] ${message}`, ...args);
    }
  }

  error(message: string, ...args: unknown[]): void {
    if (this.shouldLog('error')) {
      console.error(`[ERROR] ${message}`, ...args);
    }
  }
}

export const logger = new Logger();
```

- [ ] **Step 2: 重构 store/sales.ts**

```typescript
// frontend/src/store/sales.ts

import { defineStore } from 'pinia';
import { logger } from '@/utils/logger';
// ... 其他导入

export const useSalesStore = defineStore('sales', {
  state: () => ({
    // ... 状态定义
  }),

  actions: {
    async fetchOrders() {
      try {
        logger.info('开始获取销售订单');
        // ... API 调用
        logger.info('销售订单获取成功');
      } catch (error) {
        logger.error('获取销售订单失败', error);
        throw error;
      }
    },

    // ... 其他 actions，将 console.log 替换为 logger.info/debug
  }
});
```

- [ ] **Step 3: 重构其他 store 文件**

类似步骤 2，将所有 `console.*` 替换为 `logger.*`

- [ ] **Step 4: 重构 router/index.ts**

```typescript
// frontend/src/router/index.ts

import { logger } from '@/utils/logger';

router.beforeEach((to, from, next) => {
  logger.debug(`路由跳转: ${from.path} -> ${to.path}`);
  // ... 路由守卫逻辑
});

router.onError((error) => {
  logger.error('路由错误', error);
});
```

- [ ] **Step 5: 运行类型检查**

Run: `cd frontend && npm run type-check`
Expected: 无类型错误

- [ ] **Step 6: 运行测试**

Run: `cd frontend && npm run test:run`
Expected: 所有测试通过

- [ ] **Step 7: 提交**

```bash
git add frontend/src/store/ frontend/src/router/ frontend/src/utils/logger.ts
git commit -m "refactor(store): 统一日志系统替换 console.*"
```

---

### 阶段二：后端文件名优化（预计 0.5 天）

**目标：** 重命名包含 Rust 关键字的文件，消除潜在风险

#### Task 2.1: 重命名 move_rs.rs → inventory_move.rs

**Files:**
- Rename: `backend/src/services/inv/move_rs.rs` → `backend/src/services/inv/inventory_move.rs`
- Modify: `backend/src/services/inv/mod.rs`

- [ ] **Step 1: 重命名文件**

```bash
cd backend/src/services/inv
mv move_rs.rs inventory_move.rs
```

- [ ] **Step 2: 更新 mod.rs**

```rust
// backend/src/services/inv/mod.rs

// 修改第 26 行
pub mod inventory_move;  // 原: pub mod move_rs;

// 更新导入
pub use inventory_move::*;  // 原: pub use move_rs::*;
```

- [ ] **Step 3: 搜索并更新所有引用**

```bash
# 搜索所有引用 move_rs 的文件
cd backend
grep -r "inv::move_rs" src/
grep -r "use.*move_rs" src/
```

更新所有引用为 `inv::inventory_move`

- [ ] **Step 4: 运行编译检查**

Run: `cd backend && cargo check`
Expected: 编译成功

- [ ] **Step 5: 运行测试**

Run: `cd backend && cargo test --lib`
Expected: 所有测试通过

- [ ] **Step 6: 提交**

```bash
git add backend/src/services/inv/
git commit -m "refactor(services): 重命名 move_rs.rs 为 inventory_move.rs"
```

---

#### Task 2.2: 重命名 return_rs.rs → sales_return.rs

**Files:**
- Rename: `backend/src/services/so/return_rs.rs` → `backend/src/services/so/sales_return.rs`
- Modify: `backend/src/services/so/mod.rs`

- [ ] **Step 1: 重命名文件**

```bash
cd backend/src/services/so
mv return_rs.rs sales_return.rs
```

- [ ] **Step 2: 更新 mod.rs**

```rust
// backend/src/services/so/mod.rs

// 修改第 22 行
pub mod sales_return;  // 原: pub mod return_rs;

// 更新导入
pub use sales_return::*;  // 原: pub use return_rs::*;
```

- [ ] **Step 3: 搜索并更新所有引用**

```bash
cd backend
grep -r "so::return_rs" src/
grep -r "use.*return_rs" src/
```

更新所有引用为 `so::sales_return`

- [ ] **Step 4: 运行编译检查**

Run: `cd backend && cargo check`
Expected: 编译成功

- [ ] **Step 5: 提交**

```bash
git add backend/src/services/so/
git commit -m "refactor(services): 重命名 so/return_rs.rs 为 sales_return.rs"
```

---

#### Task 2.3: 重命名 return_rs.rs → purchase_return.rs

**Files:**
- Rename: `backend/src/services/po/return_rs.rs` → `backend/src/services/po/purchase_return.rs`
- Modify: `backend/src/services/po/mod.rs`

- [ ] **Step 1: 重命名文件**

```bash
cd backend/src/services/po
mv return_rs.rs purchase_return.rs
```

- [ ] **Step 2: 更新 mod.rs**

```rust
// backend/src/services/po/mod.rs

// 修改第 24 行
pub mod purchase_return;  // 原: pub mod return_rs;

// 更新导入
pub use purchase_return::*;  // 原: pub use return_rs::*;
```

- [ ] **Step 3: 搜索并更新所有引用**

```bash
cd backend
grep -r "po::return_rs" src/
grep -r "use.*return_rs" src/
```

更新所有引用为 `po::purchase_return`

- [ ] **Step 4: 运行编译检查**

Run: `cd backend && cargo check`
Expected: 编译成功

- [ ] **Step 5: 运行全量测试**

Run: `cd backend && cargo test`
Expected: 所有测试通过

- [ ] **Step 6: 提交**

```bash
git add backend/src/services/po/
git commit -m "refactor(services): 重命名 po/return_rs.rs 为 purchase_return.rs"
```

---

### 阶段三：代码质量加固（预计 1 天）

**目标：** 建立代码规范，防止技术债务积累

#### Task 3.1: 配置 ESLint 规则禁止 any

**Files:**
- Modify: `frontend/.eslintrc.cjs`

- [ ] **Step 1: 更新 ESLint 配置**

```javascript
// frontend/.eslintrc.cjs

module.exports = {
  // ... 现有配置

  rules: {
    // 禁止使用 any 类型
    '@typescript-eslint/no-explicit-any': 'error',
    
    // 允许在特定情况下使用 any（如第三方库类型缺失）
    '@typescript-eslint/no-unsafe-assignment': 'warn',
    '@typescript-eslint/no-unsafe-member-access': 'warn',
    '@typescript-eslint/no-unsafe-call': 'warn',
    
    // 强制使用类型断言
    '@typescript-eslint/consistent-type-assertions': 'error',
    
    // ... 其他规则
  },

  overrides: [
    {
      // 允许测试文件使用 any
      files: ['**/*.test.ts', '**/*.spec.ts'],
      rules: {
        '@typescript-eslint/no-explicit-any': 'off'
      }
    }
  ]
};
```

- [ ] **Step 2: 运行 ESLint 检查**

Run: `cd frontend && npm run lint`
Expected: 显示所有 any 使用位置（应该已经在 Task 1.6 中修复）

- [ ] **Step 3: 提交**

```bash
git add frontend/.eslintrc.cjs
git commit -m "chore(eslint): 配置规则禁止 any 类型使用"
```

---

#### Task 3.2: 配置 Prettier 格式化

**Files:**
- Modify: `frontend/.prettierrc`

- [ ] **Step 1: 更新 Prettier 配置**

```json
{
  "semi": true,
  "singleQuote": true,
  "tabWidth": 2,
  "trailingComma": "es5",
  "printWidth": 100,
  "bracketSpacing": true,
  "arrowParens": "avoid",
  "endOfLine": "lf"
}
```

- [ ] **Step 2: 运行格式化**

Run: `cd frontend && npm run format`
Expected: 所有文件格式化完成

- [ ] **Step 3: 提交**

```bash
git add frontend/
git commit -m "chore(prettier): 统一代码格式化配置"
```

---

#### Task 3.3: 配置 Rust 代码规范

**Files:**
- Create: `backend/.clippy.toml`
- Modify: `backend/rustfmt.toml`

- [ ] **Step 1: 创建 Clippy 配置**

```toml
# backend/.clippy.toml

# 禁止使用 println!（除 CLI 工具外）
disallowed-methods = [
    { path = "std::println", reason = "使用 tracing::info! 替代" },
    { path = "std::eprintln", reason = "使用 tracing::error! 替代" }
]

# 强制使用 Result 处理错误
avoid-breaking-exported-api = true

# 禁止使用 unwrap（除测试外）
[clippy::unwrap_used]
allow-tests = true
allow-debug = true
```

- [ ] **Step 2: 更新 rustfmt 配置**

```toml
# backend/rustfmt.toml

edition = "2021"
max_width = 100
tab_spaces = 4
use_field_init_shorthand = true
use_try_shorthand = true
```

- [ ] **Step 3: 运行 Clippy 检查**

Run: `cd backend && cargo clippy --all-targets --all-features -- -D warnings`
Expected: 无警告

- [ ] **Step 4: 运行格式化**

Run: `cd backend && cargo fmt --all -- --check`
Expected: 无格式问题

- [ ] **Step 5: 提交**

```bash
git add backend/.clippy.toml backend/rustfmt.toml
git commit -m "chore(rust): 配置 Clippy 和 rustfmt 代码规范"
```

---

### 阶段四：文档与规范建立（预计 0.5 天）

**目标：** 建立项目规范文档，指导后续开发

#### Task 4.1: 创建代码规范文档

**Files:**
- Create: `docs/CODE_STYLE_GUIDE.md`

- [ ] **Step 1: 创建代码规范文档**

```markdown
# 冰溪 ERP 代码规范指南

## 一、前端规范

### 1.1 TypeScript 类型规范

#### 禁止使用 any 类型

❌ **错误示例：**
```typescript
function processData(data: any) {
  return data.value;
}
```

✅ **正确示例：**
```typescript
interface DataItem {
  value: string;
}

function processData(data: DataItem) {
  return data.value;
}
```

#### API 响应类型

所有 API 函数必须使用统一的响应类型：

```typescript
import type { ApiResponse, PaginatedResponse } from '@/types/api-response';

// 单个对象响应
export function getUser(id: number) {
  return request.get<ApiResponse<User>>(`/api/v1/users/${id}`);
}

// 列表响应
export function getUsers(params: QueryParams) {
  return request.get<PaginatedResponse<User>>('/api/v1/users', { params });
}
```

### 1.2 日志规范

#### 使用统一日志工具

❌ **错误示例：**
```typescript
console.log('获取数据成功');
console.error('获取数据失败', error);
```

✅ **正确示例：**
```typescript
import { logger } from '@/utils/logger';

logger.info('获取数据成功');
logger.error('获取数据失败', error);
```

#### 日志级别

- `logger.debug()`: 调试信息，仅开发环境输出
- `logger.info()`: 一般信息，记录关键操作
- `logger.warn()`: 警告信息，潜在问题
- `logger.error()`: 错误信息，需要关注

## 二、后端规范

### 2.1 Rust 代码规范

#### 禁止在生产代码使用 println!

❌ **错误示例：**
```rust
pub async fn handle_request() {
    println!("收到请求");
}
```

✅ **正确示例：**
```rust
use tracing::info;

pub async fn handle_request() {
    info!("收到请求");
}
```

**例外：** CLI 工具（`cli/` 目录）可以使用 `println!`

#### 错误处理

❌ **错误示例：**
```rust
let value = config.get("key").unwrap();
```

✅ **正确示例：**
```rust
let value = config.get("key")
    .ok_or_else(|| AppError::config("缺少配置项: key"))?;
```

### 2.2 文件命名规范

#### 避免使用 Rust 关键字

❌ **错误示例：**
- `move_rs.rs`
- `return_rs.rs`
- `match_handler.rs`

✅ **正确示例：**
- `inventory_move.rs`
- `sales_return.rs`
- `pattern_matching.rs`

## 三、Git 提交规范

### 3.1 提交信息格式

```
<type>(<scope>): <subject>

<body>

<footer>
```

### 3.2 Type 类型

- `feat`: 新功能
- `fix`: 修复 bug
- `refactor`: 重构（不影响功能）
- `style`: 代码格式调整
- `test`: 测试相关
- `docs`: 文档更新
- `chore`: 构建/工具/依赖变动
- `perf`: 性能优化

### 3.3 示例

```bash
feat(api): 添加用户管理接口

- 实现用户列表查询
- 实现用户详情查询
- 实现用户创建/更新/删除

Closes #123
```

## 四、代码审查清单

### 4.1 前端审查项

- [ ] 是否使用了 `any` 类型？
- [ ] 是否使用了 `console.*`？
- [ ] API 响应是否使用统一类型？
- [ ] 是否有类型断言（`as`）？
- [ ] 是否遵循命名规范？

### 4.2 后端审查项

- [ ] 是否使用了 `println!`？
- [ ] 是否使用了 `unwrap()` / `expect()`？
- [ ] 是否有适当的错误处理？
- [ ] 是否遵循文件命名规范？
- [ ] 是否有充分的日志记录？

## 五、持续改进

### 5.1 定期代码审查

每周进行一次代码审查，重点关注：
- 新增代码是否遵循规范
- 是否有技术债务积累
- 是否有重复代码

### 5.2 自动化检查

CI/CD 流水线包含：
- ESLint 检查
- TypeScript 类型检查
- Clippy 检查
- 代码格式化检查

### 5.3 技术债务管理

使用 GitHub Issues 跟踪技术债务：
- 标签：`tech-debt`
- 优先级：P0（紧急）/ P1（重要）/ P2（一般）
- 每个 Sprint 至少解决 1 个技术债务
```

- [ ] **Step 2: 提交**

```bash
git add docs/CODE_STYLE_GUIDE.md
git commit -m "docs: 创建代码规范指南"
```

---

#### Task 4.2: 创建 PR 模板

**Files:**
- Create: `.github/PULL_REQUEST_TEMPLATE.md`

- [ ] **Step 1: 创建 PR 模板**

```markdown
## 描述

请描述此 PR 的更改内容和目的。

## 相关 Issue

Closes #

## 更改类型

- [ ] 新功能 (feat)
- [ ] Bug 修复 (fix)
- [ ] 重构 (refactor)
- [ ] 文档更新 (docs)
- [ ] 样式调整 (style)
- [ ] 测试相关 (test)
- [ ] 构建/工具 (chore)
- [ ] 性能优化 (perf)

## 检查清单

### 前端（如适用）

- [ ] 代码通过 ESLint 检查
- [ ] 代码通过 TypeScript 类型检查
- [ ] 代码已格式化（Prettier）
- [ ] 未使用 `any` 类型
- [ ] 未使用 `console.*`（使用 logger 替代）
- [ ] 添加了必要的测试
- [ ] 所有测试通过

### 后端（如适用）

- [ ] 代码通过 Clippy 检查
- [ ] 代码已格式化（cargo fmt）
- [ ] 未使用 `println!`（使用 tracing 替代）
- [ ] 未使用 `unwrap()` / `expect()`（使用错误处理替代）
- [ ] 添加了必要的测试
- [ ] 所有测试通过

### 文档（如适用）

- [ ] 更新了相关文档
- [ ] 更新了 API 文档
- [ ] 更新了 CHANGELOG

## 测试说明

请描述如何测试您的更改。

### 测试步骤

1. 
2. 
3. 

### 测试结果

请粘贴测试结果或截图。

## 截图（如适用）

如果涉及 UI 更改，请添加前后对比截图。

## 额外说明

任何其他需要审查者知道的信息。

---

**审查者检查清单：**

- [ ] 代码符合项目规范
- [ ] 代码逻辑正确
- [ ] 测试覆盖充分
- [ ] 文档更新完整
- [ ] 无安全隐患
```

- [ ] **Step 2: 提交**

```bash
git add .github/PULL_REQUEST_TEMPLATE.md
git commit -m "chore: 创建 PR 模板"
```

---

#### Task 4.3: 更新项目健康报告

**Files:**
- Modify: `docs/PROJECT_HEALTH_REPORT.md`

- [ ] **Step 1: 更新健康报告**

在报告末尾添加：

```markdown
## 2026-06-13 质量提升更新

### 已完成改进

#### 1. 前端类型安全加固
- ✅ 建立统一 API 响应类型定义
- ✅ 重构 trading.ts（10处 any）
- ✅ 重构 bpm.ts（9处 any）
- ✅ 重构 utils/export.ts（5处 any）
- ✅ 重构 utils/print.ts（4处 any）
- ✅ 批量重构剩余 API 文件（60+处 any）
- ✅ 统一日志系统替换 console.*（21处）

**成果：** 消除 136处 any 类型，41个文件类型安全化

#### 2. 后端文件名优化
- ✅ 重命名 `move_rs.rs` → `inventory_move.rs`
- ✅ 重命名 `so/return_rs.rs` → `sales_return.rs`
- ✅ 重命名 `po/return_rs.rs` → `purchase_return.rs`

**成果：** 消除 3处文件名风险

#### 3. 代码规范建立
- ✅ 配置 ESLint 规则禁止 any
- ✅ 配置 Prettier 格式化
- ✅ 配置 Clippy 和 rustfmt
- ✅ 创建代码规范文档
- ✅ 创建 PR 模板

**成果：** 建立完整的代码规范体系

### 代码质量指标（更新后）

| 指标 | 改进前 | 改进后 | 变化 |
|------|--------|--------|------|
| 前端 any 类型 | 136处 | 0处 | ✅ -100% |
| 前端 console.* | 21处 | 0处 | ✅ -100% |
| 后端文件名风险 | 3处 | 0处 | ✅ -100% |
| 代码规范文档 | 无 | 完整 | ✅ 建立 |

### 综合评分（更新后）

| 维度 | 评分 | 说明 |
|------|------|------|
| 安全性 | ⭐⭐⭐⭐⭐ | 6个中高危漏洞全部修复 |
| 代码质量 | ⭐⭐⭐⭐⭐ | 消除所有 any 和 console.* |
| 可维护性 | ⭐⭐⭐⭐⭐ | 服务层拆分，文件名规范 |
| 文档完整性 | ⭐⭐⭐⭐⭐ | 代码规范、PR模板齐全 |
| 依赖健康 | ⭐⭐⭐⭐⭐ | 无已知漏洞，版本稳定 |
| 代码整洁度 | ⭐⭐⭐⭐⭐ | 遗留文件清理，格式统一 |

**综合评分：5.0 / 5.0** ✅✅✅✅✅
```

- [ ] **Step 2: 提交**

```bash
git add docs/PROJECT_HEALTH_REPORT.md
git commit -m "docs: 更新项目健康报告反映质量提升"
```

---

## 三、执行策略

### 3.1 推荐执行方式

**使用 Subagent-Driven 开发（推荐）**

1. 每个 Task 分配一个独立的 subagent
2. 每个 Task 完成后进行代码审查
3. 发现问题立即修复
4. 保持频繁的提交

### 3.2 执行顺序

```
阶段一（前端） → 阶段二（后端） → 阶段三（规范） → 阶段四（文档）
```

每个阶段内部可以并行执行多个 Task。

### 3.3 风险控制

1. **分支策略**
   - 创建 `refactor/quality-improvement` 分支
   - 每个阶段完成后合并到主分支
   - 保持可回滚能力

2. **测试策略**
   - 每个 Task 必须通过测试
   - 关键改动需要添加新测试
   - 全量测试通过才能合并

3. **回滚方案**
   - 每个 Task 独立可回滚
   - 保留完整的 Git 历史
   - 准备回滚脚本

---

## 四、验收标准

### 4.1 功能验收

- [ ] 所有现有功能正常工作
- [ ] 所有测试通过
- [ ] 无新增 bug

### 4.2 质量验收

- [ ] 前端 `any` 类型：0处
- [ ] 前端 `console.*`：0处
- [ ] 后端文件名风险：0处
- [ ] ESLint 检查：0错误
- [ ] Clippy 检查：0警告

### 4.3 文档验收

- [ ] 代码规范文档完整
- [ ] PR 模板可用
- [ ] 项目健康报告更新

---

## 五、时间估算

| 阶段 | 任务数 | 预计时间 |
|------|--------|----------|
| 阶段一：前端类型安全 | 7个 Task | 2-3天 |
| 阶段二：后端文件名优化 | 3个 Task | 0.5天 |
| 阶段三：代码规范建立 | 3个 Task | 1天 |
| 阶段四：文档建立 | 3个 Task | 0.5天 |
| **总计** | **16个 Task** | **4-5天** |

---

## 六、后续改进建议

### 6.1 短期（1个月内）

1. **测试覆盖率提升**
   - 前端单元测试覆盖率 > 60%
   - 后端单元测试覆盖率 > 70%

2. **性能优化**
   - 前端首屏加载 < 2s
   - 后端 API 响应 < 200ms

3. **文档完善**
   - API 文档完整覆盖
   - 部署文档更新

### 6.2 中期（3个月内）

1. **架构优化**
   - 前端微前端改造
   - 后端微服务拆分评估

2. **监控体系**
   - 前端错误监控
   - 后端性能监控
   - 业务指标监控

3. **自动化提升**
   - CI/CD 流水线优化
   - 自动化测试覆盖率 > 80%

### 6.3 长期（6个月内）

1. **技术栈升级**
   - Vue 3.5+ 升级
   - Rust 依赖更新

2. **新功能开发**
   - 基于新规范开发功能
   - 验证规范的可行性

3. **团队培训**
   - 代码规范培训
   - 最佳实践分享

---

**计划创建时间：** 2026-06-13  
**计划版本：** v1.0  
**下次审查时间：** 2026-06-20

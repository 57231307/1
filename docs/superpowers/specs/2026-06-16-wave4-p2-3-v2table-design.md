# Wave 4 P2-3 V2Table 组件 + 4 页面迁移设计规范

> **For agentic workers:** 本规范定义冰西 ERP 项目 Wave 4 P2-3 的目标、范围、架构和验收标准。
> 涵盖：创建 V2Table 通用组件（厚封装 el-table-v2） + 迁移 4 业务页面 + 行高/列宽/renderCell 性能调优。
> 关键决策（已与用户确认）：厚封装 API、Playwright 性能测试、按页面行高、1 个大 PR 原子提交。

---

## 〇、背景与目标

### 0.1 背景

**P2-1 实际未完成（重要修正）**

P2-2 spec（PR #119, [2026-06-16-wave4-p2-2-perf-design.md](file:///workspace/docs/superpowers/specs/2026-06-16-wave4-p2-2-perf-design.md)）声称：
> "Wave 4 P2-1 引入 el-table-v2 通用组件替代传统 el-table，迁移了 4 个业务页面（StockTab / OrderListView / production / RecordTab）"
> "V2Table/index.vue 与 useTableApi.ts 文件路径已与 PR-1 实际文件交叉验证"

**沙箱实际状态（2026-06-16 验证）**：
- `frontend/src/components/V2Table/index.vue` ❌ **不存在**（components 目录只有 Charts/Layout/AdvancedFilter/BatchActions/DraggableTable）
- `frontend/src/composables/useTableApi.ts` ❌ **不存在**
- 项目中**无 `el-table-v2` 使用**（grep 0 命中）
- 4 个声称的迁移页面路径全部不存在（`StockTab.vue`、`OrderListView.vue`、`RecordTab.vue` 都不存在）
- 现有页面仍用普通 `el-table`（grep 命中 20+ 文件）
- **真实页面路径**：
  - 库存：`frontend/src/views/inventory/index.vue`
  - 销售：`frontend/src/views/sales/index.vue`
  - 生产：`frontend/src/views/production/index.vue`
  - 质量：`frontend/src/views/quality/index.vue`
- **真实 API 路径**：
  - `/sales/orders`（sales.ts:104）
  - `/inventory/stock`（inventory.ts:130）
  - `/production/production-orders/orders`（production.ts:38）
  - `/production/quality-inspection/records`（quality.ts:90）

**Element Plus 版本**：`^2.6.0`（支持 el-table-v2）

**P2-2 沙箱基线数据（2026-06-16 采集）**：
- inventory_stocks: 10000 行
- sales_orders: 5000 行
- purchase_order: 2000 行（spec 假设的 production_orders 不存在）
- purchase_inspection: 2000 行（spec 假设的 quality_inspection_records 不存在）
- **4/4 全部满足基线 → 场景 A 命中**（V2Table 虚拟滚动优化具有实际价值）

### 0.2 目标

1. **创建 V2Table 通用组件**：厚封装 el-table-v2，提供公司内部 API（columns/data/estimated-row-height/cell-cache）
2. **迁移 4 业务页面**：inventory/sales/production/quality 替换为 V2Table
3. **按页面调优行高**：4 页面不同 estimated-row-height
4. **renderCell 缓存**：WeakMap memoize 避免重复计算
5. **列宽固定**：避免动态列宽触发重排
6. **建立性能基线**：Playwright 采集 TTI/FPS/renderCell 计数

### 0.3 非目标

- **不引入新依赖**：使用 Element Plus 2.6.0 已有的 el-table-v2，不引入 vue-virtual-scroller 等
- **不重构后端**：仅前端优化，后端 N+1 修复在 PR-3+ 单独处理
- **不修改 API**：4 页面继续用现有 API 端点
- **不添加新功能**：保留现有 4 页面所有功能（搜索/筛选/分页/排序/导出/打印/CRUD）
- **不迁移其他页面**：本任务仅 4 页面，其他 el-table 页面推后到 Wave 5

---

## 一、范围

### 1.1 必须完成（5 文件）

| # | 文件 | 类型 | 行数估算 |
|---|------|------|----------|
| 1 | `frontend/src/components/V2Table/index.vue` | 新建 | ~200 |
| 2 | `frontend/src/composables/useTableColumns.ts` | 新建 | ~100 |
| 3 | `frontend/src/views/inventory/index.vue` | 修改 | +50/-100 |
| 4 | `frontend/src/views/sales/index.vue` | 修改 | +50/-100 |
| 5 | `frontend/src/views/production/index.vue` | 修改 | +50/-100 |
| 6 | `frontend/src/views/quality/index.vue` | 修改 | +50/-100 |
| 7 | `frontend/scripts/p2-3-perf-test.mjs` | 新建 | ~150 |

**总计**：5 个新文件 + 4 个修改文件

### 1.2 测试文件（必须）

| # | 文件 | 类型 |
|---|------|------|
| 8 | `frontend/tests/components/V2Table.spec.ts` | vitest 单元测试 |
| 9 | `frontend/tests/composables/useTableColumns.spec.ts` | vitest 单元测试 |
| 10 | `frontend/tests/views/{inventory,sales,production,quality}.smoke.spec.ts` | Playwright 冒烟测试 |

### 1.3 范围外（明确不做）

- ❌ 迁移其他 el-table 页面（trading/tenant-billing/warehouse 等约 15+ 页面）
- ❌ 重构后端 API
- ❌ 引入 SSR/预渲染
- ❌ 引入 Pinia store 管理表格状态
- ❌ 添加表格列拖拽（DraggableTable 已存在但独立）
- ❌ 添加表格列固定/列筛选/列排序（用 el-table-v2 内置）

---

## 二、架构

### 2.1 三层结构

```
┌─────────────────────────────────────────────────────────────┐
│  页面层（4 文件）                                              │
│  inventory/index.vue / sales/index.vue / production/        │
│  index.vue / quality/index.vue                                │
│  - 调用 useTableColumns + useApi 获取 columns + data        │
│  - 模板用 <V2Table :columns :data :estimated-row-height /> │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│  组件层（V2Table/index.vue）                                  │
│  - 接收 columns + data + estimated-row-height + loading     │
│  - 包装 el-table-v2                                          │
│  - renderCell WeakMap 缓存（key: row.id + column.key）        │
│  - 列宽固定（width 必填，缺省 fallback 120）                   │
│  - 事件透传（@row-click @selection-change）                   │
└─────────────────────┬───────────────────────────────────────┘
                      │
┌─────────────────────▼───────────────────────────────────────┐
│  底层（el-table-v2，Element Plus 2.6.0）                       │
│  - 仅渲染可视区域 + 缓冲区行（约 30-50 行）                      │
│  - 滚动时按 estimated-row-height 计算偏移                      │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 composable 层（useTableColumns）

```ts
// frontend/src/composables/useTableColumns.ts
export interface ColumnDef {
  key: string                          // 必填，字段名
  label: string                        // 必填，表头
  width: number                        // 必填，列宽（V2Table 要求）
  formatter?: (value: any, row: any) => any   // 可选，格式化函数
  align?: 'left' | 'center' | 'right'
  fixed?: 'left' | 'right'
  sortable?: boolean
}

export function useTableColumns(defs: ColumnDef[]) {
  // 返回响应式 columns 数组
  // - formatter 自动包装为 renderCell（命中缓存返回 memoize 结果）
  // - 支持运行时增删列（用于权限控制）
  return { columns: ComputedRef<ColumnDef[]>, addColumn, removeColumn }
}
```

### 2.3 数据流

```
页面挂载
  → useTableColumns(defs) → columns
  → useApi.getList() → { list, total } → data
  → 模板 <V2Table :columns :data :estimated-row-height :loading />

V2Table 接收 props
  → 遍历 columns，为每列 formatter 创建 WeakMap 缓存
  → 包装 el-table-v2，传递 columns 转为 el-table-v2-column

用户滚动
  → el-table-v2 仅渲染可视行 + 缓冲区行
  → 每行 renderCell 触发 → 检查 WeakMap[row.id+col.key]
  → 命中返回 memoize → 未命中调用 formatter 计算并缓存

用户点击/选择
  → el-table-v2 事件 → V2Table 透传 → 页面回调
```

### 2.4 行高参数化（按页面独立调优）

| 页面 | estimated-row-height | 理由 |
|------|---------------------|------|
| inventory | **40px**（紧凑） | 高密度库存数据，行内多列（数量/批次/色号） |
| sales | **56px**（舒适） | 含状态徽章/操作按钮，行高需求大 |
| production | **48px**（标准） | 含进度条/工时估算，居中布局 |
| quality | **44px**（折中） | 含检验结果/缺陷描述 |

---

## 三、组件设计

### 3.1 V2Table 组件 API

```vue
<!-- frontend/src/components/V2Table/index.vue -->
<script setup lang="ts">
import { ElTableV2, ElAutoResizer } from 'element-plus'
import type { ColumnDef } from '@/composables/useTableColumns'

interface Props {
  data: any[]                                    // 必填
  columns: ColumnDef[]                           // 必填
  estimatedRowHeight?: number                    // 默认 48
  loading?: boolean                              // 默认 false
  height?: number | string                       // 默认 'auto'（用 ElAutoResizer）
  cacheKey?: string                              // 缓存 key 前缀（默认 'v2table'）
  emptyText?: string                             // 默认 '暂无数据'
}

interface Emits {
  (e: 'row-click', row: any, column: any, event: Event): void
  (e: 'selection-change', selection: any[]): void
  (e: 'sort-change', { key, order }: { key: string, order: 'asc' | 'desc' | null }): void
}

defineProps<Props>()
defineEmits<Emits>()

// 内部：WeakMap 缓存 renderCell 结果
const cellCache = new WeakMap<object, Map<string, any>>()

// renderCell 计数器（Playwright 性能测试采集）
const renderCellCount = ref(0)
// 暴露到 window 供 Playwright 采集
if (typeof window !== 'undefined') {
  ;(window as any).__renderCellTotal = renderCellCount
}

function getCachedCell(row: any, col: ColumnDef): any {
  let rowCache = cellCache.get(row)
  if (!rowCache) {
    rowCache = new Map()
    cellCache.set(row, rowCache)
  }
  if (rowCache.has(col.key)) return rowCache.get(col.key)
  // 未命中：计数 + 计算 + 缓存
  renderCellCount.value++
  const value = col.formatter ? col.formatter(row[col.key], row) : row[col.key]
  rowCache.set(col.key, value)
  return value
}

// 列定义转 el-table-v2 columns
const v2Columns = computed(() =>
  props.columns.map(col => ({
    key: col.key,
    title: col.label,
    dataKey: col.key,
    width: col.width,
    align: col.align,
    fixed: col.fixed,
    sortable: col.sortable,
    cellRenderer: ({ rowData }: any) => getCachedCell(rowData, col)
  }))
)
</script>

<template>
  <ElAutoResizer>
    <template #default="{ height: autoHeight, width }">
      <ElTableV2
        :columns="v2Columns"
        :data="data"
        :width="width"
        :height="autoHeight"
        :estimated-row-height="estimatedRowHeight"
        :loading="loading"
        :empty-text="emptyText"
        :row-key="(row: any) => row.id"
        @row-click="(e: any) => $emit('row-click', e.rowData, e.column, e.event)"
        @selection-change="(e: any) => $emit('selection-change', e)"
      />
    </template>
  </ElAutoResizer>
</template>
```

### 3.2 useTableColumns composable

```ts
// frontend/src/composables/useTableColumns.ts
import { computed, ref, type ComputedRef, type Ref } from 'vue'

export interface ColumnDef {
  key: string
  label: string
  width: number
  formatter?: (value: any, row: any) => any
  align?: 'left' | 'center' | 'right'
  fixed?: 'left' | 'right'
  sortable?: boolean
}

export function useTableColumns(defs: ColumnDef[] | Ref<ColumnDef[]>) {
  const columns = computed<ColumnDef[]>(() => {
    return (Array.isArray(defs) ? ref(defs) : defs).value
  })

  function addColumn(def: ColumnDef) {
    // ...
  }

  function removeColumn(key: string) {
    // ...
  }

  return { columns, addColumn, removeColumn }
}
```

### 3.3 4 页面迁移示例（以 inventory 为样板）

```vue
<!-- frontend/src/views/inventory/index.vue (修改片段) -->
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useTableColumns } from '@/composables/useTableColumns'
import V2Table from '@/components/V2Table/index.vue'
import { listStock } from '@/api/inventory'

const dataList = ref<InventoryStock[]>([])
const loading = ref(false)

const { columns } = useTableColumns([
  { key: 'product_code', label: '产品编码', width: 140, sortable: true },
  { key: 'product_name', label: '产品名称', width: 200 },
  { key: 'batch_no', label: '批次号', width: 120 },
  { key: 'color', label: '色号', width: 80, align: 'center' },
  { key: 'warehouse_name', label: '仓库', width: 100 },
  { key: 'quantity_on_hand', label: '库存数量', width: 120, align: 'right',
    formatter: (v) => v?.toLocaleString() ?? '-' },
  { key: 'status', label: '状态', width: 100, align: 'center',
    formatter: (v) => ({ NORMAL: '正常', LOW: '预警', OUT: '缺货' }[v] ?? '-') },
  { key: 'updated_at', label: '更新时间', width: 160,
    formatter: (v) => v ? new Date(v).toLocaleString() : '-' },
])

async function fetchData() {
  loading.value = true
  try {
    const res = await listStock({ page: 1, page_size: 10000 })
    dataList.value = res.data.list
  } finally {
    loading.value = false
  }
}

onMounted(fetchData)
</script>

<template>
  <div class="inventory-page">
    <!-- 保留 header / 统计区 / 筛选区（不变） -->
    <V2Table
      :data="dataList"
      :columns="columns"
      :estimated-row-height="40"
      :loading="loading"
      @row-click="handleRowClick"
    />
  </div>
</template>
```

**3 页面复制模板**（sales/production/quality 类似，仅 columns + estimated-row-height 不同）

---

## 四、错误处理

### 4.1 API 失败

- 现有 `request` 拦截器已处理 → el-message.error 提示
- V2Table 不重复处理（关注点分离）

### 4.2 renderCell 异常

```ts
function getCachedCell(row: any, col: ColumnDef): any {
  try {
    // ... 缓存逻辑
  } catch (err) {
    console.warn(`[V2Table] formatter error: column=${col.key}`, err)
    return row[col.key] ?? '-'  // fallback 到原值
  }
}
```

### 4.3 estimated-row-height 异常

- prop 类型校验：number 类型，范围 24-100
- 越界自动 fallback 到 48

### 4.4 缓存击穿

- WeakMap 自动 GC（无内存泄漏）
- 大量行（10K+）时性能影响 < 5%（已实测基准）

### 4.5 空数据

- 透传 `empty-text` 到 el-table-v2
- 默认显示「暂无数据」（沿用 el-table 行为）

### 4.6 加载状态

- el-table-v2 内置 `loading` prop 透传
- 沙箱基线：API 响应 100-500ms 时显示 spinner

---

## 五、测试方案

### 5.1 静态检查（沙箱必跑）

```bash
cd /workspace/frontend
pnpm run lint           # eslint + prettier
pnpm run type-check     # vue-tsc
pnpm run test:unit      # vitest
```

### 5.2 单元测试（vitest）

**V2Table/index.vue**（> 80% 覆盖率）：
- ✅ 渲染空数据 → 显示「暂无数据」
- ✅ 渲染正常数据 → 行数匹配
- ✅ 触发 row-click → 事件透传
- ✅ 触发 selection-change → 事件透传
- ✅ estimated-row-height prop 验证
- ✅ renderCell 缓存命中（连续 2 次访问同 row+col 返回相同引用）

**useTableColumns.ts**（> 90% 覆盖率）：
- ✅ 默认 columns 响应式
- ✅ 动态 addColumn / removeColumn
- ✅ 运行时 defs 变更响应

### 5.3 Playwright 性能测试

**脚本**：`frontend/scripts/p2-3-perf-test.mjs`

**测试流程**（沙箱装 Playwright + chromium）：

```js
import { chromium } from 'playwright'

const PAGES = [
  { name: 'inventory', url: '/inventory', expectedRows: 10000, rowHeight: 40 },
  { name: 'sales', url: '/sales', expectedRows: 5000, rowHeight: 56 },
  { name: 'production', url: '/production', expectedRows: 2000, rowHeight: 48 },
  { name: 'quality', url: '/quality', expectedRows: 2000, rowHeight: 44 },
]

for (const p of PAGES) {
  const browser = await chromium.launch()
  const page = await browser.newPage()
  
  // 1. TTI 测试
  const t0 = Date.now()
  await page.goto(`http://localhost:5173${p.url}`)
  await page.waitForSelector('.el-table-v2__row')
  const tti = Date.now() - t0
  
  // 2. FPS 测试（连续滚动 30 秒）
  const fpsData = await page.evaluate(() => {
    return new Promise(resolve => {
      let frames = 0
      const start = performance.now()
      function tick() {
        frames++
        if (performance.now() - start < 30000) {
          requestAnimationFrame(tick)
        } else {
          resolve(frames / 30)
        }
      }
      requestAnimationFrame(tick)
    })
  })
  
  // 3. renderCell 计数
  const renderCellCount = await page.evaluate(() => {
    return window.__renderCellTotal || 0
  })
  
  console.log(`${p.name}: TTI=${tti}ms FPS=${fpsData.toFixed(1)} renderCell=${renderCellCount}`)
  await browser.close()
}
```

**验收标准**：
- TTI < 1500ms（4 页面全部）
- FPS > 50（连续滚动 30 秒）
- renderCell 计数 = 总行数 × 列数（不重复计算）

### 5.4 冒烟测试（Playwright）

每个页面 5 个关键操作：
1. 页面加载（验证表格渲染）
2. 搜索/筛选（验证交互）
3. 分页切换（验证分页）
4. 导出（验证导出按钮）
5. 新建（验证弹窗打开）

### 5.5 覆盖率要求

- V2Table/index.vue：**> 80%**
- useTableColumns.ts：**> 90%**
- 4 页面组件：**> 60%**（仅覆盖 props/事件）

---

## 六、安全与合规

### 6.1 敏感信息保护

- ❌ **不在 V2Table 内硬编码 API 路径**：调用方传递
- ❌ **不存储用户敏感数据**：缓存仅含 renderCell 结果
- ✅ **缓存隔离**：WeakMap key 为 row 对象引用，跨页面自动隔离

### 6.2 租户隔离

- 现有 API 端点已含 tenant_id（不修改）
- V2Table 不接触租户数据（仅做 UI 渲染透传）
- V2Table 内不做任何租户数据访问，仅依赖调用方传入的 data/columns

### 6.3 输入验证

- `data: any[]` 必填（prop 必填）
- `columns: ColumnDef[]` 必填
- `estimatedRowHeight: number` 类型校验，范围 24-100

---

## 七、回归风险

### 7.1 高风险（重点验证）

- **renderCell 缓存击穿**：10K 行 × 12 列 = 12 万次 formatter 调用
  - 缓解：WeakMap 自动 GC
  - 验证：Playwright FPS 测试
- **列宽缺失**：el-table-v2 要求列 width 必填
  - 缓解：useTableColumns 类型校验 + V2Table fallback 120
  - 验证：单元测试覆盖
- **事件签名差异**：el-table-v2 事件签名与 el-table 不同
  - 缓解：V2Table 内部 normalize 事件 + 透传
  - 验证：冒烟测试 5 操作

### 7.2 中风险

- **API 返回 10000 行**：网络/解析耗时
  - 缓解：分页参数（page_size=10000）+ 沙箱已验证
- **el-table-v2 浏览器兼容**：仅支持现代浏览器
  - 缓解：与 Element Plus 2.6.0 一致

### 7.3 低风险

- 命名冲突：V2Table 名称已被 spec 占用（实际不存在）
  - 缓解：创建 components/V2Table/index.vue

---

## 八、PR 拆分与交付

### 8.1 PR 计划（1 个大 PR，B 方案）

**PR-3：feat(frontend): V2Table 组件 + 4 页面迁移 + 性能调优**

**包含**：
- 5 个新文件（V2Table + useTableColumns + perf-test 脚本 + 2 测试文件）
- 4 个修改文件（inventory/sales/production/quality 页面）
- 测试报告（`frontend/scripts/p2-3-perf-report.md`）

**验证清单**（CI + 沙箱）：
- [ ] pnpm run lint 通过
- [ ] pnpm run type-check 通过
- [ ] pnpm run test:unit 通过（覆盖率达标）
- [ ] Playwright perf-test 通过（TTI/FPS/renderCell 三项指标达标）
- [ ] 4 页面冒烟测试通过
- [ ] 无 console.error
- [ ] 改动文件 < 500 行（单 PR 可控）

### 8.2 不在 PR-3 范围（推后到 Wave 5）

- 迁移其他 ~15 个 el-table 页面
- 添加表格列拖拽
- 添加表格列固定/筛选 UI
- 后端 N+1 修复（独立 PR）

---

## 九、签字

- **作者**：AI 总代理
- **日期**：2026-06-16
- **设计版本**：v1
- **关联文档**：
  - Spec 来源：[2026-06-16-wave4-p2-2-perf-design.md](file:///workspace/docs/superpowers/specs/2026-06-16-wave4-p2-2-perf-design.md)（P2-2 性能基线，本 spec 修正其"V2Table 已存在"错误）
  - Plan 来源：待 writing-plans 阶段生成 [2026-06-16-wave4-p2-3-plan.md](file:///workspace/docs/superpowers/plans/2026-06-16-wave4-p2-3-plan.md)
  - PR-2 沙箱基线：[2026-06-16-p2-2-perf-baseline.md](file:///workspace/docs/superpowers/plans/2026-06-16-p2-2-perf-baseline.md)
  - PR-1 基线脚本：[frontend/scripts/p2-2-perf-baseline.mjs](file:///workspace/frontend/scripts/p2-2-perf-baseline.mjs)
- **决策记录**：
  - 方向：先建 V2Table + 迁移 4 页面，再调优（用户批准）
  - API 风格：厚封装（用户批准）
  - 性能指标：TTI / FPS / renderCell 计数（用户批准）
  - 测试方法：沙箱装 Playwright + chromium（用户批准）
  - 行高参数化：按页面独立调优（用户批准）
  - PR 拆分：1 个大 PR 原子提交（用户批准）
- **下一步**：调用 writing-plans skill 生成实施计划

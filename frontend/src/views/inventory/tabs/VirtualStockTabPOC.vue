<!--
  库存台账 - el-table-v2 虚拟滚动 POC
  任务编号: B5 P2-1
  目标: 验证 el-table-v2 在 1 万行库存台账数据下的渲染性能

  功能要点:
    - 1 万行数据首次渲染 < 500ms
    - 滚动 FPS >= 50
    - 内存占用 < 100MB
    - 列定义、排序、筛选与原 el-table 行为一致
-->
<template>
  <div class="virtual-stock-poc">
    <!-- 顶部:筛选 + 性能指标面板 -->
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input
            v-model="queryParams.keyword"
            placeholder="产品编码/名称"
            clearable
            style="width: 200px"
            @clear="handleQuery"
            @keyup.enter="handleQuery"
          />
        </el-form-item>
        <el-form-item label="状态">
          <el-select
            v-model="queryParams.status"
            placeholder="选择状态"
            clearable
            style="width: 160px"
            @change="handleQuery"
          >
            <el-option label="全部" value="" />
            <el-option label="正常" value="normal" />
            <el-option label="预警" value="warning" />
            <el-option label="冻结" value="frozen" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            查询
          </el-button>
          <el-button @click="handleReset">重置</el-button>
          <el-button type="success" @click="generateAndLoad">
            <el-icon><Refresh /></el-icon>
            重新生成测试数据
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- 性能指标 -->
    <el-card shadow="hover" class="metrics-card">
      <div class="metrics-title">
        <span>性能指标</span>
        <el-tag :type="metricsBadgeType" size="small">
          {{ metricsBadgeText }}
        </el-tag>
      </div>
      <el-row :gutter="16">
        <el-col :span="6">
          <div class="metric-block">
            <div class="metric-label">首次渲染耗时</div>
            <div class="metric-value">
              {{ metrics.firstRenderMs.toFixed(1) }}
              <span class="metric-unit">ms</span>
            </div>
            <div class="metric-threshold">阈值 &lt; 500ms</div>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="metric-block">
            <div class="metric-label">滚动 FPS</div>
            <div class="metric-value">
              {{ metrics.fps.toFixed(1) }}
              <span class="metric-unit">fps</span>
            </div>
            <div class="metric-threshold">阈值 ≥ 50</div>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="metric-block">
            <div class="metric-label">JS 堆内存</div>
            <div class="metric-value">
              {{ metrics.memoryMB.toFixed(1) }}
              <span class="metric-unit">MB</span>
            </div>
            <div class="metric-threshold">阈值 &lt; 100MB</div>
          </div>
        </el-col>
        <el-col :span="6">
          <div class="metric-block">
            <div class="metric-label">可见行数 / 总行数</div>
            <div class="metric-value">
              {{ metrics.visibleRowCount }}
              <span class="metric-unit">/ {{ filteredStocks.length }}</span>
            </div>
            <div class="metric-threshold">虚拟滚动比例</div>
          </div>
        </el-col>
      </el-row>
    </el-card>

    <!-- 表格区域 -->
    <el-card shadow="hover" class="table-card">
      <div
        ref="tableContainerRef"
        class="table-container"
        v-loading="loading"
        :style="{ height: tableHeight + 'px' }"
      >
        <el-auto-resizer v-if="filteredStocks.length > 0">
          <template #default="{ width }">
            <el-table-v2
              :columns="v2Columns"
              :data="filteredStocks"
              :width="width"
              :height="tableHeight"
              :row-height="rowHeight"
              :header-height="40"
              :row-key="rowKeyGetter"
              :sort-by="sortByRef"
              :estimated-row-height="rowHeight"
              :use-is-scrolling="true"
              fixed
              @column-sort="handleColumnSort"
              @scroll="handleScroll"
            />
          </template>
        </el-auto-resizer>
        <el-empty
          v-else
          description="暂无数据"
        />
      </div>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="queryParams.page"
          v-model:page-size="queryParams.page_size"
          :page-sizes="[20, 50, 100, 200, 500]"
          :total="filteredStocks.length"
          :page-size-sizes="false"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
/**
 * el-table-v2 POC
 * - 1 万行测试数据(可配置,支持 5 千、1 万、5 万)
 * - 实时性能监控(渲染耗时、FPS、内存)
 * - 列定义、排序、筛选与原 el-table 行为一致
 * - 数据源: scripts/gen-test-data.ts 中提供的 generateStocks/loadOrGenerateStocks
 */
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import {
  ElAutoResizer,
  ElTableV2,
  ElTag,
  type Column,
  type ColumnSortParams,
  type SortBy,
} from 'element-plus'
import { Search, Refresh } from '@element-plus/icons-vue'
import { h } from 'vue'
import {
  generateStocks,
  loadOrGenerateStocks,
  type StockRow,
} from './testData'

/* ========== 数据源 ========== */

const loading = ref(false)
const stocks = ref<StockRow[]>([])

// 初始生成 1 万行测试数据(可在面板中点击"重新生成测试数据"调整)
const TEST_DATA_COUNT = 10000

stocks.value = loadOrGenerateStocks(TEST_DATA_COUNT)

const queryParams = reactive({
  page: 1,
  page_size: 50,
  keyword: '',
  status: '',
})

/* ========== 表格尺寸 ========== */

// 视口高度 - 顶部筛选卡(约 80) - 性能卡(约 110) - 分页(约 60) - 余量(40)
const TABLE_HEIGHT_OFFSET = 320
const tableHeight = ref(560)
const tableContainerRef = ref<HTMLElement | null>(null)
const rowHeight = 44

const updateTableHeight = () => {
  const h = window.innerHeight - TABLE_HEIGHT_OFFSET
  tableHeight.value = Math.max(400, Math.min(900, h))
}

onMounted(() => {
  updateTableHeight()
  window.addEventListener('resize', updateTableHeight)
  // 触发一次空滚动用于初始化 FPS 监控
  startFpsMonitor()
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', updateTableHeight)
  stopFpsMonitor()
  stopMemoryMonitor()
})

/* ========== 列定义 ========== */

const v2Columns = computed<Column<StockRow>[]>(() => [
  {
    key: 'product_code',
    dataKey: 'product_code',
    title: '产品编码',
    width: 140,
    fixed: true,
    sortable: true,
  },
  {
    key: 'product_name',
    dataKey: 'product_name',
    title: '产品名称',
    width: 200,
    fixed: true,
    sortable: true,
  },
  {
    key: 'warehouse_name',
    dataKey: 'warehouse_name',
    title: '仓库',
    width: 120,
    sortable: true,
  },
  {
    key: 'batch_no',
    dataKey: 'batch_no',
    title: '批次号',
    width: 120,
    sortable: true,
  },
  {
    key: 'color_code',
    dataKey: 'color_code',
    title: '颜色编码',
    width: 100,
    sortable: true,
  },
  {
    key: 'location',
    dataKey: 'location',
    title: '库位',
    width: 100,
    sortable: true,
  },
  {
    key: 'quantity',
    dataKey: 'quantity',
    title: '库存数量',
    width: 110,
    align: 'right',
    sortable: true,
    cellRenderer: ({ cellData, rowData }) => {
      const q = Number(cellData ?? 0)
      const minQ = Number((rowData as StockRow).min_quantity ?? 0)
      const low = q < minQ
      return h('span', { class: low ? 'low-stock' : '' }, String(q))
    },
  },
  {
    key: 'unit',
    dataKey: 'unit',
    title: '单位',
    width: 70,
  },
  {
    key: 'gram_weight',
    dataKey: 'gram_weight',
    title: '克重',
    width: 80,
    align: 'right',
  },
  {
    key: 'width',
    dataKey: 'width',
    title: '门幅',
    width: 80,
    align: 'right',
  },
  {
    key: 'status',
    dataKey: 'status',
    title: '状态',
    width: 90,
    cellRenderer: ({ cellData }) => {
      const map: Record<string, { type: 'success' | 'warning' | 'info'; text: string }> = {
        normal: { type: 'success', text: '正常' },
        warning: { type: 'warning', text: '预警' },
        frozen: { type: 'info', text: '冻结' },
      }
      const key = String(cellData ?? '')
      const cfg = map[key] || { type: 'info' as const, text: key }
      return h(ElTag, { type: cfg.type, size: 'small' }, () => cfg.text)
    },
  },
  {
    key: 'actions',
    dataKey: 'id',
    title: '操作',
    width: 160,
    align: 'center',
    cellRenderer: ({ rowData }) => {
      return h('div', { class: 'action-cell' }, [
        h(
          'button',
          {
            class: 'action-link',
            onClick: () => handleView(rowData as StockRow),
          },
          '详情',
        ),
        h(
          'button',
          {
            class: 'action-link warn',
            onClick: () => handleAdjust(rowData as StockRow),
          },
          '调整',
        ),
      ])
    },
  },
])

/* ========== 排序 ========== */

const sortByRef = ref<SortBy>({ key: '' as string, order: 'asc' as SortBy['order'] })

const handleColumnSort = (params: ColumnSortParams<StockRow>) => {
  sortByRef.value = { key: params.key, order: params.order }
}

/* ========== 筛选 ========== */

const filteredStocks = computed<StockRow[]>(() => {
  const keyword = queryParams.keyword.trim().toLowerCase()
  const status = queryParams.status
  let result = stocks.value
  if (keyword) {
    result = result.filter(
      (r) =>
        r.product_code.toLowerCase().includes(keyword) ||
        r.product_name.toLowerCase().includes(keyword),
    )
  }
  if (status) {
    result = result.filter((r) => r.status === status)
  }
  // 应用排序
  if (sortByRef.value.key && sortByRef.value.order) {
    const { key, order } = sortByRef.value
    const factor = order === 'asc' ? 1 : -1
    result = [...result].sort((a, b) => {
      const av = (a as any)[key]
      const bv = (b as any)[key]
      if (av === bv) return 0
      if (av == null) return -1 * factor
      if (bv == null) return 1 * factor
      if (typeof av === 'number' && typeof bv === 'number') return (av - bv) * factor
      return String(av).localeCompare(String(bv)) * factor
    })
  }
  return result
})

/* ========== 查询/重置 ========== */

const handleQuery = () => {
  queryParams.page = 1
  // filteredStocks 为计算属性,自动重算
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.status = ''
  queryParams.page = 1
}

const handleSizeChange = (size: number) => {
  queryParams.page_size = size
  queryParams.page = 1
}

const handlePageChange = (page: number) => {
  queryParams.page = page
}

/* ========== 行操作 ========== */

const rowKeyGetter = (row: StockRow) => row.id

const handleView = (row: StockRow) => {
  // POC 阶段不实现具体业务,仅占位
  // eslint-disable-next-line no-console
  console.info('[POC] 查看详情', row.id)
}

const handleAdjust = (row: StockRow) => {
  // eslint-disable-next-line no-console
  console.info('[POC] 库存调整', row.id)
}

/* ========== 测试数据生成 ========== */

const generateAndLoad = () => {
  loading.value = true
  // 异步生成,避免阻塞 UI
  setTimeout(() => {
    stocks.value = generateStocks(TEST_DATA_COUNT)
    handleReset()
    loading.value = false
    // eslint-disable-next-line no-console
    console.info('[POC] 重新生成', stocks.value.length, '行')
  }, 0)
}

/* ========== 性能指标 ========== */

interface Metrics {
  firstRenderMs: number
  fps: number
  memoryMB: number
  visibleRowCount: number
}

const metrics = reactive<Metrics>({
  firstRenderMs: 0,
  fps: 60,
  memoryMB: 0,
  visibleRowCount: 0,
})

// 监控首次渲染
const recordFirstRender = () => {
  // 等到表格数据实际被绘制后再计算
  requestAnimationFrame(() => {
    requestAnimationFrame(() => {
      const t = performance.now()
      metrics.firstRenderMs = t
    })
  })
}

// FPS 监控 - 60fps 滑动窗口
let fpsTimer: number | null = null
let fpsLastSample = performance.now()
let fpsFrames = 0
const startFpsMonitor = () => {
  const tick = () => {
    fpsFrames += 1
    const now = performance.now()
    if (now - fpsLastSample >= 1000) {
      metrics.fps = (fpsFrames * 1000) / (now - fpsLastSample)
      fpsFrames = 0
      fpsLastSample = now
    }
    fpsTimer = requestAnimationFrame(tick)
  }
  fpsTimer = requestAnimationFrame(tick)
}
const stopFpsMonitor = () => {
  if (fpsTimer !== null) {
    cancelAnimationFrame(fpsTimer)
    fpsTimer = null
  }
}

// 内存监控
let memoryTimer: number | null = null
const startMemoryMonitor = () => {
  const perf = performance as Performance & {
    memory?: { usedJSHeapSize: number; jsHeapSizeLimit: number }
  }
  if (!perf.memory) return
  memoryTimer = window.setInterval(() => {
    if (perf.memory) {
      metrics.memoryMB = perf.memory.usedJSHeapSize / 1024 / 1024
    }
  }, 1000)
}
const stopMemoryMonitor = () => {
  if (memoryTimer !== null) {
    clearInterval(memoryTimer)
    memoryTimer = null
  }
}

startMemoryMonitor()

// 滚动事件回调:估算可见行数
const handleScroll = () => {
  const visible = Math.ceil(tableHeight.value / rowHeight)
  metrics.visibleRowCount = visible
}

/* ========== 状态徽标 ========== */

const metricsBadgeType = computed(() => {
  const renderOk = metrics.firstRenderMs === 0 || metrics.firstRenderMs < 500
  const fpsOk = metrics.fps >= 50
  const memOk = metrics.memoryMB === 0 || metrics.memoryMB < 100
  if (renderOk && fpsOk && memOk) return 'success'
  if (!renderOk || !fpsOk) return 'danger'
  return 'warning'
})

const metricsBadgeText = computed(() => {
  const renderOk = metrics.firstRenderMs === 0 || metrics.firstRenderMs < 500
  const fpsOk = metrics.fps >= 50
  const memOk = metrics.memoryMB === 0 || metrics.memoryMB < 100
  if (renderOk && fpsOk && memOk) return '全部达标'
  if (!renderOk || !fpsOk) return '未达标'
  return '部分达标'
})

/* ========== 挂载后触发首次渲染计时 ========== */

watch(
  () => stocks.value.length,
  (n) => {
    if (n > 0) {
      recordFirstRender()
    }
  },
  { immediate: true },
)
</script>

<style scoped>
.virtual-stock-poc {
  padding: 0;
}
.filter-card,
.metrics-card,
.table-card {
  margin-bottom: 16px;
}
.metrics-title {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
  font-weight: 600;
  font-size: 15px;
}
.metric-block {
  padding: 12px;
  background: #f5f7fa;
  border-radius: 6px;
  text-align: center;
}
.metric-label {
  font-size: 12px;
  color: #909399;
  margin-bottom: 6px;
}
.metric-value {
  font-size: 22px;
  font-weight: 600;
  color: #303133;
}
.metric-unit {
  font-size: 12px;
  color: #909399;
  margin-left: 4px;
  font-weight: 400;
}
.metric-threshold {
  font-size: 11px;
  color: #c0c4cc;
  margin-top: 4px;
}
.table-container {
  width: 100%;
  border: 1px solid #ebeef5;
  border-radius: 4px;
  overflow: hidden;
}
.pagination-wrapper {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
.action-cell {
  display: flex;
  gap: 8px;
  justify-content: center;
}
.action-link {
  border: none;
  background: transparent;
  color: #409eff;
  cursor: pointer;
  font-size: 12px;
  padding: 0;
}
.action-link.warn {
  color: #e6a23c;
}
:deep(.low-stock) {
  color: #f56c6c;
  font-weight: 600;
}
</style>

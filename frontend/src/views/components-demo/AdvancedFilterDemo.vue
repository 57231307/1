<template>
  <div class="advanced-filter-demo">
    <el-card>
      <template #header>高级筛选组件示例</template>

      <AdvancedFilter
        :fields="filterFields"
        :saved-schemes="savedSchemes"
        @apply="handleApply"
        @reset="handleReset"
        @scheme-saved="handleSchemeSaved"
        @scheme-loaded="handleSchemeLoaded"
        @logic-change="handleLogicChange"
      />

      <el-card v-if="filterResult" class="result-card">
        <template #header>筛选结果</template>
        <pre>{{ JSON.stringify(filterResult, null, 2) }}</pre>
      </el-card>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import AdvancedFilter, { type FilterGroup, type SavedScheme } from '@/components/AdvancedFilter.vue'

const filterFields = [
  { key: 'name', label: '订单名称', type: 'text' as const },
  {
    key: 'status',
    label: '订单状态',
    type: 'select' as const,
    options: [
      { label: '待处理', value: 'pending' },
      { label: '处理中', value: 'processing' },
      { label: '已完成', value: 'completed' },
      { label: '已取消', value: 'cancelled' },
    ],
  },
  { key: 'amount', label: '订单金额', type: 'number' as const },
  { key: 'date', label: '创建日期', type: 'date' as const },
  { key: 'customer', label: '客户名称', type: 'text' as const },
]

const savedSchemes = ref<SavedScheme[]>([
  {
    id: '1',
    name: '待处理订单',
    groups: [
      {
        logic: 'AND',
        items: [{ field: 'status', operator: 'eq', value: 'pending' }],
      },
    ],
    createdAt: '2026-01-15T10:00:00Z',
  },
  {
    id: '2',
    name: '高额订单',
    groups: [
      {
        logic: 'AND',
        items: [{ field: 'amount', operator: 'gte', value: 10000 }],
      },
    ],
    createdAt: '2026-01-16T10:00:00Z',
  },
])

const filterResult = ref<FilterGroup[] | null>(null)

const handleApply = (filters: FilterGroup[]) => {
  filterResult.value = filters
}

const handleReset = () => {
  filterResult.value = null
}

const handleSchemeSaved = (scheme: SavedScheme) => {
  savedSchemes.value.push(scheme)
}

const handleSchemeLoaded = (_scheme: SavedScheme) => {
  // 方案加载完成
}

/// 条件组逻辑切换处理（批次 253：演示 logicChange 事件的真实接入）
///
/// 父组件可在此处实现自动重新查询或更新筛选预览。
/// 当前演示：更新筛选结果以反映新的逻辑关系。
const handleLogicChange = (
  _groupIndex: number,
  _logic: 'AND' | 'OR',
  filters: FilterGroup[]
) => {
  // 自动应用筛选以反映新的逻辑关系（演示逻辑切换的实时效果）
  filterResult.value = filters
}
</script>

<style scoped>
.advanced-filter-demo {
  padding: 10px;
}

.result-card {
  margin-top: 20px;
}

.result-card pre {
  background: #f5f7fa;
  padding: 16px;
  border-radius: 4px;
  overflow-x: auto;
  font-size: 13px;
  line-height: 1.6;
}
</style>

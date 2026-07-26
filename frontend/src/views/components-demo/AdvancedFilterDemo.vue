<template>
  <div class="advanced-filter-demo">
    <el-card>
      <template #header>{{ t('componentsDemo.advancedFilter.title') }}</template>

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
        <template #header>{{ t('componentsDemo.advancedFilter.resultTitle') }}</template>
        <pre>{{ JSON.stringify(filterResult, null, 2) }}</pre>
      </el-card>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import AdvancedFilter, { type FilterGroup, type SavedScheme } from '@/components/AdvancedFilter.vue'

const { t } = useI18n({ useScope: 'global' })

const filterFields = [
  {
    key: 'name',
    label: t('componentsDemo.advancedFilter.fields.orderName'),
    type: 'text' as const,
  },
  {
    key: 'status',
    label: t('componentsDemo.advancedFilter.fields.orderStatus'),
    type: 'select' as const,
    options: [
      { label: t('componentsDemo.advancedFilter.statusPending'), value: 'pending' },
      { label: t('componentsDemo.advancedFilter.statusProcessing'), value: 'processing' },
      { label: t('componentsDemo.advancedFilter.statusCompleted'), value: 'completed' },
      { label: t('componentsDemo.advancedFilter.statusCancelled'), value: 'cancelled' },
    ],
  },
  {
    key: 'amount',
    label: t('componentsDemo.advancedFilter.fields.orderAmount'),
    type: 'number' as const,
  },
  {
    key: 'date',
    label: t('componentsDemo.advancedFilter.fields.createDate'),
    type: 'date' as const,
  },
  {
    key: 'customer',
    label: t('componentsDemo.advancedFilter.fields.customer'),
    type: 'text' as const,
  },
]

const savedSchemes = ref<SavedScheme[]>([
  {
    id: '1',
    name: t('componentsDemo.advancedFilter.schemePendingOrders'),
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
    name: t('componentsDemo.advancedFilter.schemeHighAmountOrders'),
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
const handleLogicChange = (_groupIndex: number, _logic: 'AND' | 'OR', filters: FilterGroup[]) => {
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

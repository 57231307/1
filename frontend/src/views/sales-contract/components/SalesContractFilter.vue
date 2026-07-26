<!--
  SalesContractFilter.vue - 销售合同过滤栏
  拆分自 sales-contract/index.vue（P14 批 2 I-3 第 1 批）
  批次 284：接入 useTableApi 模式（localQuery + handleSearch/handleReset，保留 dateRange/date-change）
-->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form
      :inline="true"
      :model="localQuery"
      class="filter-form"
      :aria-label="t('salesContract.filter.ariaLabel')"
    >
      <el-form-item :label="t('salesContract.filter.labelKeyword')">
        <el-input
          v-model="localQuery.keyword"
          :placeholder="t('salesContract.filter.placeholderKeyword')"
          clearable
          @clear="handleSearch"
        />
      </el-form-item>
      <el-form-item :label="t('salesContract.filter.labelCustomer')">
        <el-select
          v-model="localQuery.customer_id"
          :placeholder="t('salesContract.filter.placeholderCustomer')"
          clearable
          @change="handleSearch"
        >
          <el-option v-for="c in customers" :key="c.id" :label="c.customer_name" :value="c.id" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('salesContract.filter.labelStatus')">
        <el-select
          v-model="localQuery.status"
          :placeholder="t('salesContract.filter.placeholderStatus')"
          clearable
          @change="handleSearch"
        >
          <el-option :label="t('salesContract.filter.optionDraft')" value="draft" />
          <el-option :label="t('salesContract.filter.optionPending')" value="pending" />
          <el-option :label="t('salesContract.filter.optionActive')" value="active" />
          <el-option :label="t('salesContract.filter.optionCompleted')" value="completed" />
          <el-option :label="t('salesContract.filter.optionCancelled')" value="cancelled" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('salesContract.filter.labelSignedDate')">
        <el-date-picker
          :model-value="dateRange"
          type="daterange"
          :range-separator="t('salesContract.filter.rangeSeparator')"
          :start-placeholder="t('salesContract.filter.placeholderStartDate')"
          :end-placeholder="t('salesContract.filter.placeholderEndDate')"
          @update:model-value="(v: [Date, Date] | null) => emit('date-change', v)"
        />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="handleSearch">
          <el-icon><Search /></el-icon>
          {{ t('salesContract.filter.buttonSearch') }}
        </el-button>
        <el-button @click="handleReset">
          <el-icon><Refresh /></el-icon>
          {{ t('salesContract.filter.buttonReset') }}
        </el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { Search, Refresh } from '@element-plus/icons-vue'
import type { Customer } from '@/api/customer'

const { t } = useI18n({ useScope: 'global' })

/**
 * 销售合同过滤栏组件（批次 284：localQuery + handleSearch/handleReset 模式，保留 dateRange/date-change）
 */
const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: Record<string, unknown>
  // 客户列表
  customers: Customer[]
  // 日期范围（特殊处理：日期范围需转换为 signed_date_from/signed_date_to）
  dateRange: [Date, Date] | null
}>()

const emit = defineEmits<{
  // 触发加载
  fetch: []
  // 整体回写查询参数
  'update:queryParams': [value: Record<string, unknown>]
  // 日期变化（特殊处理：由父组件转换为 signed_date_from/signed_date_to）
  'date-change': [v: [Date, Date] | null]
}>()

// 本地查询条件（筛选字段，不含分页参数）
const localQuery = reactive<{ keyword: string; customer_id: number | undefined; status: string }>({
  keyword: (props.queryParams.keyword as string) ?? '',
  customer_id: props.queryParams.customer_id as number | undefined,
  status: (props.queryParams.status as string) ?? '',
})

/** 搜索：先同步筛选条件到父组件，再触发加载 */
const handleSearch = () => {
  emit('update:queryParams', { ...localQuery })
  emit('fetch')
}

/** 重置：清空筛选条件 + 通知父组件清空日期范围 + 同步 + 触发加载 */
const handleReset = () => {
  localQuery.keyword = ''
  localQuery.customer_id = undefined
  localQuery.status = ''
  emit('update:queryParams', { ...localQuery })
  // 日期范围特殊处理：重置时通知父组件清空 dateRange
  emit('date-change', null)
  emit('fetch')
}
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}
.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
</style>

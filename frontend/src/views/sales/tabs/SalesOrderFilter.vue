<!--
  SalesOrderFilter.vue - 销售订单筛选组件
  来源：原 sales/index.vue 中 筛选表单区
  拆分日期：2026-06-17 P1-3-Batch-1
  说明：本文件由 sales/index.vue 拆分而来，作为纯展示组件，逻辑完整可独立运行
-->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="localQuery" class="filter-form">
      <el-form-item label="关键词">
        <el-input
          v-model="localQuery.keyword"
          placeholder="订单号/客户名"
          clearable
          @clear="emit('query')"
        />
      </el-form-item>
      <el-form-item label="客户">
        <el-select
          v-model="localQuery.customer_id"
          placeholder="选择客户"
          clearable
          @change="emit('query')"
        >
          <el-option
            v-for="c in customers"
            :key="c.id"
            :label="c.customer_name"
            :value="c.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item label="订单状态">
        <el-select
          v-model="localQuery.status"
          placeholder="选择状态"
          clearable
          @change="emit('query')"
        >
          <el-option label="待审批" value="pending" />
          <el-option label="已审批" value="approved" />
          <el-option label="已发货" value="shipped" />
          <el-option label="已完成" value="completed" />
          <el-option label="已取消" value="cancelled" />
        </el-select>
      </el-form-item>
      <el-form-item label="日期范围">
        <el-date-picker
          v-model="dateRange"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          @change="handleDateChange"
        />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="emit('query')">
          <el-icon><Search /></el-icon>
          查询
        </el-button>
        <el-button @click="handleReset">
          <el-icon><Refresh /></el-icon>
          重置
        </el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { Search, Refresh } from '@element-plus/icons-vue'
import type { Customer } from '@/api/customer'

// 筛选条件接口
export interface SalesOrderQuery {
  page: number
  page_size: number
  keyword: string
  customer_id: number | undefined
  status: string
  order_date_from: string
  order_date_to: string
}

const props = defineProps<{
  queryParams: SalesOrderQuery
  customers: Customer[]
}>()

const emit = defineEmits<{
  query: []
  reset: []
  'update:dateRange': [value: string[]]
}>()

// 本地副本，避免直接修改 prop
const localQuery = reactive<SalesOrderQuery>({ ...props.queryParams })
const dateRange = ref<[Date, Date] | null>(null)

watch(
  () => props.queryParams,
  newParams => {
    Object.assign(localQuery, newParams)
  },
  { deep: true }
)

const handleDateChange = (val: [Date, Date] | null) => {
  if (val) {
    localQuery.order_date_from = val[0].toISOString().split('T')[0]
    localQuery.order_date_to = val[1].toISOString().split('T')[0]
  } else {
    localQuery.order_date_from = ''
    localQuery.order_date_to = ''
  }
  emit('query')
}

const handleReset = () => {
  localQuery.keyword = ''
  localQuery.customer_id = undefined
  localQuery.status = ''
  dateRange.value = null
  localQuery.order_date_from = ''
  localQuery.order_date_to = ''
  emit('reset')
}
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}

.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}
</style>

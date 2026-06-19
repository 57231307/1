<!--
  PurchaseOrderFilter.vue - 采购订单筛选组件
  来源：原 purchase/index.vue 中 筛选表单
  拆分日期：2026-06-17 P1-3-Batch-2
-->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="localQuery" class="filter-form">
      <el-form-item label="关键词">
        <el-input
          v-model="localQuery.keyword"
          placeholder="订单号/供应商名"
          clearable
          @clear="emit('query')"
        />
      </el-form-item>
      <el-form-item label="供应商">
        <el-select
          v-model="localQuery.supplier_id"
          placeholder="选择供应商"
          clearable
          @change="emit('query')"
        >
          <el-option
            v-for="s in suppliers"
            :key="s.id"
            :label="s.supplier_name"
            :value="s.id"
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
          <el-option label="部分收货" value="partial" />
          <el-option label="已完成" value="completed" />
          <el-option label="已取消" value="cancelled" />
        </el-select>
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
import { reactive, watch } from 'vue'
import { Search, Refresh } from '@element-plus/icons-vue'
import type { Supplier } from '@/api/supplier'

export interface PurchaseQuery {
  page: number
  page_size: number
  keyword: string
  supplier_id: number | undefined
  status: string
}

const props = defineProps<{
  queryParams: PurchaseQuery
  suppliers: Supplier[]
}>()

const emit = defineEmits<{
  query: []
  reset: []
}>()

const localQuery = reactive<PurchaseQuery>({ ...props.queryParams })

watch(
  () => props.queryParams,
  newParams => {
    Object.assign(localQuery, newParams)
  },
  { deep: true }
)

const handleReset = () => {
  localQuery.keyword = ''
  localQuery.supplier_id = undefined
  localQuery.status = ''
  emit('reset')
}
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}
</style>

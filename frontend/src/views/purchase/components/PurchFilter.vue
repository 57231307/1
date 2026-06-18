<script setup lang="ts">
/**
 * PurchFilter - 采购管理筛选表单
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 筛选表单）
 */
import { Search, Refresh } from '@element-plus/icons-vue'
import type { Supplier } from '@/api/supplier'

interface QueryParams {
  page: number
  page_size: number
  keyword: string
  supplier_id: number | undefined
  status: string
}

interface Props {
  queryParams: QueryParams
  suppliers: Supplier[]
  onQuery: () => void
  onReset: () => void
}

defineProps<Props>()
</script>

<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="queryParams" class="filter-form">
      <el-form-item label="关键词">
        <el-input
          v-model="queryParams.keyword"
          placeholder="订单号/供应商名"
          clearable
          @clear="onQuery"
        />
      </el-form-item>
      <el-form-item label="供应商">
        <el-select
          v-model="queryParams.supplier_id"
          placeholder="选择供应商"
          clearable
          @change="onQuery"
        >
          <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
        </el-select>
      </el-form-item>
      <el-form-item label="订单状态">
        <el-select
          v-model="queryParams.status"
          placeholder="选择状态"
          clearable
          @change="onQuery"
        >
          <el-option label="待审批" value="pending" />
          <el-option label="已审批" value="approved" />
          <el-option label="部分收货" value="partial" />
          <el-option label="已完成" value="completed" />
          <el-option label="已取消" value="cancelled" />
        </el-select>
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="onQuery">
          <el-icon><Search /></el-icon>
          查询
        </el-button>
        <el-button @click="onReset">
          <el-icon><Refresh /></el-icon>
          重置
        </el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

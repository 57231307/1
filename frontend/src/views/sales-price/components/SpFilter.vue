<!--
  SpFilter.vue - 销售价格过滤栏
  拆分自 sales-price/index.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="queryParams" class="filter-form">
      <el-form-item label="关键词">
        <el-input
          :model-value="queryParams.keyword"
          placeholder="产品名称/客户名称"
          clearable
          @update:model-value="(v: string) => (queryParams.keyword = v)"
          @clear="emit('query')"
        />
      </el-form-item>
      <el-form-item label="客户">
        <el-select
          :model-value="queryParams.customer_id"
          placeholder="选择客户"
          clearable
          filterable
          @update:model-value="(v: number) => (queryParams.customer_id = v)"
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
      <el-form-item label="产品">
        <el-select
          :model-value="queryParams.product_id"
          placeholder="选择产品"
          clearable
          filterable
          @update:model-value="(v: number) => (queryParams.product_id = v)"
          @change="emit('query')"
        >
          <el-option
            v-for="p in products"
            :key="p.id"
            :label="p.product_name"
            :value="p.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item label="价格状态">
        <el-select
          :model-value="queryParams.status"
          placeholder="选择状态"
          clearable
          @update:model-value="(v: string) => (queryParams.status = v)"
          @change="emit('query')"
        >
          <el-option label="待审批" value="pending" />
          <el-option label="已生效" value="active" />
          <el-option label="已过期" value="expired" />
          <el-option label="已停用" value="inactive" />
        </el-select>
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="emit('query')">
          <el-icon><Search /></el-icon>
          查询
        </el-button>
        <el-button @click="emit('reset')">
          <el-icon><Refresh /></el-icon>
          重置
        </el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { Search, Refresh } from '@element-plus/icons-vue'
import type { Customer } from '@/api/customer'
import type { Product } from '@/api/product'

// 销售价格查询参数类型
interface SpQueryParams {
  keyword: string
  customer_id: number | undefined
  product_id: number | undefined
  status: string
  page: number
  page_size: number
}

/**
 * 销售价格过滤栏组件
 */
defineProps<{
  // 查询参数
  queryParams: SpQueryParams
  // 客户列表
  customers: Customer[]
  // 产品列表
  products: Product[]
}>()

const emit = defineEmits<{
  // 查询
  query: []
  // 重置
  reset: []
}>()
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

<!--
  PpFilter.vue - 采购价格过滤栏
  拆分自 purchase-price/index.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="queryParams" class="filter-form">
      <el-form-item label="关键词">
        <el-input
          :model-value="queryParams.keyword"
          placeholder="产品名称/供应商名称"
          clearable
          @update:model-value="(v: string) => (queryParams.keyword = v)"
          @clear="emit('query')"
        />
      </el-form-item>
      <el-form-item label="供应商">
        <el-select
          :model-value="queryParams.supplier_id"
          placeholder="选择供应商"
          clearable
          @update:model-value="(v: number) => (queryParams.supplier_id = v)"
          @change="emit('query')"
        >
          <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
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
          <el-option label="已生效" value="active" />
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
import type { Supplier } from '@/api/supplier'
import type { Product } from '@/api/product'

// 采购价格查询参数类型
interface PpQueryParams {
  keyword: string
  supplier_id: number | undefined
  product_id: number | undefined
  status: string
  page: number
  page_size: number
}

/**
 * 采购价格过滤栏组件
 */
defineProps<{
  // 查询参数
  queryParams: PpQueryParams
  // 供应商列表
  suppliers: Supplier[]
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

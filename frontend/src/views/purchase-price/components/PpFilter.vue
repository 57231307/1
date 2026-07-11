<!--
  PpFilter.vue - 采购价格过滤栏
  拆分自 purchase-price/index.vue（P14 批 2 I-3 第 3 批）
  批次 285：接入 useTableApi 模式（localQuery + handleSearch/handleReset）
-->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="localQuery" class="filter-form">
      <el-form-item label="关键词">
        <el-input
          v-model="localQuery.keyword"
          placeholder="产品名称/供应商名称"
          clearable
          @clear="handleSearch"
        />
      </el-form-item>
      <el-form-item label="供应商">
        <el-select
          v-model="localQuery.supplier_id"
          placeholder="选择供应商"
          clearable
          @change="handleSearch"
        >
          <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
        </el-select>
      </el-form-item>
      <el-form-item label="产品">
        <el-select
          v-model="localQuery.product_id"
          placeholder="选择产品"
          clearable
          filterable
          @change="handleSearch"
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
          v-model="localQuery.status"
          placeholder="选择状态"
          clearable
          @change="handleSearch"
        >
          <el-option label="已生效" value="active" />
          <el-option label="已停用" value="inactive" />
        </el-select>
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="handleSearch">
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
import { reactive } from 'vue'
import { Search, Refresh } from '@element-plus/icons-vue'
import type { Supplier } from '@/api/supplier'
import type { Product } from '@/api/product'

/**
 * 采购价格过滤栏组件（批次 285：localQuery + handleSearch/handleReset 模式）
 */
const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: Record<string, unknown>
  // 供应商列表
  suppliers: Supplier[]
  // 产品列表
  products: Product[]
}>()

const emit = defineEmits<{
  // 触发加载
  fetch: []
  // 整体回写查询参数
  'update:queryParams': [value: Record<string, unknown>]
}>()

// 本地查询条件（筛选字段，不含分页参数）
const localQuery = reactive<{
  keyword: string
  supplier_id: number | undefined
  product_id: number | undefined
  status: string
}>({
  keyword: (props.queryParams.keyword as string) ?? '',
  supplier_id: props.queryParams.supplier_id as number | undefined,
  product_id: props.queryParams.product_id as number | undefined,
  status: (props.queryParams.status as string) ?? '',
})

/** 搜索：先同步筛选条件到父组件，再触发加载 */
const handleSearch = () => {
  emit('update:queryParams', { ...localQuery })
  emit('fetch')
}

/** 重置：清空筛选条件 + 同步 + 触发加载 */
const handleReset = () => {
  localQuery.keyword = ''
  localQuery.supplier_id = undefined
  localQuery.product_id = undefined
  localQuery.status = ''
  emit('update:queryParams', { ...localQuery })
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

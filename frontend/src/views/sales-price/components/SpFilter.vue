<!--
  SpFilter.vue - 销售价格过滤栏
  拆分自 sales-price/index.vue（P14 批 2 I-3 第 3 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="localQueryParams" class="filter-form">
      <el-form-item label="关键词">
        <el-input
          v-model="localQueryParams.keyword"
          placeholder="产品名称/客户名称"
          clearable
          @clear="emit('query')"
        />
      </el-form-item>
      <el-form-item label="客户">
        <el-select
          v-model="localQueryParams.customer_id"
          placeholder="选择客户"
          clearable
          filterable
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
          v-model="localQueryParams.product_id"
          placeholder="选择产品"
          clearable
          filterable
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
        <el-select v-model="localQueryParams.status" placeholder="选择状态" clearable @change="emit('query')">
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
import { ref, watch, nextTick } from 'vue'
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
const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: SpQueryParams
  // 客户列表
  customers: Customer[]
  // 产品列表
  products: Product[]
}>()

const emit = defineEmits<{
  // 查询
  (e: 'query'): void
  // 重置
  (e: 'reset'): void
  // 整体回写查询参数（父组件监听此事件并 Object.assign 到自己的 queryParams）
  (e: 'update:queryParams', queryParams: SpQueryParams): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localQueryParams = ref<SpQueryParams>({ ...props.queryParams })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件重置）
watch(
  () => props.queryParams,
  (newParams) => {
    if (syncing) return
    syncing = true
    localQueryParams.value = { ...newParams }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localQueryParams,
  (newParams) => {
    if (syncing) return
    syncing = true
    emit('update:queryParams', { ...newParams })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)
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

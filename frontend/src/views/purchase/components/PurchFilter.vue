<!--
  PurchFilter - 采购管理筛选表单
  任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 筛选表单）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
-->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="localQueryParams" class="filter-form">
      <el-form-item label="关键词">
        <el-input
          v-model="localQueryParams.keyword"
          placeholder="订单号/供应商名"
          clearable
          @clear="onQuery"
        />
      </el-form-item>
      <el-form-item label="供应商">
        <el-select
          v-model="localQueryParams.supplier_id"
          placeholder="选择供应商"
          clearable
          @change="onQuery"
        >
          <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
        </el-select>
      </el-form-item>
      <el-form-item label="订单状态">
        <el-select
          v-model="localQueryParams.status"
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

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { Search, Refresh } from '@element-plus/icons-vue'
import type { Supplier } from '@/api/supplier'

interface QueryParams {
  page: number
  page_size: number
  keyword: string
  supplier_id: number | undefined
  status: string
}

const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: QueryParams
  // 供应商列表
  suppliers: Supplier[]
  // 查询回调
  onQuery: () => void
  // 重置回调
  onReset: () => void
}>()

const emit = defineEmits<{
  // 整体回写查询参数（父组件监听此事件并 Object.assign 到自己的 queryParams）
  (e: 'update:queryParams', queryParams: QueryParams): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localQueryParams = ref<QueryParams>({ ...props.queryParams })

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

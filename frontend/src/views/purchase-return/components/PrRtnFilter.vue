<!--
  PrRtnFilter.vue - 采购退货过滤栏
  任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
-->
<template>
  <el-card class="filter-card">
    <el-form :inline="true" :model="localQueryParams">
      <el-form-item label="退货单号">
        <el-input
          v-model="localQueryParams.keyword"
          placeholder="请输入退货单号"
          clearable
          @keyup.enter="emit('query')"
        />
      </el-form-item>
      <el-form-item label="供应商">
        <el-select
          v-model="localQueryParams.supplierId"
          placeholder="选择供应商"
          clearable
          filterable
        >
          <el-option
            v-for="supplier in suppliers"
            :key="supplier.id"
            :label="supplier.name"
            :value="supplier.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item label="状态">
        <el-select v-model="localQueryParams.status" placeholder="选择状态" clearable>
          <el-option label="草稿" value="draft" />
          <el-option label="待审批" value="pending" />
          <el-option label="已审批" value="approved" />
          <el-option label="已拒绝" value="rejected" />
          <el-option label="已完成" value="completed" />
        </el-select>
      </el-form-item>
      <el-form-item label="退货日期">
        <el-date-picker
          :model-value="dateRange"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          @update:model-value="onDateChange"
        />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="emit('query')">查询</el-button>
        <el-button @click="emit('reset')">重置</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'

// 过滤栏查询参数
interface QueryParams {
  page: number
  pageSize: number
  keyword: string
  supplierId: number | undefined
  status: string
}

// 供应商数据结构
interface Supplier {
  id: number
  name: string
}

const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: QueryParams
  // 供应商列表
  suppliers: Supplier[]
  // 日期范围
  dateRange: [Date, Date] | null
}>()

// 定义事件
const emit = defineEmits<{
  // 查询事件
  (e: 'query'): void
  // 重置事件
  (e: 'reset'): void
  // 日期变化事件
  (e: 'date-change', value: [Date, Date] | null): void
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

/** 日期范围变化 */
const onDateChange = (v: [Date, Date] | null) => {
  emit('date-change', v)
}
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}
</style>

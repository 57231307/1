<!--
  ScFilter.vue - 销售合同过滤栏
  拆分自 sales-contract/index.vue（P14 批 2 I-3 第 1 批）
  行为完全保持一致（仅结构重构）
  P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
-->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="localQueryParams" class="filter-form">
      <el-form-item label="关键词">
        <el-input
          v-model="localQueryParams.keyword"
          placeholder="合同编号/合同名称"
          clearable
          @clear="emit('query')"
        />
      </el-form-item>
      <el-form-item label="客户">
        <el-select
          v-model="localQueryParams.customer_id"
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
      <el-form-item label="合同状态">
        <el-select
          v-model="localQueryParams.status"
          placeholder="选择状态"
          clearable
          @change="emit('query')"
        >
          <el-option label="草稿" value="draft" />
          <el-option label="待审批" value="pending" />
          <el-option label="执行中" value="active" />
          <el-option label="已完成" value="completed" />
          <el-option label="已取消" value="cancelled" />
        </el-select>
      </el-form-item>
      <el-form-item label="签订日期">
        <el-date-picker
          :model-value="dateRange"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          @update:model-value="(v: [Date, Date] | null) => emit('date-change', v)"
        />
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

interface ScQueryParams {
  keyword: string
  customer_id: number | undefined
  status: string
  signed_date_from: string
  signed_date_to: string
  page: number
  page_size: number
}

/**
 * 销售合同过滤栏组件
 */
const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: ScQueryParams
  // 客户列表
  customers: Customer[]
  // 日期范围
  dateRange: [Date, Date] | null
}>()

const emit = defineEmits<{
  // 查询
  query: []
  // 重置
  reset: []
  // 日期变化
  'date-change': [v: [Date, Date] | null]
  // 整体回写查询参数
  'update:queryParams': [queryParams: ScQueryParams]
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localQueryParams = ref<ScQueryParams>({ ...props.queryParams })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local
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

// 本地变化时通知父组件
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

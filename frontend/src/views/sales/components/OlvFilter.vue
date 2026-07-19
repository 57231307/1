<!--
  OlvFilter.vue - 销售订单列表过滤栏
  拆分自 sales/views/OrderListView.vue（P14 批 2 I-3 第 3 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="localFilterForm" aria-label="销售订单筛选表单">
      <el-form-item label="订单号">
        <el-input
          v-model="localFilterForm.order_no"
          placeholder="订单号"
          clearable
        />
      </el-form-item>
      <el-form-item label="客户">
        <el-input
          v-model="localFilterForm.customer_name"
          placeholder="客户名称"
          clearable
        />
      </el-form-item>
      <el-form-item label="状态">
        <el-select v-model="localFilterForm.status" placeholder="选择状态" clearable>
          <el-option label="待审批" value="pending" />
          <el-option label="已审批" value="approved" />
          <el-option label="已发货" value="shipped" />
          <el-option label="已完成" value="completed" />
          <el-option label="已取消" value="cancelled" />
        </el-select>
      </el-form-item>
      <el-form-item label="日期">
        <el-date-picker
          v-model="localFilterForm.dateRange"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
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

// 销售订单过滤表单类型
interface OlvFilterForm {
  order_no: string
  customer_name: string
  status: string
  dateRange: Date[] | null
}

/**
 * 销售订单列表过滤栏组件
 */
const props = defineProps<{
  // 过滤表单（由父组件管理，子组件通过 emit('update:filterForm') 回写）
  filterForm: OlvFilterForm
}>()

const emit = defineEmits<{
  // 查询
  (e: 'query'): void
  // 重置
  (e: 'reset'): void
  // 整体回写过滤表单（父组件监听此事件并 Object.assign 到自己的 filterForm）
  (e: 'update:filterForm', filterForm: OlvFilterForm): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localFilterForm = ref<OlvFilterForm>({
  ...props.filterForm,
  dateRange: props.filterForm.dateRange ? [...props.filterForm.dateRange] : null,
})

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件重置）
watch(
  () => props.filterForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    localFilterForm.value = {
      ...newForm,
      dateRange: newForm.dateRange ? [...newForm.dateRange] : null,
    }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localFilterForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    emit('update:filterForm', {
      ...newForm,
      dateRange: newForm.dateRange ? [...newForm.dateRange] : null,
    })
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
</style>

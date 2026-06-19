<!-- eslint-disable vue/no-mutating-props -->
<!--
  PrRtnFilter.vue - 采购退货过滤栏
  任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
-->
<template>
  <el-card class="filter-card">
    <el-form :inline="true" :model="queryParams">
      <el-form-item label="退货单号">
        <el-input
          v-model="queryParams.keyword"
          placeholder="请输入退货单号"
          clearable
          @keyup.enter="emit('query')"
        />
      </el-form-item>
      <el-form-item label="供应商">
        <el-select
          v-model="queryParams.supplierId"
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
        <el-select v-model="queryParams.status" placeholder="选择状态" clearable>
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

// 采购退货过滤栏属性
defineProps<{
  // 查询参数
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
}>()

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

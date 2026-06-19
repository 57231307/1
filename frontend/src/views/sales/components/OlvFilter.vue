<!--
  OlvFilter.vue - 销售订单列表过滤栏
  拆分自 sales/views/OrderListView.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="filterForm">
      <el-form-item label="订单号">
        <el-input
          :model-value="filterForm.order_no"
          placeholder="订单号"
          clearable
          @update:model-value="(v: string) => (filterForm.order_no = v)"
        />
      </el-form-item>
      <el-form-item label="客户">
        <el-input
          :model-value="filterForm.customer_name"
          placeholder="客户名称"
          clearable
          @update:model-value="(v: string) => (filterForm.customer_name = v)"
        />
      </el-form-item>
      <el-form-item label="状态">
        <el-select
          :model-value="filterForm.status"
          placeholder="选择状态"
          clearable
          @update:model-value="(v: string) => (filterForm.status = v)"
        >
          <el-option label="待审批" value="pending" />
          <el-option label="已审批" value="approved" />
          <el-option label="已发货" value="shipped" />
          <el-option label="已完成" value="completed" />
          <el-option label="已取消" value="cancelled" />
        </el-select>
      </el-form-item>
      <el-form-item label="日期">
        <el-date-picker
          :model-value="filterForm.dateRange"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          @update:model-value="(v: Date[]) => (filterForm.dateRange = v)"
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
/* eslint-disable vue/no-mutating-props */

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
defineProps<{
  // 过滤表单
  filterForm: OlvFilterForm
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
</style>

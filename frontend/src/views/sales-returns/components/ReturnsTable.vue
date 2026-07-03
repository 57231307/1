<!--
  ReturnsTable.vue - 销售退货列表表格
  任务编号: P14 批 2 I-3 第 7 批
  拆分原 sales-returns/index.vue 的列表表格部分
-->
<template>
  <el-table v-loading="loading" :data="list" border>
    <el-table-column prop="returnNo" label="退货单号" />
    <el-table-column prop="salesOrderNo" label="销售订单号" />
    <el-table-column prop="customerName" label="客户名称" />
    <el-table-column prop="returnDate" label="退货日期" />
    <el-table-column prop="totalAmount" label="退货金额" />
    <el-table-column prop="status" label="状态">
      <template #default="{ row }">
        <el-tag :type="getStatusType(row.status)">
          {{ getStatusLabel(row.status) }}
        </el-tag>
      </template>
    </el-table-column>
    <el-table-column label="操作" width="250">
      <template #default="{ row }">
        <el-button size="small" @click="emit('view', row)">详情</el-button>
        <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
        <el-button v-permission="'sales_return:update'" size="small" @click="emit('edit', row)"
          >编辑</el-button
        >
        <el-button
          v-if="row.status === 'PENDING'"
          size="small"
          type="primary"
          @click="emit('approve', row)"
          >审核</el-button
        >
      </template>
    </el-table-column>
  </el-table>
</template>

<script setup lang="ts">
import { getStatusType, getStatusLabel } from '../composables/srFmts'

defineProps<{
  list: any[]
  loading: boolean
}>()

const emit = defineEmits<{
  (e: 'view', row: any): void
  (e: 'edit', row: any): void
  (e: 'approve', row: any): void
}>()
</script>

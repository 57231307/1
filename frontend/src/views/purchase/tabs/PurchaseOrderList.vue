<!--
  PurchaseOrderList.vue - 采购订单列表组件（V2Table 版）
  来源：原 purchase/index.vue 中 订单列表区
  拆分日期：2026-06-17 P1-3-Batch-2
-->
<template>
  <el-card shadow="hover" class="table-card">
    <V2Table
      :data="orders"
      :columns="orderColumns"
      :loading="loading"
      :total="total"
      :page="queryParams.page"
      :page-size="queryParams.page_size"
      :estimated-row-height="56"
      @row-click="(row: any) => emit('view', row)"
      @page-change="handlePageChange"
      @size-change="handleSizeChange"
    />
  </el-card>
</template>

<script setup lang="ts">
import { computed, h } from 'vue'
import { ElTag, ElButton } from 'element-plus'
import V2Table from '@/components/V2Table/index.vue'
import { useTableColumns } from '@/composables/useTableColumns'
import type { PurchaseOrder } from '@/api/purchase'

interface PurchaseQuery {
  page: number
  page_size: number
  keyword: string
  supplier_id: number | undefined
  status: string
}

const props = defineProps<{
  orders: PurchaseOrder[]
  total: number
  loading: boolean
  queryParams: PurchaseQuery
}>()

const emit = defineEmits<{
  view: [row: PurchaseOrder]
  approve: [row: PurchaseOrder]
  receive: [row: PurchaseOrder]
  query: []
  'update:queryParams': [value: PurchaseQuery]
}>()

const getStatusType = (status: string) => {
  const typeMap: Record<string, string> = {
    pending: 'warning',
    approved: 'primary',
    partial: 'info',
    completed: 'success',
    cancelled: 'danger',
  }
  return typeMap[status] || 'info'
}

const getStatusText = (status: string) => {
  const textMap: Record<string, string> = {
    pending: '待审批',
    approved: '已审批',
    partial: '部分收货',
    completed: '已完成',
    cancelled: '已取消',
  }
  return textMap[status] || status
}

const { columns: orderColumns } = useTableColumns<PurchaseOrder>([
  { key: 'order_no', title: '订单号', width: 160, sortable: true },
  { key: 'supplier_name', title: '供应商', width: 180 },
  { key: 'order_date', title: '订单日期', width: 120 },
  { key: 'required_date', title: '要求交货日期', width: 120 },
  {
    key: 'total_amount',
    title: '订单金额',
    width: 120,
    align: 'right',
    formatter: (row: PurchaseOrder) =>
      row.total_amount != null ? `¥${row.total_amount.toLocaleString()}` : '-',
  },
  {
    key: 'received_amount',
    title: '已收货金额',
    width: 120,
    align: 'right',
    formatter: (row: PurchaseOrder) =>
      `¥${(row.received_amount || 0).toLocaleString()}`,
  },
  {
    key: 'status',
    title: '订单状态',
    width: 100,
    align: 'center',
    formatter: (row: PurchaseOrder) => getStatusText(row.status),
  },
  { key: 'creator_name', title: '创建人', width: 100 },
])

const handlePageChange = (newPage: number) => {
  emit('update:queryParams', { ...props.queryParams, page: newPage })
  emit('query')
}

const handleSizeChange = (newSize: number) => {
  emit('update:queryParams', { ...props.queryParams, page_size: newSize, page: 1 })
  emit('query')
}
</script>

<style scoped>
.table-card {
  margin-bottom: 20px;
}
</style>

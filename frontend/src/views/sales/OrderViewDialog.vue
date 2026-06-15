<!--
  OrderViewDialog.vue - 销售订单详情对话框
  来源：原 sales/index.vue 中 订单详情 dialog
  拆分日期：2026-06-15 B3-1
-->
<template>
  <el-dialog
    :model-value="visible"
    title="订单详情"
    width="1000px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item label="订单号">{{ order?.order_no }}</el-descriptions-item>
      <el-descriptions-item label="订单状态">
        <el-tag :type="getStatusType(order?.status)" size="small">
          {{ getStatusText(order?.status) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item label="客户名称">{{ order?.customer_name }}</el-descriptions-item>
      <el-descriptions-item label="订单日期">{{ order?.order_date }}</el-descriptions-item>
      <el-descriptions-item label="要求交货日期">{{ order?.required_date }}</el-descriptions-item>
      <el-descriptions-item label="联系人">{{ order?.contact_person }}</el-descriptions-item>
      <el-descriptions-item label="联系电话">{{ order?.contact_phone }}</el-descriptions-item>
      <el-descriptions-item label="收货地址" :span="2">{{
        order?.delivery_address
      }}</el-descriptions-item>
      <el-descriptions-item label="订单金额">
        ¥{{ order?.total_amount?.toLocaleString() }}
      </el-descriptions-item>
      <el-descriptions-item label="创建人">{{ order?.creator_name }}</el-descriptions-item>
    </el-descriptions>

    <el-divider content-position="left">订单明细</el-divider>
    <el-table :data="order?.items" border>
      <el-table-column prop="product_name" label="产品名称" />
      <el-table-column prop="product_code" label="产品编码" width="120" />
      <el-table-column prop="quantity" label="数量" width="80" align="right" />
      <el-table-column prop="unit" label="单位" width="60" />
      <el-table-column prop="unit_price" label="单价" width="100" align="right">
        <template #default="{ row }">¥{{ row.unit_price.toLocaleString() }}</template>
      </el-table-column>
      <el-table-column prop="subtotal" label="小计" width="120" align="right">
        <template #default="{ row }">
          <strong>¥{{ row.subtotal.toLocaleString() }}</strong>
        </template>
      </el-table-column>
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import type { SalesOrder } from '@/api/sales'

defineProps<{
  visible: boolean
  order: SalesOrder | null
}>()

const emit = defineEmits<{
  'update:visible': [value: boolean]
}>()

const getStatusType = (status: string | undefined) => {
  const typeMap: Record<string, string> = {
    pending: 'warning',
    approved: 'primary',
    shipped: 'success',
    completed: 'info',
    cancelled: 'danger',
  }
  return typeMap[status || ''] || 'info'
}

const getStatusText = (status: string | undefined) => {
  const textMap: Record<string, string> = {
    pending: '待审批',
    approved: '已审批',
    shipped: '已发货',
    completed: '已完成',
    cancelled: '已取消',
  }
  return textMap[status || ''] || status || ''
}
</script>

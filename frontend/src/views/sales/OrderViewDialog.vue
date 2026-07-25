<!--
  OrderViewDialog.vue - 销售订单详情对话框
  来源：原 sales/index.vue 中 订单详情 dialog
  拆分日期：2026-06-15 B3-1
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('sales.orderView.title')"
    width="1000px"
    :aria-label="t('sales.orderView.dialogAriaLabel')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item :label="t('sales.orderView.orderNo')">{{ order?.order_no }}</el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.orderStatus')">
        <el-tag :type="getStatusType(order?.status)" size="small">
          {{ getStatusText(order?.status) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.customerName')">{{ order?.customer_name }}</el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.orderDate')">{{ order?.order_date }}</el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.requiredDate')">{{ order?.required_date }}</el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.contactPerson')">{{ order?.contact_person }}</el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.contactPhone')">{{ order?.contact_phone }}</el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.deliveryAddress')" :span="2">{{
        order?.delivery_address
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.orderAmount')">
        ¥{{ order?.total_amount?.toLocaleString() }}
      </el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.creatorName')">{{ order?.creator_name }}</el-descriptions-item>
    </el-descriptions>

    <el-divider content-position="left">{{ t('sales.orderView.orderItems') }}</el-divider>
    <el-table :data="order?.items" border :aria-label="t('sales.orderView.itemsTableAriaLabel')">
      <el-table-column prop="product_name" :label="t('sales.orderView.productName')" />
      <el-table-column prop="product_code" :label="t('sales.orderView.productCode')" width="120" />
      <el-table-column prop="quantity" :label="t('sales.orderView.quantity')" width="80" align="right" />
      <el-table-column prop="unit" :label="t('sales.orderView.unit')" width="60" />
      <el-table-column prop="unit_price" :label="t('sales.orderView.unitPrice')" width="100" align="right">
        <template #default="{ row }">¥{{ row.unit_price.toLocaleString() }}</template>
      </el-table-column>
      <el-table-column prop="subtotal" :label="t('sales.orderView.subtotal')" width="120" align="right">
        <template #default="{ row }">
          <strong>¥{{ row.subtotal.toLocaleString() }}</strong>
        </template>
      </el-table-column>
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { SalesOrder } from '@/api/sales'

const { t } = useI18n({ useScope: 'global' })

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
    pending: t('sales.statusLabels.pending'),
    approved: t('sales.statusLabels.approved'),
    shipped: t('sales.statusLabels.shipped'),
    completed: t('sales.statusLabels.completed'),
    cancelled: t('sales.statusLabels.cancelled'),
  }
  return textMap[status || ''] || status || ''
}
</script>

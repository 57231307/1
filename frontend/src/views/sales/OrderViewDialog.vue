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
      <el-descriptions-item :label="t('sales.orderView.orderNo')">{{
        order?.order_no
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.orderStatus')">
        <el-tag :type="getStatusType(order?.status)" size="small">
          {{ getStatusText(order?.status) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.customerName')">{{
        order?.customer_name
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.orderDate')">{{
        order?.order_date
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.requiredDate')">{{
        order?.required_date
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.contactPerson')">{{
        order?.contact_person
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.contactPhone')">{{
        order?.contact_phone
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.deliveryAddress')" :span="2">{{
        order?.shipping_address
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.orderAmount')">
        ¥{{ order?.total_amount?.toLocaleString() }}
      </el-descriptions-item>
      <el-descriptions-item :label="t('sales.orderView.creatorName')">{{
        order?.creator_name
      }}</el-descriptions-item>
    </el-descriptions>

    <el-divider content-position="left">{{ t('sales.orderView.orderItems') }}</el-divider>
    <el-table :data="order?.items" border :aria-label="t('sales.orderView.itemsTableAriaLabel')">
      <el-table-column prop="product_name" :label="t('sales.orderView.productName')" />
      <el-table-column prop="product_code" :label="t('sales.orderView.productCode')" width="120" />
      <el-table-column :label="t('sales.orderView.colorNo')" width="120">
        <template #default="{ row }">{{ row.color_no || t('sales.orderView.greige') }}</template>
      </el-table-column>
      <el-table-column
        prop="quantity"
        :label="t('sales.orderView.quantity')"
        width="80"
        align="right"
      />
      <el-table-column prop="unit" :label="t('sales.orderView.unit')" width="60" />
      <el-table-column
        prop="unit_price"
        :label="t('sales.orderView.unitPrice')"
        width="100"
        align="right"
      >
        <template #default="{ row }">¥{{ row.unit_price.toLocaleString() }}</template>
      </el-table-column>
      <el-table-column
        prop="subtotal"
        :label="t('sales.orderView.subtotal')"
        width="120"
        align="right"
      >
        <template #default="{ row }">
          <strong>¥{{ row.subtotal.toLocaleString() }}</strong>
        </template>
      </el-table-column>
      <el-table-column
        prop="quantity_tolerance_pct"
        :label="t('sales.orderView.tolerance')"
        width="100"
        align="right"
      >
        <template #default="{ row }">
          {{ row.quantity_tolerance_pct != null ? Number(row.quantity_tolerance_pct) + '%' : '-' }}
        </template>
      </el-table-column>
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { SalesOrder } from '@/api/sales';
import { salesStatusLabelKey, salesStatusTagType } from '@/utils/sales-status';

const { t } = useI18n({ useScope: 'global' });

defineProps<{
  visible: boolean;
  order: SalesOrder | null;
}>();

const emit = defineEmits<{
  'update:visible': [value: boolean];
}>();

const getStatusType = (status: string | undefined) => salesStatusTagType(status);

/** 状态文案：词表外的值由 utils/sales-status 抛错暴露，不再回显裸枚举；订单未加载时无状态可显示 */
const getStatusText = (status: string | undefined) => {
  const key = salesStatusLabelKey(status);
  return key ? t(key) : '';
};
</script>

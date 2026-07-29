<script setup lang="ts">
/**
 * ViewDlg - 采购单详情对话框（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 查看对话框）
 */
import { useI18n } from 'vue-i18n';
import type { PurchaseOrder } from '@/api/purchase';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  data: PurchaseOrder | null;
  getStatusType: (s: string) => string;
  getStatusText: (s: string) => string;
  getPaymentStatusType: (s: string) => string;
  getPaymentStatusText: (s: string) => string;
}

defineProps<Props>();

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void;
}>();
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    :title="t('purchase.viewDlg.title')"
    width="800px"
    :aria-label="t('purchase.viewDlg.ariaLabel')"
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <template v-if="data">
      <el-descriptions :column="2" border>
        <el-descriptions-item :label="t('purchase.viewDlg.orderNo')">{{
          data.order_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchase.viewDlg.supplier')">{{
          data.supplier_name
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchase.viewDlg.orderDate')">{{
          data.order_date
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchase.viewDlg.requiredDate')">{{
          data.required_date
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchase.viewDlg.totalAmount')"
          >¥{{ data.total_amount?.toLocaleString() }}</el-descriptions-item
        >
        <el-descriptions-item :label="t('purchase.viewDlg.receivedAmount')"
          >¥{{ (data.received_amount || 0).toLocaleString() }}</el-descriptions-item
        >
        <el-descriptions-item :label="t('purchase.viewDlg.paymentStatus')">
          <el-tag :type="getPaymentStatusType(data.payment_status || '')">{{
            getPaymentStatusText(data.payment_status || '')
          }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('purchase.viewDlg.status')">
          <el-tag :type="getStatusType(data.status)">{{ getStatusText(data.status) }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('purchase.viewDlg.creator')">{{
          data.creator_name
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchase.viewDlg.createdAt')">{{
          data.created_at
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchase.viewDlg.remark')" :span="2">{{
          data.remarks || t('purchase.viewDlg.noRemark')
        }}</el-descriptions-item>
      </el-descriptions>
      <div style="margin-top: 20px">
        <h4>{{ t('purchase.viewDlg.detailTitle') }}</h4>
        <el-table
          :data="data.items || []"
          border
          style="width: 100%"
          :aria-label="t('purchase.viewDlg.detailListAria')"
        >
          <el-table-column
            prop="product_name"
            :label="t('purchase.viewDlg.colProduct')"
            width="150"
          />
          <el-table-column
            prop="product_code"
            :label="t('purchase.viewDlg.colProductCode')"
            width="120"
          />
          <el-table-column prop="quantity" :label="t('purchase.viewDlg.colQuantity')" width="100" />
          <el-table-column
            prop="unit_price"
            :label="t('purchase.viewDlg.colUnitPrice')"
            width="100"
          />
          <el-table-column prop="subtotal" :label="t('purchase.viewDlg.colSubtotal')" width="120" />
          <el-table-column
            prop="received_quantity"
            :label="t('purchase.viewDlg.colReceived')"
            width="100"
          />
          <el-table-column prop="remarks" :label="t('purchase.viewDlg.colRemark')" />
        </el-table>
      </div>
    </template>
  </el-dialog>
</template>

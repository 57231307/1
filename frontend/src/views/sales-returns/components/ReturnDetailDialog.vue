<!--
  ReturnDetailDialog.vue - 销售退货详情对话框
  任务编号: P14 批 2 I-3 第 7 批
  拆分原 sales-returns/index.vue 的详情对话框部分
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('salesReturns.detailDialog.dialogTitle')"
    width="800px"
    :aria-label="t('salesReturns.detailDialog.dialogAriaLabel')"
    @update:model-value="onClose"
  >
    <template v-if="currentReturn">
      <el-descriptions :column="2" border>
        <el-descriptions-item :label="t('salesReturns.detailDialog.labelReturnNo')">{{
          currentReturn.returnNo
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesReturns.detailDialog.labelSalesOrderNo')">{{
          currentReturn.salesOrderNo
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesReturns.detailDialog.labelCustomerName')">{{
          currentReturn.customerName
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesReturns.detailDialog.labelReturnDate')">{{
          currentReturn.returnDate
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesReturns.detailDialog.labelReturnAmount')">{{
          formatAmount(currentReturn.totalAmount ?? 0)
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesReturns.detailDialog.labelStatus')">
          <el-tag :type="getStatusType(currentReturn.status ?? '')">
            {{ getStatusLabel(currentReturn.status ?? '') }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('salesReturns.detailDialog.labelReason')" :span="2">{{
          currentReturn.reason
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesReturns.detailDialog.labelRemarks')" :span="2">{{
          currentReturn.remarks
        }}</el-descriptions-item>
      </el-descriptions>

      <div style="margin-top: 20px">
        <h4>{{ t('salesReturns.detailDialog.titleReturnDetails') }}</h4>
        <el-table
          :data="currentReturn.items || []"
          border
          size="small"
          :aria-label="t('salesReturns.detailDialog.detailsTableAriaLabel')"
        >
          <el-table-column
            prop="productName"
            :label="t('salesReturns.detailDialog.columnProductName')"
          />
          <el-table-column
            prop="productCode"
            :label="t('salesReturns.detailDialog.columnProductCode')"
          />
          <el-table-column prop="quantity" :label="t('salesReturns.detailDialog.columnQuantity')" />
          <el-table-column
            prop="unitPrice"
            :label="t('salesReturns.detailDialog.columnUnitPrice')"
          />
          <el-table-column prop="amount" :label="t('salesReturns.detailDialog.columnAmount')" />
          <el-table-column prop="reason" :label="t('salesReturns.detailDialog.columnReason')" />
        </el-table>
      </div>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { getStatusType, formatAmount } from '../composables/srFmts';
import type { SalesReturn } from '@/api/sales-return';

const { t } = useI18n({ useScope: 'global' });

defineProps<{
  visible: boolean;
  currentReturn: SalesReturn | null;
}>();

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void;
}>();

/** 获取退货状态标签（i18n 响应式） */
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    PENDING: t('salesReturns.detailDialog.statusPending'),
    APPROVED: t('salesReturns.detailDialog.statusApproved'),
    REJECTED: t('salesReturns.detailDialog.statusRejected'),
    COMPLETED: t('salesReturns.detailDialog.statusCompleted'),
  };
  return map[status] || status;
};

const onClose = (val: boolean) => {
  emit('update:visible', val);
};
</script>

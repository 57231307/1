<!--
  PurchaseReceiptDetail.vue - 采购入库详情
  拆分自 purchaseReceipt/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('purchaseReceipt.detail.title')"
    width="800px"
    :aria-label="t('purchaseReceipt.detail.aria.dialog')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <div v-if="data">
      <el-descriptions :column="4" border>
        <el-descriptions-item :label="t('purchaseReceipt.detail.label.receiptNo')">{{
          data.receipt_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseReceipt.detail.label.receiptDate')">{{
          data.receipt_date
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseReceipt.detail.label.purchaseOrderNo')">{{
          data.purchase_order_no || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseReceipt.detail.label.supplier')">{{
          data.supplier_name
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseReceipt.detail.label.warehouse')">{{
          data.warehouse_name
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseReceipt.detail.label.amount')">{{
          data.total_amount != null ? formatCurrency(data.total_amount) : '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseReceipt.detail.label.status')">{{
          getStatusLabelFmt(data.receipt_status)
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseReceipt.detail.label.createdBy')">{{
          data.created_by_name
        }}</el-descriptions-item>
      </el-descriptions>
      <div class="detail-items">
        <h4>{{ t('purchaseReceipt.detail.itemsTitle') }}</h4>
        <el-table
          :data="items"
          border
          style="width: 100%"
          :aria-label="t('purchaseReceipt.detail.aria.itemsList')"
        >
          <el-table-column
            prop="material_code"
            :label="t('purchaseReceipt.detail.column.productCode')"
            width="120"
          />
          <el-table-column
            prop="material_name"
            :label="t('purchaseReceipt.detail.column.productName')"
            width="150"
          />
          <el-table-column
            prop="color_code"
            :label="t('purchaseReceipt.detail.column.colorNo')"
            width="100"
          />
          <el-table-column
            prop="grade"
            :label="t('purchaseReceipt.detail.column.grade')"
            width="80"
          />
          <el-table-column
            prop="quantity"
            :label="t('purchaseReceipt.detail.column.quantity')"
            width="100"
            align="right"
          />
          <el-table-column prop="unit_master" :label="t('purchaseReceipt.detail.column.unit')" />
          <el-table-column
            prop="quantity_alt"
            :label="t('purchaseReceipt.detail.column.quantityAlt')"
            width="100"
            align="right"
          />
          <el-table-column
            prop="unit_price"
            :label="t('purchaseReceipt.detail.column.price')"
            width="100"
            align="right"
          >
            <template #default="scope">
              {{ Number(scope.row.unit_price ?? 0).toFixed(2) }}
            </template>
          </el-table-column>
          <el-table-column
            prop="amount"
            :label="t('purchaseReceipt.detail.column.amount')"
            width="120"
            align="right"
          >
            <template #default="scope">
              {{ Number(scope.row.amount ?? 0).toFixed(2) }}
            </template>
          </el-table-column>
          <el-table-column prop="notes" :label="t('purchaseReceipt.detail.column.remark')" />
        </el-table>
      </div>
    </div>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { formatCurrency } from '@/utils';
import type { PurchaseReceiptEntity, ReceiptItem } from '@/api/purchase-receipt';
import { getReceiptStatusLabel } from '../composables/prcFmts';

const { t } = useI18n({ useScope: 'global' });

/**
 * 采购入库详情组件
 */
defineProps<{
  // 对话框可见性
  visible: boolean;
  // 详情数据
  data: PurchaseReceiptEntity | null;
  // 明细列表
  items: ReceiptItem[];
}>();

const emit = defineEmits<{
  'update:visible': [v: boolean];
}>();

// 透传格式化函数
const getStatusLabelFmt = getReceiptStatusLabel;
</script>

<style scoped>
.detail-items {
  margin-top: 20px;
}
</style>

<!--
  PurchaseReturnDetail.vue - 采购退货详情对话框
  任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('purchaseReturn.detail.title')"
    width="900px"
    :aria-label="t('purchaseReturn.detail.aria.dialog')"
    @update:model-value="onVisibleChange"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item :label="t('purchaseReturn.detail.label.returnNo')">{{
        detailData.returnNo
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseReturn.detail.label.purchaseOrderNo')">{{
        detailData.purchaseOrderNo
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseReturn.detail.label.supplier')">{{
        detailData.supplierName
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseReturn.detail.label.returnDate')">{{
        detailData.returnDate
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseReturn.detail.label.returnAmount')">
        <span class="amount">¥{{ detailData.totalAmount || 0 }}</span>
      </el-descriptions-item>
      <el-descriptions-item :label="t('purchaseReturn.detail.label.status')">
        <el-tag :type="getStatusType(detailData.status || '')">
          {{ getStatusText(detailData.status || '') }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item :label="t('purchaseReturn.detail.label.reason')" :span="2">
        {{ detailData.reason || '-' }}
      </el-descriptions-item>
      <el-descriptions-item :label="t('purchaseReturn.detail.label.remarks')" :span="2">
        {{ detailData.remarks || '-' }}
      </el-descriptions-item>
    </el-descriptions>

    <el-divider content-position="left">{{ t('purchaseReturn.detail.itemsTitle') }}</el-divider>
    <el-table
      :data="detailData.items || []"
      border
      :aria-label="t('purchaseReturn.detail.aria.itemsTable')"
    >
      <el-table-column
        prop="productName"
        :label="t('purchaseReturn.detail.column.productName')"
        min-width="150"
      />
      <el-table-column
        prop="quantity"
        :label="t('purchaseReturn.detail.column.quantity')"
        width="100"
      />
      <el-table-column
        prop="unitPrice"
        :label="t('purchaseReturn.detail.column.unitPrice')"
        width="100"
      />
      <el-table-column
        prop="amount"
        :label="t('purchaseReturn.detail.column.amount')"
        width="120"
      />
      <el-table-column
        prop="reason"
        :label="t('purchaseReturn.detail.column.reason')"
        min-width="150"
      />
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { PurchaseReturn } from '@/api/purchase-return';
import { getStatusType, getStatusText } from '../composables/prRtnFmts';

const { t } = useI18n({ useScope: 'global' });

// 采购退货详情对话框属性
defineProps<{
  // 对话框可见性
  visible: boolean;
  // 详情数据
  detailData: PurchaseReturn;
}>();

// 定义事件
const emit = defineEmits<{
  // 关闭
  (e: 'update:visible', value: boolean): void;
}>();

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v);
};
</script>

<style scoped>
.amount {
  font-weight: 600;
  color: #f56c6c;
}
</style>

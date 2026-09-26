<!--
  PurchaseInspectionDetail.vue - 采购验货详情对话框
  拆分自 purchase-inspection/index.vue（P14 批 2 I-3 第 5 批）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('purchaseInspection.detail.title')"
    width="800px"
    :aria-label="t('purchaseInspection.detail.ariaLabel')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item :label="t('purchaseInspection.detail.label.inspectionNo')">{{
        data.inspection_no
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseInspection.detail.label.receiptNo')">{{
        data.receipt_no
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseInspection.detail.label.supplier')">{{
        data.supplier_name
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseInspection.detail.label.inspectionDate')">{{
        data.inspection_date
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseInspection.detail.label.inspector')">{{
        data.inspector_name
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseInspection.detail.label.status')">
        <el-tag :type="getStatusType(data.inspection_status)">
          {{ getStatusText(data.inspection_status) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item :label="t('purchaseInspection.detail.label.result')">
        <el-tag v-if="data.inspection_result" :type="getResultType(data.inspection_result)">
          {{ getResultText(data.inspection_result) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item :label="t('purchaseInspection.detail.label.remark')">{{
        data.notes
      }}</el-descriptions-item>
    </el-descriptions>

    <el-divider content-position="left">{{
      t('purchaseInspection.detail.divider.items')
    }}</el-divider>
    <!-- 明细由 handleView 异步加载到 detailItems（表头 get_inspection 不含 items） -->
    <el-table
      :data="detailItems"
      border
      :aria-label="t('purchaseInspection.detail.ariaLabelItemsTable')"
    >
      <el-table-column
        prop="product_name"
        :label="t('purchaseInspection.detail.column.productName')"
        min-width="150"
      />
      <el-table-column
        prop="expected_quantity"
        :label="t('purchaseInspection.detail.column.expectedQuantity')"
        width="100"
      />
      <el-table-column
        prop="inspected_quantity"
        :label="t('purchaseInspection.detail.column.inspectedQuantity')"
        width="100"
      />
      <el-table-column
        prop="passed_quantity"
        :label="t('purchaseInspection.detail.column.passedQuantity')"
        width="100"
      />
      <el-table-column
        prop="failed_quantity"
        :label="t('purchaseInspection.detail.column.failedQuantity')"
        width="100"
      />
      <el-table-column
        prop="defect_reason"
        :label="t('purchaseInspection.detail.column.defectReason')"
        min-width="150"
      />
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { getStatusType, getStatusText, getResultType, getResultText } from '../composables/piFmts';
import type { PurchaseInspection, PurchaseInspectionItem } from '@/api/purchase-inspection';

const { t } = useI18n({ useScope: 'global' });

/**
 * 详情对话框
 * data: 表头数据（get_inspection 仅返回 Model，不含 items）
 * detailItems: 由 handleView 异步加载的明细数组（/inspections/{id}/items）
 */
defineProps<{
  // 可见性
  visible: boolean;
  // 表头详情数据
  data: PurchaseInspection;
  // 明细列表（独立于 data.items，handleView 加载后传入）
  detailItems: PurchaseInspectionItem[];
}>();

const emit = defineEmits<{
  'update:visible': [v: boolean];
}>();
</script>

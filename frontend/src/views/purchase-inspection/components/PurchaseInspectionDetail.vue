<!--
  PurchaseInspectionDetail.vue - 采购验货详情对话框
  拆分自 purchase-inspection/index.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
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
      <el-descriptions-item :label="t('purchaseInspection.detail.label.inspectionNo')">{{ data.inspection_no }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseInspection.detail.label.receiptNo')">{{ data.receipt_no }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseInspection.detail.label.supplier')">{{ data.supplier_name }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseInspection.detail.label.inspectionDate')">{{ data.inspection_date }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseInspection.detail.label.inspector')">{{ data.inspector_name }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseInspection.detail.label.status')">
        <el-tag :type="getStatusType(data.status)">
          {{ getStatusText(data.status) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item :label="t('purchaseInspection.detail.label.result')">
        <el-tag v-if="data.result" :type="getResultType(data.result)">
          {{ getResultText(data.result) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item :label="t('purchaseInspection.detail.label.remark')">{{ data.remark || '-' }}</el-descriptions-item>
    </el-descriptions>

    <el-divider content-position="left">{{ t('purchaseInspection.detail.divider.items') }}</el-divider>
    <el-table :data="data.items || []" border :aria-label="t('purchaseInspection.detail.ariaLabelItemsTable')">
      <el-table-column prop="product_name" :label="t('purchaseInspection.detail.column.productName')" min-width="150" />
      <el-table-column prop="expected_quantity" :label="t('purchaseInspection.detail.column.expectedQuantity')" width="100" />
      <el-table-column prop="inspected_quantity" :label="t('purchaseInspection.detail.column.inspectedQuantity')" width="100" />
      <el-table-column prop="passed_quantity" :label="t('purchaseInspection.detail.column.passedQuantity')" width="100" />
      <el-table-column prop="failed_quantity" :label="t('purchaseInspection.detail.column.failedQuantity')" width="100" />
      <el-table-column prop="defect_reason" :label="t('purchaseInspection.detail.column.defectReason')" min-width="150" />
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { getStatusType, getStatusText, getResultType, getResultText } from '../composables/piFmts'
import type { PurchaseInspection } from '@/api/purchase-inspection'

const { t } = useI18n({ useScope: 'global' })

/**
 * 详情对话框
 */
defineProps<{
  // 可见性
  visible: boolean
  // 详情数据
  data: PurchaseInspection
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
}>()
</script>

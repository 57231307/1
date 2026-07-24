<!--
  PurchaseInspectionDetail.vue - 采购验货详情对话框
  拆分自 purchase-inspection/index.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="检验单详情"
    width="800px"
    aria-label="检验单详情对话框"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item label="检验单号">{{ data.inspection_no }}</el-descriptions-item>
      <el-descriptions-item label="入库单号">{{ data.receipt_no }}</el-descriptions-item>
      <el-descriptions-item label="供应商">{{ data.supplier_name }}</el-descriptions-item>
      <el-descriptions-item label="检验日期">{{ data.inspection_date }}</el-descriptions-item>
      <el-descriptions-item label="检验员">{{ data.inspector_name }}</el-descriptions-item>
      <el-descriptions-item label="状态">
        <el-tag :type="getStatusType(data.status)">
          {{ getStatusText(data.status) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item label="检验结果">
        <el-tag v-if="data.result" :type="getResultType(data.result)">
          {{ getResultText(data.result) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item label="备注">{{ data.remark || '-' }}</el-descriptions-item>
    </el-descriptions>

    <el-divider content-position="left">检验明细</el-divider>
    <el-table :data="data.items || []" border aria-label="检验明细表">
      <el-table-column prop="product_name" label="产品名称" min-width="150" />
      <el-table-column prop="expected_quantity" label="预期数量" width="100" />
      <el-table-column prop="inspected_quantity" label="检验数量" width="100" />
      <el-table-column prop="passed_quantity" label="合格数量" width="100" />
      <el-table-column prop="failed_quantity" label="不合格数量" width="100" />
      <el-table-column prop="defect_reason" label="缺陷原因" min-width="150" />
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { getStatusType, getStatusText, getResultType, getResultText } from '../composables/piFmts'
import type { PurchaseInspection } from '@/api/purchase-inspection'

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

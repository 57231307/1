<!-- eslint-disable vue/no-mutating-props -->
<!--
  PrRtnDetail.vue - 采购退货详情对话框
  任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="退货单详情"
    width="900px"
    @update:model-value="onVisibleChange"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item label="退货单号">{{ detailData.returnNo }}</el-descriptions-item>
      <el-descriptions-item label="采购单号">{{ detailData.purchaseOrderNo }}</el-descriptions-item>
      <el-descriptions-item label="供应商">{{ detailData.supplierName }}</el-descriptions-item>
      <el-descriptions-item label="退货日期">{{ detailData.returnDate }}</el-descriptions-item>
      <el-descriptions-item label="退货金额">
        <span class="amount">¥{{ detailData.totalAmount || 0 }}</span>
      </el-descriptions-item>
      <el-descriptions-item label="状态">
        <el-tag :type="getStatusType(detailData.status || '')">
          {{ getStatusText(detailData.status || '') }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item label="退货原因" :span="2">
        {{ detailData.reason || '-' }}
      </el-descriptions-item>
      <el-descriptions-item label="备注" :span="2">
        {{ detailData.remarks || '-' }}
      </el-descriptions-item>
    </el-descriptions>

    <el-divider content-position="left">退货明细</el-divider>
    <el-table :data="detailData.items || []" border>
      <el-table-column prop="productName" label="产品名称" min-width="150" />
      <el-table-column prop="quantity" label="退货数量" width="100" />
      <el-table-column prop="unitPrice" label="单价" width="100" />
      <el-table-column prop="amount" label="金额" width="120" />
      <el-table-column prop="reason" label="退货原因" min-width="150" />
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import type { PurchaseReturn } from '@/api/purchase-return'
import { getStatusType, getStatusText } from '../composables/prRtnFmts'

// 采购退货详情对话框属性
const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 详情数据
  detailData: PurchaseReturn
}>()

// 定义事件
const emit = defineEmits<{
  // 关闭
  (e: 'update:visible', value: boolean): void
}>()

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v)
}
</script>

<style scoped>
.amount {
  font-weight: 600;
  color: #f56c6c;
}
</style>

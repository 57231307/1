<!--
  ReturnDetailDialog.vue - 销售退货详情对话框
  任务编号: P14 批 2 I-3 第 7 批
  拆分原 sales-returns/index.vue 的详情对话框部分
-->
<template>
  <el-dialog :model-value="visible" title="退货单详情" width="800px" @update:model-value="onClose">
    <el-descriptions :column="2" border>
      <el-descriptions-item label="退货单号">{{ currentReturn.returnNo }}</el-descriptions-item>
      <el-descriptions-item label="销售订单号">{{ currentReturn.salesOrderNo }}</el-descriptions-item>
      <el-descriptions-item label="客户名称">{{ currentReturn.customerName }}</el-descriptions-item>
      <el-descriptions-item label="退货日期">{{ currentReturn.returnDate }}</el-descriptions-item>
      <el-descriptions-item label="退货金额"
        >{{ formatAmount(currentReturn.totalAmount) }}</el-descriptions-item
      >
      <el-descriptions-item label="状态">
        <el-tag :type="getStatusType(currentReturn.status)">
          {{ getStatusLabel(currentReturn.status) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item label="退货原因" :span="2">{{ currentReturn.reason }}</el-descriptions-item>
      <el-descriptions-item label="备注" :span="2">{{ currentReturn.remarks }}</el-descriptions-item>
    </el-descriptions>

    <div style="margin-top: 20px">
      <h4>退货明细</h4>
      <el-table :data="currentReturn.items || []" border size="small">
        <el-table-column prop="productName" label="产品名称" />
        <el-table-column prop="productCode" label="产品编码" />
        <el-table-column prop="quantity" label="退货数量" />
        <el-table-column prop="unitPrice" label="单价" />
        <el-table-column prop="amount" label="金额" />
        <el-table-column prop="reason" label="退货原因" />
      </el-table>
    </div>
  </el-dialog>
</template>

<script setup lang="ts">
import { getStatusType, getStatusLabel, formatAmount } from '../composables/srFmts'

defineProps<{
  visible: boolean
  currentReturn: any
}>()

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void
}>()

const onClose = (val: boolean) => {
  emit('update:visible', val)
}
</script>

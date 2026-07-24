<!--
  PurchaseReceiptDetail.vue - 采购入库详情
  拆分自 purchaseReceipt/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="入库单详情"
    width="800px"
    aria-label="采购入库单详情对话框"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <div v-if="data">
      <el-descriptions :column="4" border>
        <el-descriptions-item label="入库单号">{{ data.receipt_no }}</el-descriptions-item>
        <el-descriptions-item label="入库日期">{{ data.receipt_date }}</el-descriptions-item>
        <el-descriptions-item label="采购订单号">{{
          data.purchase_order_no || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="供应商">{{ data.supplier_name }}</el-descriptions-item>
        <el-descriptions-item label="仓库">{{ data.warehouse_name }}</el-descriptions-item>
        <el-descriptions-item label="入库金额">{{
          (data.total_amount || 0).toFixed(2)
        }}</el-descriptions-item>
        <el-descriptions-item label="状态">{{ getStatusLabelFmt(data.status) }}</el-descriptions-item>
        <el-descriptions-item label="创建人">{{ data.created_by_name }}</el-descriptions-item>
      </el-descriptions>
      <div class="detail-items">
        <h4>入库明细</h4>
        <el-table :data="items" border style="width: 100%" aria-label="入库单明细列表">
          <el-table-column prop="product_code" label="产品编码" width="120" />
          <el-table-column prop="product_name" label="产品名称" width="150" />
          <el-table-column prop="color_no" label="色号" width="100" />
          <el-table-column prop="grade" label="等级" width="80" />
          <el-table-column prop="quantity" label="数量" width="100" align="right" />
          <el-table-column prop="price" label="单价" width="100" align="right">
            <template #default="scope">
              {{ (scope.row.price || 0).toFixed(2) }}
            </template>
          </el-table-column>
          <el-table-column prop="amount" label="金额" width="120" align="right">
            <template #default="scope">
              {{ (scope.row.amount || 0).toFixed(2) }}
            </template>
          </el-table-column>
          <el-table-column prop="remark" label="备注" />
        </el-table>
      </div>
    </div>
  </el-dialog>
</template>

<script setup lang="ts">
import type { PurchaseReceiptEntity, ReceiptItem } from '@/api/purchaseReceipt'
import { getStatusLabel } from '../composables/prcFmts'

/**
 * 采购入库详情组件
 */
defineProps<{
  // 对话框可见性
  visible: boolean
  // 详情数据
  data: PurchaseReceiptEntity | null
  // 明细列表
  items: ReceiptItem[]
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
}>()

// 透传格式化函数
const getStatusLabelFmt = getStatusLabel
</script>

<style scoped>
.detail-items {
  margin-top: 20px;
}
</style>

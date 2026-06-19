<!--
  PpDetail.vue - 采购价格查看详情对话框
  拆分自 purchase-price/index.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    title="价格详情"
    width="700px"
    @update:model-value="onVisibleChange"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item label="产品名称">{{ viewData.product_name }}</el-descriptions-item>
      <el-descriptions-item label="供应商">{{ viewData.supplier_name }}</el-descriptions-item>
      <el-descriptions-item label="采购价格">{{
        formatCurrency(viewData.price || 0)
      }}</el-descriptions-item>
      <el-descriptions-item label="币种">{{ viewData.currency }}</el-descriptions-item>
      <el-descriptions-item label="单位">{{ viewData.unit }}</el-descriptions-item>
      <el-descriptions-item label="最小订购量">{{
        viewData.min_order_qty || '-'
      }}</el-descriptions-item>
      <el-descriptions-item label="价格类型">{{
        getPriceTypeLabel(viewData.price_type || '')
      }}</el-descriptions-item>
      <el-descriptions-item label="状态">
        <el-tag :type="getStatusType(viewData.status || '')">{{
          getStatusLabel(viewData.status || '')
        }}</el-tag>
      </el-descriptions-item>
      <el-descriptions-item label="生效日期">{{ viewData.effective_date }}</el-descriptions-item>
      <el-descriptions-item label="到期日期">{{
        viewData.expiry_date || '-'
      }}</el-descriptions-item>
      <el-descriptions-item label="备注" :span="2">{{
        viewData.remarks || '-'
      }}</el-descriptions-item>
    </el-descriptions>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import {
  formatCurrency,
  getPriceTypeLabel,
  getStatusType,
  getStatusLabel,
} from '../composables/ppFmts'

// 查看详情数据类型
interface PpViewData {
  product_name?: string
  supplier_name?: string
  price?: number
  currency?: string
  unit?: string
  min_order_qty?: number
  price_type?: string
  status?: string
  effective_date?: string
  expiry_date?: string
  remarks?: string
}

/**
 * 采购价格查看详情对话框组件
 */
defineProps<{
  // 对话框可见性
  visible: boolean
  // 详情数据
  viewData: PpViewData
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
}>()

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v)
}
</script>

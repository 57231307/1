<!--
  PurchaseContractDetail.vue - 采购合同查看详情对话框
  拆分自 purchase-contract/index.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="合同详情"
    width="800px"
    aria-label="采购合同详情对话框"
    @update:model-value="onVisibleChange"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item label="合同编号">{{ viewData.contract_no }}</el-descriptions-item>
      <el-descriptions-item label="合同名称">{{ viewData.contract_name }}</el-descriptions-item>
      <el-descriptions-item label="供应商">{{ viewData.supplier_name }}</el-descriptions-item>
      <el-descriptions-item label="合同类型">{{ viewData.contract_type }}</el-descriptions-item>
      <el-descriptions-item label="合同金额">{{
        formatCurrency(viewData.total_amount || 0)
      }}</el-descriptions-item>
      <el-descriptions-item label="签订日期">{{ viewData.signed_date }}</el-descriptions-item>
      <el-descriptions-item label="生效日期">{{ viewData.effective_date }}</el-descriptions-item>
      <el-descriptions-item label="到期日期">{{ viewData.expiry_date }}</el-descriptions-item>
      <el-descriptions-item label="付款条件">{{
        viewData.payment_terms || '-'
      }}</el-descriptions-item>
      <el-descriptions-item label="付款方式">{{
        viewData.payment_method || '-'
      }}</el-descriptions-item>
      <el-descriptions-item label="交货日期">{{
        viewData.delivery_date || '-'
      }}</el-descriptions-item>
      <el-descriptions-item label="交货地点">{{
        viewData.delivery_location || '-'
      }}</el-descriptions-item>
      <el-descriptions-item label="状态">
        <el-tag :type="getStatusType(viewData.status || '')">{{
          getStatusLabel(viewData.status || '')
        }}</el-tag>
      </el-descriptions-item>
      <el-descriptions-item label="备注" :span="2">{{
        viewData.remarks || '-'
      }}</el-descriptions-item>
    </el-descriptions>
  </el-dialog>
</template>

<script setup lang="ts">
import { formatCurrency, getStatusType, getStatusLabel } from '../composables/pcFmts'

// 详情数据类型
interface PcViewData {
  contract_no?: string
  contract_name?: string
  supplier_name?: string
  contract_type?: string
  total_amount?: number
  signed_date?: string
  effective_date?: string
  expiry_date?: string
  payment_terms?: string
  payment_method?: string
  delivery_date?: string
  delivery_location?: string
  status?: string
  remarks?: string
}

/**
 * 采购合同查看详情对话框组件
 */
defineProps<{
  // 对话框可见性
  visible: boolean
  // 详情数据
  viewData: PcViewData
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
}>()

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v)
}
</script>

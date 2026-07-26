<!--
  SalesPriceView.vue - 销售价格查看详情对话框
  拆分自 sales-price/index.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('salesPrice.view.dialogTitle')"
    width="600px"
    :aria-label="t('salesPrice.view.dialogAriaLabel')"
    @update:model-value="onVisibleChange"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item :label="t('salesPrice.view.labelProductName')">{{
        viewData.product_name
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelCustomer')">{{
        viewData.customer_name || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelPrice')">{{
        formatCurrency(viewData.price || 0)
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelCurrency')">{{
        viewData.currency
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelUnit')">{{
        viewData.unit
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelMinOrderQty')">{{
        viewData.min_order_qty || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelPriceType')">{{
        getPriceTypeLabel(viewData.price_type || '')
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelPriceLevel')">{{
        viewData.price_level || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelEffectiveDate')">{{
        viewData.effective_date || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelExpiryDate')">{{
        viewData.expiry_date || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelStatus')">
        <el-tag :type="getStatusType(viewData.status || '')">{{
          getStatusLabel(viewData.status || '')
        }}</el-tag>
      </el-descriptions-item>
      <el-descriptions-item :label="t('salesPrice.view.labelRemarks')" :span="2">{{
        viewData.remarks || '-'
      }}</el-descriptions-item>
    </el-descriptions>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { formatCurrency, getStatusType } from '../composables/spFmts'

const { t } = useI18n({ useScope: 'global' })

// 查看详情数据类型
interface SpViewData {
  product_name?: string
  customer_name?: string
  price?: number
  currency?: string
  unit?: string
  min_order_qty?: number
  price_type?: string
  price_level?: string
  effective_date?: string
  expiry_date?: string
  status?: string
  remarks?: string
}

/**
 * 销售价格查看详情对话框组件
 */
defineProps<{
  // 对话框可见性
  visible: boolean
  // 详情数据
  viewData: SpViewData
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
}>()

/** 获取价格类型标签（i18n 响应式） */
const getPriceTypeLabel = (type: string) => {
  const map: Record<string, string> = {
    STANDARD: t('salesPrice.view.priceTypeStandard'),
    AGREED: t('salesPrice.view.priceTypeAgreed'),
    PROMOTION: t('salesPrice.view.priceTypePromotion'),
  }
  return map[type] || type
}

/** 获取销售价格状态标签（i18n 响应式） */
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending: t('salesPrice.view.statusPending'),
    active: t('salesPrice.view.statusActive'),
    expired: t('salesPrice.view.statusExpired'),
    inactive: t('salesPrice.view.statusInactive'),
  }
  return map[status] || status
}

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v)
}
</script>

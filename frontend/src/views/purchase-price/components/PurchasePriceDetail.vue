<!--
  PurchasePriceDetail.vue - 采购价格查看详情对话框
  拆分自 purchase-price/index.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('purchasePrice.detail.title')"
    width="700px"
    :aria-label="t('purchasePrice.detail.ariaLabel')"
    @update:model-value="onVisibleChange"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item :label="t('purchasePrice.detail.label.productName')">{{
        viewData.product_name
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchasePrice.detail.label.supplier')">{{
        viewData.supplier_name
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchasePrice.detail.label.price')">{{
        formatCurrency(viewData.price)
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchasePrice.detail.label.currency')">{{
        viewData.currency
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchasePrice.detail.label.unit')">{{
        viewData.unit
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchasePrice.detail.label.minOrderQty')">{{
        viewData.min_order_qty || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchasePrice.detail.label.priceType')">{{
        getPriceTypeLabel(viewData.price_type || '')
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchasePrice.detail.label.status')">
        <el-tag :type="getStatusType(viewData.status)">{{
          getStatusLabel(viewData.status)
        }}</el-tag>
      </el-descriptions-item>
      <el-descriptions-item :label="t('purchasePrice.detail.label.effectiveDate')">{{
        viewData.effective_date
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchasePrice.detail.label.expiryDate')">{{
        viewData.expiry_date || '-'
      }}</el-descriptions-item>
    </el-descriptions>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
// F2 同形收口：状态文案与货币格式化统一走 ppFmts（无组件局部 map）；
// 此处不再 `viewData.price || 0` 把缺值掩盖成 ¥0.000000，缺值由 formatCurrency 显示 common.valueMissing
import {
  formatCurrency,
  getPriceTypeLabel,
  getStatusType,
  getStatusLabel,
} from '../composables/ppFmts';

const { t } = useI18n({ useScope: 'global' });

// 查看详情数据类型（对齐后端 purchase_price::Model 可空列；product_name/supplier_name 为待补 JOIN 列；
// price/min_order_qty 按后端 Decimal→JSON 字符串如实声明，见 api/purchase-price.ts 头注）
interface PpViewData {
  product_name?: string | null;
  supplier_name?: string | null;
  price?: string;
  currency?: string;
  unit?: string;
  min_order_qty?: string;
  price_type?: string;
  status?: string;
  effective_date?: string;
  expiry_date?: string | null;
}

/**
 * 采购价格查看详情对话框组件
 */
defineProps<{
  // 对话框可见性
  visible: boolean;
  // 详情数据
  viewData: PpViewData;
}>();

const emit = defineEmits<{
  'update:visible': [v: boolean];
}>();

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v);
};
</script>

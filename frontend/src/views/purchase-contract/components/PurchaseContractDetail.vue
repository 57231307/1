<!--
  PurchaseContractDetail.vue - 采购合同查看详情对话框
  拆分自 purchase-contract/index.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('purchaseContract.detail.title')"
    width="800px"
    :aria-label="t('purchaseContract.detail.ariaLabel')"
    @update:model-value="onVisibleChange"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item :label="t('purchaseContract.detail.contractNo')">{{
        viewData.contract_no
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseContract.detail.contractName')">{{
        viewData.contract_name
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseContract.detail.supplier')">{{
        viewData.supplier_name
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseContract.detail.contractType')">{{
        viewData.contract_type
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseContract.detail.totalAmount')">{{
        formatCurrency(viewData.total_amount || 0)
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseContract.detail.signedDate')">{{
        viewData.signed_date
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseContract.detail.effectiveDate')">{{
        viewData.effective_date
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseContract.detail.expiryDate')">{{
        viewData.expiry_date
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseContract.detail.paymentTerms')">{{
        viewData.payment_terms || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseContract.detail.paymentMethod')">{{
        viewData.payment_method || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseContract.detail.deliveryDate')">{{
        viewData.delivery_date || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseContract.detail.deliveryLocation')">{{
        viewData.delivery_location || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('purchaseContract.detail.status')">
        <el-tag :type="getStatusType(viewData.status || '')">{{
          t(`purchaseContract.status.${viewData.status || ''}`)
        }}</el-tag>
      </el-descriptions-item>
      <el-descriptions-item :label="t('purchaseContract.detail.remarks')" :span="2">{{
        viewData.remarks || '-'
      }}</el-descriptions-item>
    </el-descriptions>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import { formatCurrency, getStatusType } from '../composables/pcFmts';

const { t } = useI18n({ useScope: 'global' });

// 详情数据类型（对齐后端 purchase_contract::Model 可空列）
interface PcViewData {
  contract_no?: string;
  contract_name?: string;
  supplier_name?: string | null;
  contract_type?: string | null;
  total_amount?: number | null;
  signed_date?: string | null;
  effective_date?: string | null;
  expiry_date?: string | null;
  payment_terms?: string | null;
  payment_method?: string | null;
  delivery_date?: string | null;
  delivery_location?: string | null;
  status?: string;
  remarks?: string | null;
}

/**
 * 采购合同查看详情对话框组件
 */
defineProps<{
  // 对话框可见性
  visible: boolean;
  // 详情数据
  viewData: PcViewData;
}>();

const emit = defineEmits<{
  'update:visible': [v: boolean];
}>();

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v);
};
</script>

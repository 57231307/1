<!--
  purchaseReceipt/index.vue - 采购入库管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 4 批
  拆分：598 行 → ~150 行 + 4 子组件 + 2 composable + 1 工具
  批次 285：PurchaseReceiptFilter/PurchaseReceiptTable 接入 useTableApi（v-model:page/page-size + @fetch + @update:queryParams）
-->
<template>
  <div class="app-container" :aria-label="t('purchaseReceipt.index.pageAriaLabel')">
    <PurchaseReceiptFilter
      :query-params="prc.queryParams"
      :suppliers="prc.supplierOptions"
      :warehouses="prc.warehouseOptions"
      :status-options="statusOptions"
      @fetch="prcProc.handleSearch"
      @update:query-params="v => Object.assign(prc.queryParams, v)"
      @add="prcProc.openAddDialog"
    />

    <PurchaseReceiptTable
      v-model:page="prc.page"
      v-model:page-size="prc.pageSize"
      :data="prc.tableData"
      :loading="prc.loading"
      :total="prc.total"
      @view="prcProc.openViewDialog"
      @edit="prcProc.openEditDialog"
      @approve="prcProc.handleApprove"
      @delete="prcProc.handleDelete"
    />

    <PurchaseReceiptForm
      v-model:visible="prc.dialogVisible"
      :title="prc.dialogTitle"
      :form="prc.form"
      :rules="prc.formRules"
      :suppliers="prc.supplierOptions"
      :warehouses="prc.warehouseOptions"
      :products="prc.productOptions"
      @add-item="prcProc.addItem"
      @remove-item="prcProc.removeItem"
      @calc-amount="prcProc.calculateItemAmount"
      @submit="prcProc.handleSubmit"
      @update:form="v => (prc.form = v)"
    />

    <PurchaseReceiptDetail
      v-model:visible="prc.viewDialogVisible"
      :data="prc.viewData"
      :items="prc.detailData"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import {
  PURCHASE_RECEIPT_STATUSES,
  purchaseReceiptStatusLabelKey,
} from '@/utils/purchase-receipt-status';
import { usePrc } from './composables/usePrc';
import { usePrcProc } from './composables/usePrcProc';
import PurchaseReceiptFilter from './components/PurchaseReceiptFilter.vue';
import PurchaseReceiptTable from './components/PurchaseReceiptTable.vue';
import PurchaseReceiptForm from './components/PurchaseReceiptForm.vue';
import PurchaseReceiptDetail from './components/PurchaseReceiptDetail.vue';

const { t } = useI18n({ useScope: 'global' });

// 业务状态
const prc = usePrc();
// 直接传入 usePrc 返回的 reactive 代理：proc 内 cb.dialogVisible = true 等写入
// 经 proxy set 回写到底层 ref，模板 v-model:visible 立即响应；对象字面量快照会把
// 解包后的普通值传进去，proc 的写入只落在临时对象上、底层 ref 永不被通知 → 弹框不打开。
const prcProc = usePrcProc(prc);

// 状态选项：取值来自与后端写入原值逐字一致的状态词表，标签走 i18n
// （不再本地维护裸中文/大小写不符的选项数组——原 'draft'/'approved' 小写值与
// 后端 receipt_status 的 DRAFT/CONFIRMED/COMPLETED 永不相等，筛选恒零命中）
const statusOptions = computed(() => [
  { value: '', label: t('purchaseReceipt.filter.label.all') },
  ...PURCHASE_RECEIPT_STATUSES.map(value => ({
    value,
    label: t(purchaseReceiptStatusLabelKey(value)),
  })),
]);

// 懒加载标记
const hasLoaded = createLazyLoader();

// 列表由 useTableApi setup 自动加载，onMounted 仅加载辅助数据
onMounted(() => {
  loadIfNot('suppliers', prc.loadSuppliers, hasLoaded);
  loadIfNot('warehouses', prc.loadWarehouses, hasLoaded);
  loadIfNot('products', prc.loadProducts, hasLoaded);
});
</script>

<style scoped>
.app-container {
  padding: 20px;
}
</style>

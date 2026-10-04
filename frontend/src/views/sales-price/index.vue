<!--
  sales-price/index.vue - 销售价格管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 3 批
  拆分：677 行 → ~150 行 + 5 子组件 + 2 composable + 1 工具
  批次 284：SalesPriceFilter/SalesPriceTable 接入 useTableApi（v-model:page/page-size + @fetch + @update:queryParams）
-->
<template>
  <div class="sales-price-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ t('salesPrice.index.pageTitle') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{
            t('salesPrice.index.breadcrumbHome')
          }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('salesPrice.index.breadcrumbSales') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('salesPrice.index.breadcrumbSalesPrice') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="onCreate">
          <el-icon><Plus /></el-icon>
          {{ t('salesPrice.index.buttonCreatePrice') }}
        </el-button>
        <el-button @click="spProc.handleStrategy">
          <el-icon><Setting /></el-icon>
          {{ t('salesPrice.index.buttonPriceStrategy') }}
        </el-button>
        <el-button v-permission="'sales.price.export'" @click="onExport">
          <el-icon><Download /></el-icon>
          {{ t('salesPrice.index.buttonExport') }}
        </el-button>
      </div>
    </div>

    <SalesPriceFilter
      :query-params="sp.queryParams"
      :customers="sp.customers"
      :products="sp.products"
      @fetch="sp.handleQuery"
      @update:query-params="v => Object.assign(sp.queryParams, v)"
    />

    <SalesPriceTable
      v-model:page="sp.page"
      v-model:page-size="sp.pageSize"
      :price-list="sp.priceList"
      :loading="sp.loading"
      :total="sp.total"
      @view="spProc.handleView"
      @edit="onEdit"
      @approve="spProc.handleApprove"
      @history="spProc.handleHistory"
    />

    <SalesPriceForm
      v-model:visible="dialogVisible"
      :title="sp.dialogTitle"
      :form-data="sp.formData"
      :form-rules="sp.formRules"
      :customers="sp.customers"
      :products="sp.products"
      @submit="onSubmitForm"
      @update:form-data="v => Object.assign(sp.formData, v)"
    />

    <SalesPriceView v-model:visible="spProc.viewDialogVisible" :view-data="spProc.viewData" />

    <SalesPriceHistory v-model:visible="spProc.historyVisible" :history-list="spProc.historyList" />

    <!-- 当前生效价目对话框（判据 = sales_price_service.rs::list_strategies：approved + 有效期覆盖今天；
         /strategies 端点保留待单独授权，返回实为 sales_price::Model 分页 ⇒ 列按真实字段呈现；价格/状态走 spFmts 统一格式化） -->
    <el-dialog
      :model-value="spProc.strategyVisible"
      :title="t('salesPrice.index.effectiveDialogTitle')"
      width="800px"
      :aria-label="t('salesPrice.index.effectiveDialogAriaLabel')"
      @update:model-value="(v: boolean) => (spProc.strategyVisible = v)"
    >
      <el-table
        v-loading="spProc.strategyLoading"
        :data="spProc.strategyList"
        border
        :aria-label="t('salesPrice.index.effectiveTableAriaLabel')"
      >
        <el-table-column
          prop="product_id"
          :label="t('salesPrice.index.effectiveColumnProductId')"
          width="100"
          align="center"
        />
        <el-table-column
          prop="price"
          :label="t('salesPrice.table.columnPrice')"
          width="140"
          align="right"
        >
          <template #default="{ row }">{{ formatCurrency(row.price) }}</template>
        </el-table-column>
        <el-table-column
          prop="currency"
          :label="t('salesPrice.table.columnCurrency')"
          width="80"
          align="center"
        />
        <el-table-column
          prop="unit"
          :label="t('salesPrice.table.columnUnit')"
          width="80"
          align="center"
        />
        <el-table-column
          prop="effective_date"
          :label="t('salesPrice.table.columnEffectiveDate')"
          width="120"
          align="center"
        />
        <el-table-column
          prop="expiry_date"
          :label="t('salesPrice.table.columnExpiryDate')"
          width="120"
          align="center"
        />
        <el-table-column
          prop="status"
          :label="t('salesPrice.table.columnStatus')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { Plus, Setting, Download } from '@element-plus/icons-vue';
import type { SalesPrice } from '@/api/sales-price';
// 当前生效价目对话框的价格/状态走 spFmts 统一格式化（未知 token 抛错，不兜底；词表权威 models/status/sales.rs::price_approval）
import { formatCurrency, getStatusType, getStatusLabel } from './composables/spFmts';
import { useSp } from './composables/useSp';
import { useSpProc } from './composables/useSpProc';
import SalesPriceFilter from './components/SalesPriceFilter.vue';
import SalesPriceTable from './components/SalesPriceTable.vue';
import SalesPriceForm from './components/SalesPriceForm.vue';
import SalesPriceView from './components/SalesPriceView.vue';
import SalesPriceHistory from './components/SalesPriceHistory.vue';

const { t } = useI18n({ useScope: 'global' });

const sp = useSp();
const spProc = useSpProc({
  getList: sp.getList,
  // V15 P0-S12 修复（Batch 475d）：传入当前筛选条件，用于后端导出
  // useTableApi 的 queryParams 为 Ref<Record<string, unknown>>，需类型断言以满足回调返回类型
  getQueryParams: () => ({
    product_id: sp.queryParams.product_id as number | undefined,
    status: sp.queryParams.status as string | undefined,
  }),
});

// 对话框可见性本地 ref
const dialogVisible = ref(false);

/** 新建价格 */
const onCreate = () => {
  sp.prepareCreate();
  dialogVisible.value = true;
};

/** 编辑价格 */
const onEdit = (row: SalesPrice) => {
  sp.prepareEdit(row);
  dialogVisible.value = true;
};

/** 提交表单 */
const onSubmitForm = async () => {
  const ok = await sp.handleSubmitForm();
  if (ok) dialogVisible.value = false;
};

/** 导出当前列表（V15 P0-S12 修复 Batch 475d：改用后端导出，返回 Promise 由 Vue 事件系统处理） */
const onExport = () => spProc.handleExport();

// 列表由 useTableApi setup 自动加载，onMounted 仅加载辅助数据
onMounted(() => {
  sp.getCustomers();
  sp.getProducts();
});
</script>

<style scoped>
.sales-price-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.page-title {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 10px;
}
</style>

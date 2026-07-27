<!--
  SalesPriceTable.vue - 销售价格列表表格
  拆分自 sales-price/index.vue（P14 批 2 I-3 第 3 批）
  批次 284：接入 useTableApi 模式（page/pageSize props + v-model 绑定分页）
-->
<template>
  <el-card shadow="hover" class="table-card">
    <el-table
      v-loading="loading"
      :data="priceList"
      border
      stripe
      :aria-label="t('salesPrice.table.ariaLabel')"
    >
      <el-table-column
        type="index"
        :label="t('salesPrice.table.columnIndex')"
        width="60"
        align="center"
      />
      <el-table-column
        prop="product_name"
        :label="t('salesPrice.table.columnProductName')"
        min-width="150"
        show-overflow-tooltip
      />
      <el-table-column
        prop="customer_name"
        :label="t('salesPrice.table.columnCustomer')"
        width="150"
        show-overflow-tooltip
      />
      <el-table-column
        prop="price"
        :label="t('salesPrice.table.columnPrice')"
        width="120"
        align="right"
      >
        <template #default="{ row }">
          {{ formatCurrency(row.price) }}
        </template>
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
        prop="min_order_qty"
        :label="t('salesPrice.table.columnMinOrderQty')"
        width="100"
        align="right"
      />
      <el-table-column
        prop="price_type"
        :label="t('salesPrice.table.columnPriceType')"
        width="100"
        align="center"
      >
        <template #default="{ row }">
          <el-tag>{{ getPriceTypeLabel(row.price_type) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column
        prop="price_level"
        :label="t('salesPrice.table.columnPriceLevel')"
        width="100"
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
      <el-table-column
        :label="t('salesPrice.table.columnAction')"
        width="200"
        align="center"
        fixed="right"
      >
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="emit('view', row as SalesPrice)">{{
            t('salesPrice.table.buttonView')
          }}</el-button>
          <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
          <el-button
            v-if="row.status === 'pending'"
            v-permission="PERMISSIONS.SALES_PRICE_UPDATE"
            type="primary"
            link
            size="small"
            @click="emit('edit', row as SalesPrice)"
            >{{ t('salesPrice.table.buttonEdit') }}</el-button
          >
          <el-button
            v-if="row.status === 'pending'"
            type="success"
            link
            size="small"
            @click="emit('approve', row as SalesPrice)"
            >{{ t('salesPrice.table.buttonApprove') }}</el-button
          >
          <el-button type="info" link size="small" @click="emit('history', row as SalesPrice)">{{
            t('salesPrice.table.buttonHistory')
          }}</el-button>
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-container">
      <el-pagination
        :current-page="page"
        :page-size="pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        :aria-label="t('salesPrice.table.paginationAriaLabel')"
        @update:current-page="(v: number) => emit('update:page', v)"
        @update:page-size="(v: number) => emit('update:page-size', v)"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { SalesPrice } from '@/api/sales-price';
import { formatCurrency, getStatusType } from '../composables/spFmts';
// Batch 462 P0-S24：引入权限码常量，与后端 sales-prices 资源对齐
import { PERMISSIONS } from '@/constants/permissions';

const { t } = useI18n({ useScope: 'global' });

/**
 * 销售价格列表表格组件（批次 284：page/pageSize props + v-model 绑定分页）
 */
defineProps<{
  // 列表数据
  priceList: SalesPrice[];
  // 加载状态
  loading: boolean;
  // 总数
  total: number;
  // 当前页
  page: number;
  // 每页条数
  pageSize: number;
}>();

const emit = defineEmits<{
  view: [row: SalesPrice];
  edit: [row: SalesPrice];
  approve: [row: SalesPrice];
  history: [row: SalesPrice];
  'update:page': [v: number];
  'update:page-size': [v: number];
}>();

/** 获取价格类型标签（i18n 响应式） */
const getPriceTypeLabel = (type: string) => {
  const map: Record<string, string> = {
    STANDARD: t('salesPrice.table.priceTypeStandard'),
    AGREED: t('salesPrice.table.priceTypeAgreed'),
    PROMOTION: t('salesPrice.table.priceTypePromotion'),
  };
  return map[type] || type;
};

/** 获取销售价格状态标签（i18n 响应式） */
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending: t('salesPrice.table.statusPending'),
    active: t('salesPrice.table.statusActive'),
    expired: t('salesPrice.table.statusExpired'),
    inactive: t('salesPrice.table.statusInactive'),
  };
  return map[status] || status;
};
</script>

<style scoped>
.table-card {
  margin-bottom: 20px;
}
.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
}
</style>

<!--
  PurchaseReceiptTable.vue - 采购入库列表
  拆分自 purchaseReceipt/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-table
    :data="data"
    :loading="loading"
    border
    fit
    highlight-current-row
    :aria-label="t('purchaseReceipt.table.aria.list')"
    style="width: 100%"
  >
    <el-table-column
      prop="receipt_no"
      :label="t('purchaseReceipt.table.column.receiptNo')"
      width="150"
    />
    <el-table-column
      prop="receipt_date"
      :label="t('purchaseReceipt.table.column.receiptDate')"
      width="120"
    />
    <el-table-column
      prop="purchase_order_no"
      :label="t('purchaseReceipt.table.column.purchaseOrderNo')"
      width="150"
    />
    <el-table-column
      prop="supplier_name"
      :label="t('purchaseReceipt.table.column.supplier')"
      width="150"
    />
    <el-table-column
      prop="warehouse_name"
      :label="t('purchaseReceipt.table.column.warehouse')"
      width="120"
    />
    <el-table-column
      prop="total_amount"
      :label="t('purchaseReceipt.table.column.amount')"
      width="120"
      align="right"
    >
      <template #default="scope">
        {{ (scope.row.total_amount || 0).toFixed(2) }}
      </template>
    </el-table-column>
    <el-table-column prop="status" :label="t('purchaseReceipt.table.column.status')" width="100">
      <template #default="scope">
        <span :class="['status-tag', getStatusClassFmt(scope.row.status)]">
          {{ getStatusLabelFmt(scope.row.status) }}
        </span>
      </template>
    </el-table-column>
    <el-table-column
      prop="created_by_name"
      :label="t('purchaseReceipt.table.column.createdBy')"
      width="100"
    />
    <el-table-column
      prop="created_at"
      :label="t('purchaseReceipt.table.column.createdAt')"
      width="150"
    />
    <el-table-column :label="t('purchaseReceipt.table.column.action')" width="250" align="center">
      <template #default="scope">
        <el-button
          size="small"
          :aria-label="t('purchaseReceipt.table.aria.viewDetail')"
          @click="emit('view', scope.row as PurchaseReceiptEntity)"
        >
          <el-icon><View /></el-icon>
        </el-button>
        <el-button
          v-if="scope.row.status === 'draft'"
          size="small"
          type="primary"
          :aria-label="t('purchaseReceipt.table.aria.edit')"
          @click="emit('edit', scope.row as PurchaseReceiptEntity)"
        >
          <el-icon><Edit /></el-icon>
        </el-button>
        <el-button
          v-if="scope.row.status === 'draft'"
          size="small"
          type="warning"
          @click="emit('approve', scope.row as PurchaseReceiptEntity)"
        >
          <el-icon><Check /></el-icon> {{ t('purchaseReceipt.table.button.approve') }}
        </el-button>
        <el-button
          v-if="scope.row.status === 'draft'"
          size="small"
          type="danger"
          :aria-label="t('purchaseReceipt.table.aria.delete')"
          @click="emit('delete', scope.row as PurchaseReceiptEntity)"
        >
          <el-icon><Delete /></el-icon>
        </el-button>
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
      :aria-label="t('purchaseReceipt.table.aria.pagination')"
      @update:current-page="(v: number) => emit('update:page', v)"
      @update:page-size="(v: number) => emit('update:page-size', v)"
    />
  </div>
</template>

<script setup lang="ts">
import { View, Edit, Delete, Check } from '@element-plus/icons-vue';
import { useI18n } from 'vue-i18n';
import type { PurchaseReceiptEntity } from '@/api/purchase-receipt';
import { getStatusClass, getStatusLabel } from '../composables/prcFmts';

const { t } = useI18n({ useScope: 'global' });

/**
 * 采购入库列表组件（批次 285：page/pageSize props + v-model 绑定分页）
 */
defineProps<{
  // 列表数据
  data: PurchaseReceiptEntity[];
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
  view: [row: PurchaseReceiptEntity];
  edit: [row: PurchaseReceiptEntity];
  approve: [row: PurchaseReceiptEntity];
  delete: [row: PurchaseReceiptEntity];
  'update:page': [v: number];
  'update:page-size': [v: number];
}>();

// 透传格式化函数
const getStatusClassFmt = getStatusClass;
const getStatusLabelFmt = getStatusLabel;
</script>

<style scoped>
.status-tag {
  display: inline-block;
  padding: 4px 12px;
  border-radius: 20px;
  font-size: 12px;
}
.status-draft {
  background: #f5f7fa;
  color: #909399;
}
.status-approved {
  background: #f0f9eb;
  color: #67c23a;
}
.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
}
</style>

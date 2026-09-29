<!--
  PurchaseReturnTable.vue - 采购退货列表表格
  任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
  批次 286：page/pageSize props + v-model 绑定分页
-->
<template>
  <el-card class="table-card">
    <el-table
      v-loading="loading"
      :data="tableData"
      border
      stripe
      :aria-label="t('purchaseReturn.table.aria.list')"
    >
      <el-table-column
        prop="return_no"
        :label="t('purchaseReturn.table.column.returnNo')"
        min-width="140"
      />
      <el-table-column
        prop="purchase_order_no"
        :label="t('purchaseReturn.table.column.purchaseOrderNo')"
        min-width="140"
      />
      <el-table-column
        prop="supplier_name"
        :label="t('purchaseReturn.table.column.supplier')"
        min-width="150"
      />
      <el-table-column
        prop="return_date"
        :label="t('purchaseReturn.table.column.returnDate')"
        min-width="120"
      />
      <el-table-column
        prop="total_amount"
        :label="t('purchaseReturn.table.column.returnAmount')"
        min-width="100"
      >
        <template #default="{ row }">
          <span class="amount">¥{{ row.total_amount }}</span>
        </template>
      </el-table-column>
      <el-table-column
        prop="return_status"
        :label="t('purchaseReturn.table.column.status')"
        width="100"
        align="center"
      >
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.return_status)">
            {{ getStatusText(row.return_status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column
        prop="reason_detail"
        :label="t('purchaseReturn.table.column.reason')"
        min-width="150"
        show-overflow-tooltip
      />
      <el-table-column :label="t('purchaseReturn.table.column.action')" width="250" fixed="right">
        <template #default="{ row }">
          <el-button size="small" @click="emit('view', row as PurchaseReturn)">{{
            t('purchaseReturn.table.button.view')
          }}</el-button>
          <el-button
            v-if="row.return_status === PURCHASE_RETURN_STATUS.DRAFT"
            size="small"
            type="primary"
            @click="emit('edit', row as PurchaseReturn)"
          >
            {{ t('purchaseReturn.table.button.edit') }}
          </el-button>
          <el-button
            v-if="row.return_status === PURCHASE_RETURN_STATUS.DRAFT"
            size="small"
            type="warning"
            @click="emit('submit', row as PurchaseReturn)"
          >
            {{ t('purchaseReturn.table.button.submit') }}
          </el-button>
          <el-button
            v-if="row.return_status === PURCHASE_RETURN_STATUS.SUBMITTED"
            size="small"
            type="success"
            @click="emit('approve', row as PurchaseReturn)"
          >
            {{ t('purchaseReturn.table.button.approve') }}
          </el-button>
          <el-button
            v-if="row.return_status === PURCHASE_RETURN_STATUS.DRAFT"
            size="small"
            type="danger"
            @click="emit('delete', row as PurchaseReturn)"
          >
            {{ t('purchaseReturn.table.button.delete') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-pagination
      :current-page="page"
      :page-size="pageSize"
      :total="total"
      :page-sizes="[10, 20, 50, 100]"
      layout="total, sizes, prev, pager, next, jumper"
      :aria-label="t('purchaseReturn.table.aria.pagination')"
      @update:current-page="(v: number) => emit('update:page', v)"
      @update:page-size="(v: number) => emit('update:page-size', v)"
    />
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { PurchaseReturn } from '@/api/purchase-return';
import { getStatusType, getStatusText } from '../composables/prRtnFmts';
// 行内操作的状态门槛以写入侧原值为比较对象（draft 可编辑/提交/删除，submitted 可审批）
import { PURCHASE_RETURN_STATUS } from '@/utils/purchase-return-status';

const { t } = useI18n({ useScope: 'global' });

/**
 * 采购退货列表表格组件（批次 286：page/pageSize props + v-model 绑定分页）
 */
defineProps<{
  // 表格数据
  tableData: PurchaseReturn[];
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
  view: [row: PurchaseReturn];
  edit: [row: PurchaseReturn];
  submit: [row: PurchaseReturn];
  approve: [row: PurchaseReturn];
  delete: [row: PurchaseReturn];
  'update:page': [v: number];
  'update:page-size': [v: number];
}>();
</script>

<style scoped>
.table-card {
  margin-bottom: 20px;
}
.amount {
  font-weight: 600;
  color: #f56c6c;
}
:deep(.el-pagination) {
  margin-top: 20px;
  justify-content: flex-end;
}
</style>

<!--
  ReturnsTable.vue - 销售退货列表表格
  任务编号: P14 批 2 I-3 第 7 批
  拆分原 sales-returns/index.vue 的列表表格部分
-->
<template>
  <el-table v-loading="loading" :data="list" border :aria-label="t('salesReturns.table.ariaLabel')">
    <el-table-column prop="returnNo" :label="t('salesReturns.table.columnReturnNo')" />
    <el-table-column prop="salesOrderNo" :label="t('salesReturns.table.columnSalesOrderNo')" />
    <el-table-column prop="customerName" :label="t('salesReturns.table.columnCustomerName')" />
    <el-table-column prop="returnDate" :label="t('salesReturns.table.columnReturnDate')" />
    <el-table-column prop="totalAmount" :label="t('salesReturns.table.columnReturnAmount')" />
    <el-table-column prop="status" :label="t('salesReturns.table.columnStatus')">
      <template #default="{ row }">
        <el-tag :type="getStatusType(row.status)">
          {{ getStatusLabel(row.status) }}
        </el-tag>
      </template>
    </el-table-column>
    <el-table-column :label="t('salesReturns.table.columnAction')" width="250">
      <template #default="{ row }">
        <el-button size="small" @click="emit('view', row)">{{
          t('salesReturns.table.buttonDetail')
        }}</el-button>
        <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
        <el-button
          v-permission="PERMISSIONS.SALES_RETURN_UPDATE"
          size="small"
          @click="emit('edit', row)"
          >{{ t('salesReturns.table.buttonEdit') }}</el-button
        >
        <el-button
          v-if="row.status === 'PENDING'"
          size="small"
          type="primary"
          @click="emit('approve', row)"
          >{{ t('salesReturns.table.buttonApprove') }}</el-button
        >
      </template>
    </el-table-column>
  </el-table>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { SalesReturn } from '@/api/sales-return';
import { getStatusType } from '../composables/srFmts';
// Batch 462 P0-S24：引入权限码常量，与后端 sales-returns 资源对齐
import { PERMISSIONS } from '@/constants/permissions';

const { t } = useI18n({ useScope: 'global' });

defineProps<{
  list: SalesReturn[];
  loading: boolean;
}>();

const emit = defineEmits<{
  (e: 'view', row: SalesReturn): void;
  (e: 'edit', row: SalesReturn): void;
  (e: 'approve', row: SalesReturn): void;
}>();

/** 获取退货状态标签（i18n 响应式） */
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    PENDING: t('salesReturns.table.statusPending'),
    APPROVED: t('salesReturns.table.statusApproved'),
    REJECTED: t('salesReturns.table.statusRejected'),
    COMPLETED: t('salesReturns.table.statusCompleted'),
  };
  return map[status] || status;
};
</script>

<!--
  PurchaseReturnTable.vue - 采购退货列表表格
  任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
  批次 286：page/pageSize props + v-model 绑定分页
-->
<template>
  <el-card class="table-card">
    <el-table v-loading="loading" :data="tableData" border stripe :aria-label="t('purchaseReturn.table.aria.list')">
      <el-table-column prop="returnNo" :label="t('purchaseReturn.table.column.returnNo')" min-width="140" />
      <el-table-column prop="purchaseOrderNo" :label="t('purchaseReturn.table.column.purchaseOrderNo')" min-width="140" />
      <el-table-column prop="supplierName" :label="t('purchaseReturn.table.column.supplier')" min-width="150" />
      <el-table-column prop="returnDate" :label="t('purchaseReturn.table.column.returnDate')" min-width="120" />
      <el-table-column prop="totalAmount" :label="t('purchaseReturn.table.column.returnAmount')" min-width="100">
        <template #default="{ row }">
          <span class="amount">¥{{ row.totalAmount || 0 }}</span>
        </template>
      </el-table-column>
      <el-table-column prop="status" :label="t('purchaseReturn.table.column.status')" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">
            {{ getStatusText(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="reason" :label="t('purchaseReturn.table.column.reason')" min-width="150" show-overflow-tooltip />
      <el-table-column :label="t('purchaseReturn.table.column.action')" width="250" fixed="right">
        <template #default="{ row }">
          <el-button size="small" @click="emit('view', row as PurchaseReturn)">{{ t('purchaseReturn.table.button.view') }}</el-button>
          <el-button
            v-if="row.status === 'draft'"
            size="small"
            type="primary"
            @click="emit('edit', row as PurchaseReturn)"
          >
            {{ t('purchaseReturn.table.button.edit') }}
          </el-button>
          <el-button
            v-if="row.status === 'draft'"
            size="small"
            type="warning"
            @click="emit('submit', row as PurchaseReturn)"
          >
            {{ t('purchaseReturn.table.button.submit') }}
          </el-button>
          <el-button
            v-if="row.status === 'pending'"
            size="small"
            type="success"
            @click="emit('approve', row as PurchaseReturn)"
          >
            {{ t('purchaseReturn.table.button.approve') }}
          </el-button>
          <el-button
            v-if="row.status === 'draft'"
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
      @update:current-page="(v: number) => emit('update:page', v)"
      @update:page-size="(v: number) => emit('update:page-size', v)"
      :aria-label="t('purchaseReturn.table.aria.pagination')"
    />
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { PurchaseReturn } from '@/api/purchase-return'
import { getStatusType, getStatusText } from '../composables/prRtnFmts'

const { t } = useI18n({ useScope: 'global' })

/**
 * 采购退货列表表格组件（批次 286：page/pageSize props + v-model 绑定分页）
 */
defineProps<{
  // 表格数据
  tableData: PurchaseReturn[]
  // 加载状态
  loading: boolean
  // 总数
  total: number
  // 当前页
  page: number
  // 每页条数
  pageSize: number
}>()

const emit = defineEmits<{
  view: [row: PurchaseReturn]
  edit: [row: PurchaseReturn]
  submit: [row: PurchaseReturn]
  approve: [row: PurchaseReturn]
  delete: [row: PurchaseReturn]
  'update:page': [v: number]
  'update:page-size': [v: number]
}>()
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

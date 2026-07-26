<!--
  PurchasePriceTable.vue - 采购价格列表表格
  拆分自 purchase-price/index.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover" class="table-card">
    <el-table v-loading="loading" :data="priceList" border stripe :aria-label="t('purchasePrice.table.ariaLabel')">
      <el-table-column type="index" :label="t('purchasePrice.table.column.index')" width="60" align="center" />
      <el-table-column
        prop="product_name"
        :label="t('purchasePrice.table.column.productName')"
        min-width="150"
        show-overflow-tooltip
      />
      <el-table-column prop="supplier_name" :label="t('purchasePrice.table.column.supplier')" width="150" show-overflow-tooltip />
      <el-table-column prop="price" :label="t('purchasePrice.table.column.price')" width="120" align="right">
        <template #default="{ row }">
          {{ formatCurrency(row.price) }}
        </template>
      </el-table-column>
      <el-table-column prop="currency" :label="t('purchasePrice.table.column.currency')" width="80" align="center" />
      <el-table-column prop="unit" :label="t('purchasePrice.table.column.unit')" width="80" align="center" />
      <el-table-column prop="min_order_qty" :label="t('purchasePrice.table.column.minOrderQty')" width="100" align="right" />
      <el-table-column prop="price_type" :label="t('purchasePrice.table.column.priceType')" width="100" align="center">
        <template #default="{ row }">
          <el-tag>{{ getPriceTypeLabel(row.price_type) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="effective_date" :label="t('purchasePrice.table.column.effectiveDate')" width="120" align="center" />
      <el-table-column prop="expiry_date" :label="t('purchasePrice.table.column.expiryDate')" width="120" align="center" />
      <el-table-column prop="status" :label="t('purchasePrice.table.column.status')" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('purchasePrice.table.column.action')" width="200" align="center" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="emit('view', row as PurchasePrice)"
            >{{ t('purchasePrice.table.button.view') }}</el-button
          >
          <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
          <el-button
            v-if="row.status === 'active'"
            v-permission="'purchase_price:update'"
            type="primary"
            link
            size="small"
            @click="emit('edit', row as PurchasePrice)"
            >{{ t('purchasePrice.table.button.edit') }}</el-button
          >
          <el-button
            v-if="row.status === 'active'"
            type="warning"
            link
            size="small"
            @click="emit('disable', row as PurchasePrice)"
            >{{ t('purchasePrice.table.button.disable') }}</el-button
          >
          <el-button type="info" link size="small" @click="emit('history', row as PurchasePrice)"
            >{{ t('purchasePrice.table.button.history') }}</el-button
          >
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
        @update:current-page="(v: number) => emit('update:page', v)"
        @update:page-size="(v: number) => emit('update:page-size', v)"
        :aria-label="t('purchasePrice.table.ariaLabelPagination')"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { PurchasePrice } from '@/api/purchase-price'
import { formatCurrency, getPriceTypeLabel, getStatusType, getStatusLabel } from '../composables/ppFmts'

const { t } = useI18n({ useScope: 'global' })

/**
 * 采购价格列表表格组件（批次 285：page/pageSize props + v-model 绑定分页）
 */
defineProps<{
  // 列表数据
  priceList: PurchasePrice[]
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
  view: [row: PurchasePrice]
  edit: [row: PurchasePrice]
  disable: [row: PurchasePrice]
  history: [row: PurchasePrice]
  'update:page': [v: number]
  'update:page-size': [v: number]
}>()
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

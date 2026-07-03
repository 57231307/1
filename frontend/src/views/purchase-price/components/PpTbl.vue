<!--
  PpTbl.vue - 采购价格列表表格
  拆分自 purchase-price/index.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover" class="table-card">
    <el-table v-loading="loading" :data="priceList" border stripe>
      <el-table-column type="index" label="序号" width="60" align="center" />
      <el-table-column
        prop="product_name"
        label="产品名称"
        min-width="150"
        show-overflow-tooltip
      />
      <el-table-column prop="supplier_name" label="供应商" width="150" show-overflow-tooltip />
      <el-table-column prop="price" label="采购价格" width="120" align="right">
        <template #default="{ row }">
          {{ formatCurrency(row.price) }}
        </template>
      </el-table-column>
      <el-table-column prop="currency" label="币种" width="80" align="center" />
      <el-table-column prop="unit" label="单位" width="80" align="center" />
      <el-table-column prop="min_order_qty" label="最小订购量" width="100" align="right" />
      <el-table-column prop="price_type" label="价格类型" width="100" align="center">
        <template #default="{ row }">
          <el-tag>{{ getPriceTypeLabel(row.price_type) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="effective_date" label="生效日期" width="120" align="center" />
      <el-table-column prop="expiry_date" label="到期日期" width="120" align="center" />
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="200" align="center" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="emit('view', row as any)"
            >查看</el-button
          >
          <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
          <el-button
            v-if="row.status === 'active'"
            v-permission="'purchase_price:update'"
            type="primary"
            link
            size="small"
            @click="emit('edit', row as any)"
            >编辑</el-button
          >
          <el-button
            v-if="row.status === 'active'"
            type="warning"
            link
            size="small"
            @click="emit('disable', row as any)"
            >停用</el-button
          >
          <el-button type="info" link size="small" @click="emit('history', row as any)"
            >历史</el-button
          >
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-container">
      <el-pagination
        :current-page="queryParams.page"
        :page-size="queryParams.page_size"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="emit('size-change', $event as number)"
        @current-change="emit('current-change', $event as number)"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import type { PurchasePrice } from '@/api/purchase-price'
import { formatCurrency, getPriceTypeLabel, getStatusType, getStatusLabel } from '../composables/ppFmts'

// 采购价格查询参数（分页字段）
interface PpQueryParams {
  page: number
  page_size: number
}

/**
 * 采购价格列表表格组件
 */
defineProps<{
  // 列表数据
  priceList: PurchasePrice[]
  // 加载状态
  loading: boolean
  // 总数
  total: number
  // 查询参数（用于分页）
  queryParams: PpQueryParams
}>()

const emit = defineEmits<{
  view: [row: PurchasePrice]
  edit: [row: PurchasePrice]
  disable: [row: PurchasePrice]
  history: [row: PurchasePrice]
  'size-change': [val: number]
  'current-change': [val: number]
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

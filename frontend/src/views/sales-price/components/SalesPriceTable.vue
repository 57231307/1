<!--
  SalesPriceTable.vue - 销售价格列表表格
  拆分自 sales-price/index.vue（P14 批 2 I-3 第 3 批）
  批次 284：接入 useTableApi 模式（page/pageSize props + v-model 绑定分页）
-->
<template>
  <el-card shadow="hover" class="table-card">
    <el-table v-loading="loading" :data="priceList" border stripe aria-label="销售价格列表">
      <el-table-column type="index" label="序号" width="60" align="center" />
      <el-table-column
        prop="product_name"
        label="产品名称"
        min-width="150"
        show-overflow-tooltip
      />
      <el-table-column prop="customer_name" label="客户" width="150" show-overflow-tooltip />
      <el-table-column prop="price" label="销售价格" width="120" align="right">
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
      <el-table-column prop="price_level" label="价格等级" width="100" align="center" />
      <el-table-column prop="effective_date" label="生效日期" width="120" align="center" />
      <el-table-column prop="expiry_date" label="到期日期" width="120" align="center" />
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="200" align="center" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="emit('view', row as SalesPrice)"
            >查看</el-button
          >
          <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
          <el-button
            v-if="row.status === 'pending'"
            v-permission="PERMISSIONS.SALES_PRICE_UPDATE"
            type="primary"
            link
            size="small"
            @click="emit('edit', row as SalesPrice)"
            >编辑</el-button
          >
          <el-button
            v-if="row.status === 'pending'"
            type="success"
            link
            size="small"
            @click="emit('approve', row as SalesPrice)"
            >审批</el-button
          >
          <el-button type="info" link size="small" @click="emit('history', row as SalesPrice)"
            >历史</el-button
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
        aria-label="销售价格列表分页"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import type { SalesPrice } from '@/api/sales-price'
import { formatCurrency, getPriceTypeLabel, getStatusType, getStatusLabel } from '../composables/spFmts'
// Batch 462 P0-S24：引入权限码常量，与后端 sales-prices 资源对齐
import { PERMISSIONS } from '@/constants/permissions'

/**
 * 销售价格列表表格组件（批次 284：page/pageSize props + v-model 绑定分页）
 */
defineProps<{
  // 列表数据
  priceList: SalesPrice[]
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
  view: [row: SalesPrice]
  edit: [row: SalesPrice]
  approve: [row: SalesPrice]
  history: [row: SalesPrice]
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

<!--
  PrcTbl.vue - 采购入库列表
  拆分自 purchaseReceipt/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-table
    :data="data"
    :loading="loading"
    border
    fit
    highlight-current-row
    style="width: 100%"
  >
    <el-table-column prop="receipt_no" label="入库单号" width="150" />
    <el-table-column prop="receipt_date" label="入库日期" width="120" />
    <el-table-column prop="purchase_order_no" label="采购订单号" width="150" />
    <el-table-column prop="supplier_name" label="供应商" width="150" />
    <el-table-column prop="warehouse_name" label="仓库" width="120" />
    <el-table-column prop="total_amount" label="入库金额" width="120" align="right">
      <template #default="scope">
        {{ (scope.row.total_amount || 0).toFixed(2) }}
      </template>
    </el-table-column>
    <el-table-column prop="status" label="状态" width="100">
      <template #default="scope">
        <span :class="['status-tag', getStatusClassFmt(scope.row.status)]">
          {{ getStatusLabelFmt(scope.row.status) }}
        </span>
      </template>
    </el-table-column>
    <el-table-column prop="created_by_name" label="创建人" width="100" />
    <el-table-column prop="created_at" label="创建时间" width="150" />
    <el-table-column label="操作" width="250" align="center">
      <template #default="scope">
        <el-button size="small" @click="emit('view', scope.row as any)">
          <el-icon><View /></el-icon>
        </el-button>
        <el-button
          v-if="scope.row.status === 'draft'"
          size="small"
          type="primary"
          @click="emit('edit', scope.row as any)"
        >
          <el-icon><Edit /></el-icon>
        </el-button>
        <el-button
          v-if="scope.row.status === 'draft'"
          size="small"
          type="warning"
          @click="emit('approve', scope.row as any)"
        >
          <el-icon><Check /></el-icon> 审核
        </el-button>
        <el-button
          v-if="scope.row.status === 'draft'"
          size="small"
          type="danger"
          @click="emit('delete', scope.row as any)"
        >
          <el-icon><Delete /></el-icon>
        </el-button>
      </template>
    </el-table-column>
  </el-table>

  <div class="pagination-container">
    <el-pagination
      :current-page="pagination.page"
      :page-size="pagination.pageSize"
      :page-sizes="[10, 20, 50, 100]"
      :total="total"
      layout="total, sizes, prev, pager, next, jumper"
      @size-change="emit('size-change', $event as number)"
      @current-change="emit('current-change', $event as number)"
    />
  </div>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { View, Edit, Delete, Check } from '@element-plus/icons-vue'
import type { PurchaseReceiptEntity } from '@/api/purchaseReceipt'
import { getStatusClass, getStatusLabel } from '../composables/prcFmts'

// 分页类型
interface Pgn {
  page: number
  pageSize: number
}

/**
 * 采购入库列表组件
 */
defineProps<{
  // 列表数据
  data: PurchaseReceiptEntity[]
  // 加载状态
  loading: boolean
  // 总数
  total: number
  // 分页
  pagination: Pgn
}>()

const emit = defineEmits<{
  view: [row: PurchaseReceiptEntity]
  edit: [row: PurchaseReceiptEntity]
  approve: [row: PurchaseReceiptEntity]
  delete: [row: PurchaseReceiptEntity]
  'size-change': [val: number]
  'current-change': [val: number]
}>()

// 透传格式化函数
const getStatusClassFmt = getStatusClass
const getStatusLabelFmt = getStatusLabel
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

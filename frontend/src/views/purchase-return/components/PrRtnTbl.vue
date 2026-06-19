<!-- eslint-disable vue/no-mutating-props -->
<!--
  PrRtnTbl.vue - 采购退货列表表格
  任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
-->
<template>
  <el-card class="table-card">
    <el-table v-loading="loading" :data="tableData" border stripe>
      <el-table-column prop="returnNo" label="退货单号" min-width="140" />
      <el-table-column prop="purchaseOrderNo" label="采购单号" min-width="140" />
      <el-table-column prop="supplierName" label="供应商" min-width="150" />
      <el-table-column prop="returnDate" label="退货日期" min-width="120" />
      <el-table-column prop="totalAmount" label="退货金额" min-width="100">
        <template #default="{ row }">
          <span class="amount">¥{{ row.totalAmount || 0 }}</span>
        </template>
      </el-table-column>
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">
            {{ getStatusText(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="reason" label="退货原因" min-width="150" show-overflow-tooltip />
      <el-table-column label="操作" width="250" fixed="right">
        <template #default="{ row }">
          <el-button size="small" @click="emit('view', row)">查看</el-button>
          <el-button
            v-if="row.status === 'draft'"
            size="small"
            type="primary"
            @click="emit('edit', row)"
          >
            编辑
          </el-button>
          <el-button
            v-if="row.status === 'draft'"
            size="small"
            type="warning"
            @click="emit('submit', row)"
          >
            提交
          </el-button>
          <el-button
            v-if="row.status === 'pending'"
            size="small"
            type="success"
            @click="emit('approve', row)"
          >
            审批
          </el-button>
          <el-button
            v-if="row.status === 'draft'"
            size="small"
            type="danger"
            @click="emit('delete', row)"
          >
            删除
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-pagination
      v-model:current-page="queryParams.page"
      v-model:page-size="queryParams.pageSize"
      :total="total"
      :page-sizes="[10, 20, 50, 100]"
      layout="total, sizes, prev, pager, next, jumper"
      @size-change="emit('size-change')"
      @current-change="emit('current-change')"
    />
  </el-card>
</template>

<script setup lang="ts">
import type { PurchaseReturn } from '@/api/purchase-return'
import { getStatusType, getStatusText } from '../composables/prRtnFmts'

// 采购退货查询参数
interface QueryParams {
  page: number
  pageSize: number
  keyword: string
  supplierId: number | undefined
  status: string
}

// 采购退货列表表格属性
defineProps<{
  // 表格数据
  tableData: PurchaseReturn[]
  // 加载状态
  loading: boolean
  // 总数
  total: number
  // 查询参数（用于分页）
  queryParams: QueryParams
}>()

// 定义事件
const emit = defineEmits<{
  // 查看
  (e: 'view', row: PurchaseReturn): void
  // 编辑
  (e: 'edit', row: PurchaseReturn): void
  // 提交
  (e: 'submit', row: PurchaseReturn): void
  // 审批
  (e: 'approve', row: PurchaseReturn): void
  // 删除
  (e: 'delete', row: PurchaseReturn): void
  // 分页 - 每页大小
  (e: 'size-change'): void
  // 分页 - 当前页
  (e: 'current-change'): void
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

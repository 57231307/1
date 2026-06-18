<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
/**
 * PurchTbl - 采购管理数据表格（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 主表格）
 */
import type { PurchaseOrder } from '@/api/purchase'

interface Props {
  orders: PurchaseOrder[]
  loading: boolean
  total: number
  queryParams: {
    page: number
    page_size: number
    keyword: string
    supplier_id: number | undefined
    status: string
  }
  onView: (row: PurchaseOrder) => void
  onApprove: (row: PurchaseOrder) => void
  onReceive: (row: PurchaseOrder) => void
  onQuery: () => void
  getStatusType: (s: string) => string
  getStatusText: (s: string) => string
  getPaymentStatusType: (s: string) => string
  getPaymentStatusText: (s: string) => string
}

defineProps<Props>()
</script>

<template>
  <el-card shadow="hover" class="table-card">
    <el-table v-loading="loading" :data="orders" stripe>
      <el-table-column prop="order_no" label="订单号" width="160" fixed>
        <template #default="{ row }">
          <el-link type="primary" @click="onView(row as any)">{{ row.order_no }}</el-link>
        </template>
      </el-table-column>
      <el-table-column prop="supplier_name" label="供应商" width="180" fixed />
      <el-table-column prop="order_date" label="订单日期" width="120" />
      <el-table-column prop="required_date" label="要求交货日期" width="120" />
      <el-table-column prop="total_amount" label="订单金额" width="120" align="right">
        <template #default="{ row }">
          <span class="amount">¥{{ row.total_amount.toLocaleString() }}</span>
        </template>
      </el-table-column>
      <el-table-column prop="received_amount" label="已收货金额" width="120" align="right">
        <template #default="{ row }">
          <span>¥{{ (row.received_amount || 0).toLocaleString() }}</span>
        </template>
      </el-table-column>
      <el-table-column prop="payment_status" label="付款状态" width="100">
        <template #default="{ row }">
          <el-tag :type="getPaymentStatusType(row.payment_status)" size="small">
            {{ getPaymentStatusText(row.payment_status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="status" label="订单状态" width="100">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)" size="small">
            {{ getStatusText(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="creator_name" label="创建人" width="100" />
      <el-table-column label="操作" width="200" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="onView(row as any)"
            >详情</el-button
          >
          <el-button
            v-if="row.status === 'approved'"
            type="warning"
            link
            size="small"
            @click="onReceive(row as any)"
            >收货</el-button
          >
          <el-button
            v-if="row.status === 'pending'"
            type="success"
            link
            size="small"
            @click="onApprove(row as any)"
            >审批</el-button
          >
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-wrapper">
      <el-pagination
        v-model:current-page="queryParams.page"
        v-model:page-size="queryParams.page_size"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="onQuery"
        @current-change="onQuery"
      />
    </div>
  </el-card>
</template>

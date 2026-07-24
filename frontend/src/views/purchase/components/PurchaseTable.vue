<!--
  PurchaseTable - 采购管理数据表格（纯展示）
  任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 主表格）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
-->
<template>
  <el-card shadow="hover" class="table-card">
    <el-table v-loading="loading" :data="orders" stripe aria-label="采购订单列表">
      <el-table-column prop="order_no" label="订单号" width="160" fixed>
        <template #default="{ row }">
          <el-link type="primary" @click="onView(row as PurchaseOrder)">{{ row.order_no }}</el-link>
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
          <el-button type="primary" link size="small" @click="onView(row as PurchaseOrder)"
            >详情</el-button
          >
          <el-button
            v-if="row.status === 'approved'"
            v-permission="PERMISSIONS.PURCHASE_ORDER_RECEIVE"
            type="warning"
            link
            size="small"
            @click="onReceive(row as PurchaseOrder)"
            >收货</el-button
          >
          <el-button
            v-if="row.status === 'pending'"
            v-permission="PERMISSIONS.PURCHASE_ORDER_APPROVE"
            type="success"
            link
            size="small"
            @click="onApprove(row as PurchaseOrder)"
            >审批</el-button
          >
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-wrapper">
      <el-pagination
        v-model:current-page="localQueryParams.page"
        v-model:page-size="localQueryParams.page_size"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        aria-label="采购订单列表分页"
        @size-change="onQuery"
        @current-change="onQuery"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import type { PurchaseOrder } from '@/api/purchase'
// Batch 468 P0-S28：引入权限码常量，与后端 purchase-orders 资源对齐
import { PERMISSIONS } from '@/constants/permissions'

interface QueryParams {
  page: number
  page_size: number
  keyword: string
  supplier_id: number | undefined
  status: string
}

const props = defineProps<{
  // 采购订单列表
  orders: PurchaseOrder[]
  // 加载状态
  loading: boolean
  // 总数
  total: number
  // 查询参数（分页相关，由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: QueryParams
  // 查看
  onView: (row: PurchaseOrder) => void
  // 审批
  onApprove: (row: PurchaseOrder) => void
  // 收货
  onReceive: (row: PurchaseOrder) => void
  // 查询回调
  onQuery: () => void
  // 状态类型
  getStatusType: (s: string) => string
  // 状态文本
  getStatusText: (s: string) => string
  // 付款状态类型
  getPaymentStatusType: (s: string) => string
  // 付款状态文本
  getPaymentStatusText: (s: string) => string
}>()

const emit = defineEmits<{
  // 整体回写查询参数（父组件监听此事件并 Object.assign 到自己的 queryParams）
  (e: 'update:queryParams', queryParams: QueryParams): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localQueryParams = ref<QueryParams>({ ...props.queryParams })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件重置）
watch(
  () => props.queryParams,
  (newParams) => {
    if (syncing) return
    syncing = true
    localQueryParams.value = { ...newParams }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户分页）
watch(
  localQueryParams,
  (newParams) => {
    if (syncing) return
    syncing = true
    emit('update:queryParams', { ...newParams })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)
</script>

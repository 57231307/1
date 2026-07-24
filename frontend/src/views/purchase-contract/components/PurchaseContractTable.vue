<!--
  PurchaseContractTable.vue - 采购合同列表表格
  拆分自 purchase-contract/index.vue（P14 批 2 I-3 第 3 批）
  批次 284：接入 useTableApi 模式（page/pageSize props + v-model 绑定分页）
-->
<template>
  <el-card shadow="hover" class="table-card">
    <el-table v-loading="loading" :data="contractList" border stripe aria-label="采购合同列表">
      <el-table-column type="index" label="序号" width="60" align="center" />
      <el-table-column prop="contract_no" label="合同编号" width="150" show-overflow-tooltip />
      <el-table-column
        prop="contract_name"
        label="合同名称"
        min-width="200"
        show-overflow-tooltip
      />
      <el-table-column prop="supplier_name" label="供应商" width="150" show-overflow-tooltip />
      <el-table-column prop="total_amount" label="合同金额" width="120" align="right">
        <template #default="{ row }">
          {{ formatCurrency(row.total_amount) }}
        </template>
      </el-table-column>
      <el-table-column prop="signed_date" label="签订日期" width="120" align="center" />
      <el-table-column prop="effective_date" label="生效日期" width="120" align="center" />
      <el-table-column prop="expiry_date" label="到期日期" width="120" align="center" />
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="250" align="center" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="emit('view', row as PurchaseContract)"
            >查看</el-button
          >
          <!-- P2-17 修复（批次 86 v2 复审）：编辑/删除按钮补齐 v-permission -->
          <el-button
            v-if="row.status === 'draft'"
            v-permission="'purchase_contract:update'"
            type="primary"
            link
            size="small"
            @click="emit('edit', row as PurchaseContract)"
            >编辑</el-button
          >
          <el-button
            v-if="row.status === 'draft'"
            type="success"
            link
            size="small"
            @click="emit('submit', row as PurchaseContract)"
            >提交</el-button
          >
          <el-button
            v-if="row.status === 'pending'"
            type="success"
            link
            size="small"
            @click="emit('approve', row as PurchaseContract)"
            >审批</el-button
          >
          <el-button
            v-if="row.status === 'active'"
            type="warning"
            link
            size="small"
            @click="emit('execute', row as PurchaseContract)"
            >执行</el-button
          >
          <el-button
            v-if="row.status === 'draft'"
            v-permission="'purchase_contract:delete'"
            type="danger"
            link
            size="small"
            @click="emit('delete', row as PurchaseContract)"
            >删除</el-button
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
        aria-label="采购合同列表分页"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import type { PurchaseContract } from '@/api/purchase-contract'
import { formatCurrency, getStatusType, getStatusLabel } from '../composables/pcFmts'

/**
 * 采购合同列表表格组件（批次 284：page/pageSize props + v-model 绑定分页）
 */
defineProps<{
  // 列表数据
  contractList: PurchaseContract[]
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
  view: [row: PurchaseContract]
  edit: [row: PurchaseContract]
  submit: [row: PurchaseContract]
  approve: [row: PurchaseContract]
  execute: [row: PurchaseContract]
  delete: [row: PurchaseContract]
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

<!--
  VchrTbl.vue - 凭证列表表格
  拆分自 VoucherTab.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <el-table v-loading="voucherLoading" :data="vouchers" stripe>
      <el-table-column prop="voucher_no" label="凭证号" width="120" />
      <el-table-column prop="voucher_date" label="凭证日期" width="120" />
      <el-table-column prop="voucher_type" label="凭证类型" width="100" />
      <el-table-column label="借方金额" width="120" align="right">
        <template #default="{ row }">
          {{ formatMoney(row.total_debit) }}
        </template>
      </el-table-column>
      <el-table-column label="贷方金额" width="120" align="right">
        <template #default="{ row }">
          {{ formatMoney(row.total_credit) }}
        </template>
      </el-table-column>
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getVoucherStatusType(row.status)" size="small">
            {{ getVoucherStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="created_by_name" label="制单人" width="100" />
      <el-table-column prop="created_at" label="创建时间" width="160" />
      <el-table-column label="操作" width="200" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="emit('view', row)">查看</el-button>
          <el-button
            v-if="row.status === 'draft'"
            type="primary"
            link
            size="small"
            @click="emit('submit', row)"
            >提交</el-button
          >
          <el-button
            v-if="row.status === 'submitted'"
            type="success"
            link
            size="small"
            @click="emit('review', row)"
            >审核</el-button
          >
          <el-button
            v-if="row.status === 'reviewed'"
            type="warning"
            link
            size="small"
            @click="emit('post', row)"
            >过账</el-button
          >
        </template>
      </el-table-column>
    </el-table>
    <div class="pagination-wrapper">
      <el-pagination
        :current-page="voucherQueryParams.page"
        :page-size="voucherQueryParams.page_size"
        :page-sizes="[10, 20, 50, 100]"
        :total="voucherTotal"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="emit('page-change')"
        @current-change="emit('page-change')"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import type { Voucher } from '@/api/finance'

/**
 * 凭证列表表格组件
 * 仅做展示，行内操作通过 emit 通知父组件
 */
const props = defineProps<{
  vouchers: Voucher[]
  voucherLoading: boolean
  voucherTotal: number
  voucherQueryParams: { page: number; page_size: number }
  formatMoney: (amount: number) => string
  getVoucherStatusLabel: (status?: string) => string
  getVoucherStatusType: (status?: string) => string
}>()

// 查看凭证 / 提交凭证 / 审核凭证 / 过账凭证 / 分页变化（触发 fetchVouchers）
const emit = defineEmits<{
  view: [row: Voucher]
  submit: [row: Voucher]
  review: [row: Voucher]
  post: [row: Voucher]
  'page-change': []
}>()

void props
</script>

<style scoped>
.pagination-wrapper {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>

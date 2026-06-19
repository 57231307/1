<!--
  VchrDetail.vue - 凭证详情对话框
  拆分自 VoucherTab.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="凭证详情"
    width="800px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-descriptions :column="3" border>
      <el-descriptions-item label="凭证号">{{ currentVoucher?.voucher_no }}</el-descriptions-item>
      <el-descriptions-item label="凭证日期">{{
        currentVoucher?.voucher_date
      }}</el-descriptions-item>
      <el-descriptions-item label="凭证类型">{{
        currentVoucher?.voucher_type
      }}</el-descriptions-item>
      <el-descriptions-item label="状态">
        <el-tag :type="getVoucherStatusType(currentVoucher?.status)">
          {{ getVoucherStatusLabel(currentVoucher?.status) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item label="制单人">{{
        currentVoucher?.created_by_name
      }}</el-descriptions-item>
      <el-descriptions-item label="创建时间">{{
        currentVoucher?.created_at
      }}</el-descriptions-item>
    </el-descriptions>
    <el-divider>分录明细</el-divider>
    <el-table :data="currentVoucher?.entries" stripe>
      <el-table-column prop="summary" label="摘要" min-width="150" />
      <el-table-column prop="subject_code" label="科目编码" width="100" />
      <el-table-column prop="subject_name" label="科目名称" min-width="150" />
      <el-table-column label="借方" width="120" align="right">
        <template #default="{ row }">
          {{ row.debit ? formatMoney(row.debit) : '' }}
        </template>
      </el-table-column>
      <el-table-column label="贷方" width="120" align="right">
        <template #default="{ row }">
          {{ row.credit ? formatMoney(row.credit) : '' }}
        </template>
      </el-table-column>
    </el-table>
    <div class="entry-footer">
      <span>借方合计: {{ formatMoney(currentVoucher?.total_debit || 0) }}</span>
      <span>贷方合计: {{ formatMoney(currentVoucher?.total_credit || 0) }}</span>
    </div>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import type { Voucher } from '@/api/finance'

/**
 * 凭证详情对话框组件
 * 仅做展示，对话框状态由父组件控制
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 当前凭证
  currentVoucher: Voucher | null
  // 金额格式化
  formatMoney: (amount: number) => string
  // 状态标签
  getVoucherStatusLabel: (status?: string) => string
  // 状态类型
  getVoucherStatusType: (status?: string) => string
}>()

const emit = defineEmits<{
  // 关闭对话框
  'update:visible': [v: boolean]
}>()

void props
</script>

<style scoped>
.entry-footer {
  display: flex;
  justify-content: flex-end;
  gap: 16px;
  margin-top: 12px;
  font-size: 13px;
  color: #606266;
}
</style>

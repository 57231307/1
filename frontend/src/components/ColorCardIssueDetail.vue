<template>
  <!--
    色卡发放详情展示组件（V15 P0-F12）
    展示单条发放记录的全字段，供详情对话框/抽屉使用
  -->
  <el-descriptions :column="2" border>
    <el-descriptions-item label="记录 ID">{{ record.id }}</el-descriptions-item>
    <el-descriptions-item label="色卡 ID">{{ record.color_card_id }}</el-descriptions-item>
    <el-descriptions-item label="客户 ID">{{ record.customer_id }}</el-descriptions-item>
    <el-descriptions-item label="发放数量">{{ record.issue_qty }}</el-descriptions-item>
    <el-descriptions-item label="经办人">{{ record.issued_by }}</el-descriptions-item>
    <el-descriptions-item label="发放时间">{{ formatDate(record.issued_at) }}</el-descriptions-item>
    <el-descriptions-item label="预计归还">{{ formatDate(record.expected_return_date) }}</el-descriptions-item>
    <el-descriptions-item label="实际归还">{{ formatDate(record.actual_return_date) }}</el-descriptions-item>
    <el-descriptions-item label="状态">
      <el-tag :type="statusColor">{{ statusLabel }}</el-tag>
    </el-descriptions-item>
    <el-descriptions-item label="染色批号">{{ record.dye_lot_no || '-' }}</el-descriptions-item>
    <el-descriptions-item label="用途" :span="2">{{ record.purpose || '-' }}</el-descriptions-item>
    <el-descriptions-item label="备注" :span="2">{{ record.remark || '-' }}</el-descriptions-item>
    <el-descriptions-item label="赔付金额">
      {{ record.compensation_amount != null ? `¥${record.compensation_amount.toFixed(2)}` : '-' }}
    </el-descriptions-item>
    <el-descriptions-item label="归还经办人">{{ record.returned_by ?? '-' }}</el-descriptions-item>
    <el-descriptions-item label="创建时间">{{ formatDate(record.created_at) }}</el-descriptions-item>
    <el-descriptions-item label="更新时间">{{ formatDate(record.updated_at) }}</el-descriptions-item>
  </el-descriptions>
</template>

<script setup lang="ts">
// 色卡发放详情展示组件（V15 P0-F12）
// 创建时间：2026-07-18（Batch 477 P0-F12）

import { computed } from 'vue'
import type { IssueRecordInfo, IssueStatusValue } from '@/types/colorCardIssue'

const props = defineProps<{
  record: IssueRecordInfo
}>()

// 发放状态标签映射
const STATUS_LABELS: Record<IssueStatusValue, string> = {
  issued: '发放中',
  returned: '已归还',
  lost: '遗失',
  damaged: '损坏',
  cancelled: '已取消',
}

// 发放状态颜色映射（Element Plus Tag type）
const STATUS_COLORS: Record<IssueStatusValue, 'warning' | 'success' | 'danger' | 'info'> = {
  issued: 'warning',
  returned: 'success',
  lost: 'danger',
  damaged: 'danger',
  cancelled: 'info',
}

const statusLabel = computed(() => STATUS_LABELS[props.record.status] || props.record.status)
const statusColor = computed(() => STATUS_COLORS[props.record.status] || 'info')

const formatDate = (s?: string): string => {
  if (!s) return '-'
  try {
    return new Date(s).toLocaleString('zh-CN')
  } catch {
    return s
  }
}
</script>

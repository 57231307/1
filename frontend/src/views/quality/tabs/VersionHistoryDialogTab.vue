<!--
  VersionHistoryDialogTab.vue - 版本历史对话框
  来源：原 quality/index.vue 中 版本历史对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    title="版本历史"
    width="800px"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-table :data="historyList" stripe>
      <el-table-column prop="version" label="版本" width="100" />
      <el-table-column prop="status" label="状态" width="100">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)" size="small">
            {{ getStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="created_by_name" label="创建人" width="100" />
      <el-table-column prop="created_at" label="创建时间" width="160" />
      <el-table-column prop="approved_by_name" label="审批人" width="100">
        <template #default="{ row }">
          {{ row.approved_by_name || '-' }}
        </template>
      </el-table-column>
      <el-table-column prop="approved_at" label="审批时间" width="160">
        <template #default="{ row }">
          {{ row.approved_at || '-' }}
        </template>
      </el-table-column>
    </el-table>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">关闭</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import type { QualityStandard } from '@/api/quality'

interface Props {
  modelValue: boolean
  historyList: QualityStandard[]
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
}

defineProps<Props>()
const emit = defineEmits<Emits>()

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    approved: '已审批',
    published: '已发布',
    rejected: '已驳回',
  }
  return map[status] || status
}

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    approved: 'warning',
    published: 'success',
    rejected: 'danger',
  }
  return map[status] || 'info'
}
</script>

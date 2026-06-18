<script setup lang="ts">
/**
 * TplSub - 报表订阅管理对话框（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 report/templates.vue 订阅管理对话框）
 */
import { Plus, Edit, Delete } from '@element-plus/icons-vue'
import type { ReportSubscription } from '@/api/report-enhanced'

interface Props {
  modelValue: boolean
  templateName: string
  subscriptions: ReportSubscription[]
  onCreate: () => void
  onEdit: (row: ReportSubscription) => void
  onToggle: (row: ReportSubscription) => void
  onSendNow: (row: ReportSubscription) => void
  onDelete: (row: ReportSubscription) => void
  getScheduleLabel: (s: string) => string
  getFormatLabel: (f: string) => string
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    title="订阅管理"
    width="900px"
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <div class="sub-header">
      <span>模板：{{ templateName }}</span>
      <el-button type="primary" size="small" @click="onCreate">
        <el-icon><Plus /></el-icon> 新建订阅
      </el-button>
    </div>
    <el-table :data="subscriptions" border style="width: 100%; margin-top: 16px">
      <el-table-column label="频率" width="80">
        <template #default="scope">{{ getScheduleLabel(scope.row.schedule) }}</template>
      </el-table-column>
      <el-table-column prop="schedule_time" label="发送时间" width="100" />
      <el-table-column label="接收人" min-width="200">
        <template #default="scope">
          <el-tag
            v-for="(r, i) in scope.row.recipients"
            :key="i"
            size="small"
            style="margin-right: 4px"
          >
            {{ r }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="格式" width="120">
        <template #default="scope">{{ getFormatLabel(scope.row.format) }}</template>
      </el-table-column>
      <el-table-column label="状态" width="80">
        <template #default="scope">
          <el-tag size="small" :type="scope.row.active ? 'success' : 'info'">
            {{ scope.row.active ? '启用' : '禁用' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="last_sent_at" label="最后发送" width="160" />
      <el-table-column label="操作" width="200" align="center">
        <template #default="scope">
          <el-button size="small" @click="onEdit(scope.row as any)">
            <el-icon><Edit /></el-icon>
          </el-button>
          <el-button
            size="small"
            :type="scope.row.active ? 'warning' : 'success'"
            @click="onToggle(scope.row as any)"
          >
            {{ scope.row.active ? '禁用' : '启用' }}
          </el-button>
          <el-button size="small" type="success" @click="onSendNow(scope.row as any)"
            >发送</el-button
          >
          <el-button
            size="small"
            type="danger"
            @click="onDelete(scope.row as any)"
          >
            <el-icon><Delete /></el-icon>
          </el-button>
        </template>
      </el-table-column>
    </el-table>
  </el-dialog>
</template>

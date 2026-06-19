<!--
  BpmApAprDlg.vue - BPM 审批对话框（同意/拒绝）
  拆分自 bpm/approval.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    :title="action === 'approve' ? '审批通过' : '审批拒绝'"
    width="500px"
    destroy-on-close
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form :model="approveForm" label-width="80px">
      <el-form-item label="任务名称">
        <span>{{ currentTask?.task_name }}</span>
      </el-form-item>
      <el-form-item label="审批意见">
        <el-input
          v-model="approveForm.comment"
          type="textarea"
          :rows="4"
          placeholder="请输入审批意见"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button
        :type="action === 'approve' ? 'success' : 'danger'"
        :loading="submitLoading"
        @click="emit('confirm')"
        >确定</el-button
      >
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import type { ApprovalTask } from '@/api/bpm-enhanced'

// 表单字段类型
interface AprForm {
  comment: string
}

/**
 * 审批对话框组件（同意/拒绝共用）
 */
defineProps<{
  // 对话框可见性
  visible: boolean
  // 当前任务
  currentTask: ApprovalTask | null
  // 审批动作
  action: 'approve' | 'reject'
  // 提交 loading
  submitLoading: boolean
  // 表单数据
  approveForm: AprForm
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  confirm: []
}>()
</script>

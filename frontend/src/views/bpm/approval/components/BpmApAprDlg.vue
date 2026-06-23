<!--
  BpmApAprDlg.vue - BPM 审批对话框（同意/拒绝）
  拆分自 bpm/approval.vue（P14 批 2 I-3 第 4 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="action === 'approve' ? '审批通过' : '审批拒绝'"
    width="500px"
    destroy-on-close
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form :model="localApproveForm" label-width="80px">
      <el-form-item label="任务名称">
        <span>{{ currentTask?.task_name }}</span>
      </el-form-item>
      <el-form-item label="审批意见">
        <el-input
          v-model="localApproveForm.comment"
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
import { ref, watch, nextTick } from 'vue'
import type { ApprovalTask } from '@/api/bpm-enhanced'

// 表单字段类型
interface AprForm {
  comment: string
}

/**
 * 审批对话框组件（同意/拒绝共用）
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 当前任务
  currentTask: ApprovalTask | null
  // 审批动作
  action: 'approve' | 'reject'
  // 提交 loading
  submitLoading: boolean
  // 表单数据（由父组件管理，子组件通过 emit 回写）
  approveForm: AprForm
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  // 整体回写表单（父组件监听此事件并 Object.assign 到自己的 approveForm）
  'update:approveForm': [v: AprForm]
  confirm: []
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localApproveForm = ref<AprForm>({ ...props.approveForm })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件打开对话框时填充数据）
watch(
  () => props.approveForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    localApproveForm.value = { ...newForm }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localApproveForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    emit('update:approveForm', { ...newForm })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)
</script>

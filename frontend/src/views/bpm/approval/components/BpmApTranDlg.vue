<!--
  BpmApTranDlg.vue - BPM 转交任务对话框
  拆分自 bpm/approval.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    title="转交任务"
    width="500px"
    destroy-on-close
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="100px"
    >
      <el-form-item label="任务名称">
        <span>{{ currentTask?.task_name }}</span>
      </el-form-item>
      <el-form-item label="接收人 ID" prop="target_user_id">
        <el-input-number
          :model-value="form.target_user_id"
          :min="1"
          style="width: 100%"
          @update:model-value="(v: number) => (form.target_user_id = v ?? 1)"
        />
      </el-form-item>
      <el-form-item label="转交原因">
        <el-input
          v-model="form.comment"
          type="textarea"
          :rows="3"
          placeholder="请输入转交原因"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="onConfirm">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { ref } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import type { ApprovalTask } from '@/api/bpm-enhanced'

// 表单字段类型
interface TranForm {
  target_user_id: number
  comment: string
}

/**
 * 转交任务对话框组件
 */
defineProps<{
  // 对话框可见性
  visible: boolean
  // 当前任务
  currentTask: ApprovalTask | null
  // 提交 loading
  submitLoading: boolean
  // 表单数据
  form: TranForm
  // 校验规则
  rules: FormRules
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  confirm: []
}>()

// 内部表单 ref
const formRef = ref<FormInstance>()

/** 点击确定：先校验再发 confirm */
const onConfirm = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async (valid: boolean) => {
    if (!valid) return
    emit('confirm')
  })
}
</script>

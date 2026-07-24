<!--
  BpmApprovalTransferDialog.vue - BPM 转交任务对话框
  拆分自 bpm/approval.vue（P14 批 2 I-3 第 4 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :aria-label="$t('bpm.approval.transferDialog.ariaLabel')"
    :model-value="visible"
    :title="$t('bpm.approval.transferDialog.title')"
    width="500px"
    destroy-on-close
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      ref="formRef"
      :model="localForm"
      :rules="rules"
      label-width="100px"
      :aria-label="$t('bpm.approval.transferDialog.formAriaLabel')"
    >
      <el-form-item :label="$t('bpm.approval.transferDialog.taskName')">
        <span>{{ currentTask?.task_name }}</span>
      </el-form-item>
      <el-form-item :label="$t('bpm.approval.transferDialog.targetUserId')" prop="target_user_id">
        <el-input-number
          v-model="localForm.target_user_id"
          :min="1"
          style="width: 100%"
        />
      </el-form-item>
      <el-form-item :label="$t('bpm.approval.transferDialog.comment')">
        <el-input
          v-model="localForm.comment"
          type="textarea"
          :rows="3"
          :placeholder="$t('bpm.approval.transferDialog.commentPlaceholder')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{ $t('bpm.approval.transferDialog.cancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="onConfirm">{{ $t('bpm.approval.transferDialog.confirm') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
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
const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 当前任务
  currentTask: ApprovalTask | null
  // 提交 loading
  submitLoading: boolean
  // 表单数据（由父组件管理，子组件通过 emit 回写）
  form: TranForm
  // 校验规则
  rules: FormRules
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  // 整体回写表单（父组件监听此事件并 Object.assign 到自己的 form）
  'update:form': [v: TranForm]
  confirm: []
}>()

// 内部表单 ref
const formRef = ref<FormInstance>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<TranForm>({ ...props.form })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件打开对话框时填充数据）
watch(
  () => props.form,
  (newForm) => {
    if (syncing) return
    syncing = true
    localForm.value = { ...newForm }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    emit('update:form', { ...newForm })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

/** 点击确定：先校验再发 confirm */
const onConfirm = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async (valid: boolean) => {
    if (!valid) return
    emit('confirm')
  })
}
</script>

<!--
  ApproveDialogTab.vue - 审批质量标准对话框
  来源：原 quality/index.vue 中 审批质量标准对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    title="审批质量标准"
    width="500px"
    aria-label="质量审批对话框"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form ref="formRef" :model="formData" :rules="formRules" label-width="80px" aria-label="质量审批表单">
      <el-form-item label="标准编号">
        <el-input :model-value="currentRow?.standard_code" disabled />
      </el-form-item>
      <el-form-item label="标准名称">
        <el-input :model-value="currentRow?.standard_name" disabled />
      </el-form-item>
      <el-form-item label="当前版本">
        <el-input :model-value="currentRow?.version" disabled />
      </el-form-item>
      <el-form-item label="审批意见" prop="approval_comment">
        <el-input
          v-model="formData.approval_comment"
          type="textarea"
          :rows="4"
          placeholder="请输入审批意见"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">取消</el-button>
      <el-button type="warning" :loading="submitLoading" @click="handleReject">驳回</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handlePass">通过</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { rejectQualityStandard, type QualityStandard } from '@/api/quality'
import { logger } from '@/utils/logger'

interface Props {
  modelValue: boolean
  currentRow: QualityStandard | null
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
  (e: 'submitted', row: QualityStandard): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const formRef = ref<FormInstance>()
const submitLoading = ref(false)

const formData = reactive({ approval_comment: '' })

const formRules: FormRules = {
  approval_comment: [{ required: true, message: '请输入审批意见', trigger: 'blur' }],
}

const resetForm = () => {
  formData.approval_comment = ''
  formRef.value?.clearValidate()
}

watch(
  () => props.modelValue,
  val => {
    if (val) resetForm()
  }
)

const handlePass = async () => {
  if (!formRef.value || !props.currentRow) return
  await formRef.value.validate(async valid => {
    if (!valid) return
    submitLoading.value = true
    try {
      if (props.currentRow) {
        emit('submitted', props.currentRow)
      }
      emit('update:modelValue', false)
    } catch (error) {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
      logger.error('审批失败', err.message)
    } finally {
      submitLoading.value = false
    }
  })
}

const handleReject = async () => {
  if (!props.currentRow) return
  try {
    const reason = await ElMessageBox.prompt('请输入驳回原因', '确认驳回', {
      type: 'warning',
      confirmButtonText: '确定驳回',
      cancelButtonText: '取消',
      inputPlaceholder: '驳回原因（选填）',
      inputType: 'textarea',
    })
    // 批次 157d-2 修复：接入 rejectQualityStandard API
    submitLoading.value = true
    await rejectQualityStandard(props.currentRow.id, {
      reject_reason: reason.value || undefined,
    })
    ElMessage.success('驳回成功')
    emit('submitted', props.currentRow)
    emit('update:modelValue', false)
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
      logger.error('驳回失败', err.message)
    }
  } finally {
    submitLoading.value = false
  }
}
</script>

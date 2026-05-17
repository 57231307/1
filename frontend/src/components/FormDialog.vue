<template>
  <el-dialog
    v-model="visible"
    :title="title"
    :width="width"
    :close-on-click-modal="false"
    @close="handleClose"
  >
    <el-form
      ref="formRef"
      :model="formData"
      :rules="rules"
      label-width="100px"
      :disabled="mode === 'view'"
    >
      <slot :form-data="formData" :mode="mode" />
    </el-form>
    <template #footer v-if="mode !== 'view'">
      <el-button @click="handleClose">取消</el-button>
      <el-button type="primary" :loading="loading" @click="handleSubmit">
        {{ submitText }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'

interface Props {
  modelValue: boolean
  title: string
  width?: string
  mode?: 'create' | 'edit' | 'view'
  formData: Record<string, any>
  rules?: FormRules
  loading?: boolean
  submitText?: string
}

const props = withDefaults(defineProps<Props>(), {
  width: '600px',
  mode: 'create',
  loading: false,
  submitText: '保存'
})

const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  submit: [data: Record<string, any>]
  close: []
}>()

const formRef = ref<FormInstance>()
const visible = computed({
  get: () => props.modelValue,
  set: (val) => emit('update:modelValue', val)
})

const title = computed(() => {
  if (props.mode === 'create') return `新建${props.title}`
  if (props.mode === 'edit') return `编辑${props.title}`
  return `查看${props.title}`
})

watch(() => props.modelValue, (val) => {
  if (val && formRef.value) {
    formRef.value.clearValidate()
  }
})

const handleClose = () => {
  visible.value = false
  emit('close')
}

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate((valid) => {
    if (valid) {
      emit('submit', props.formData)
    }
  })
}

defineExpose({ formRef })
</script>

<!--
  KeyForm.vue - API 密钥新建/编辑对话框
  拆分自 api-gateway/index.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    :title="form.id ? '编辑密钥' : '新建密钥'"
    width="600px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      :ref="(el: any) => (formRefValue = el as FormInstance)"
      :model="form"
      :rules="rules"
      label-width="100px"
    >
      <el-form-item label="密钥名称" prop="key_name">
        <el-input
          :model-value="form.key_name"
          placeholder="请输入密钥名称"
          @update:model-value="(v: string) => (form.key_name = v ?? '')"
        />
      </el-form-item>
      <el-form-item label="描述" prop="description">
        <el-input
          :model-value="form.description"
          type="textarea"
          :rows="3"
          placeholder="请输入描述"
          @update:model-value="(v: string) => (form.description = v ?? '')"
        />
      </el-form-item>
      <el-form-item label="权限" prop="permissions">
        <el-input
          :model-value="permissionsText"
          placeholder="多个权限用逗号分隔"
          @update:model-value="(v: string) => (permissionsText = v ?? '')"
        />
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="限流(/秒)" prop="rate_limit">
            <el-input-number
              :model-value="form.rate_limit"
              :min="0"
              style="width: 100%"
              @update:model-value="(v: number) => (form.rate_limit = v ?? 0)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="过期时间" prop="expires_at">
            <el-date-picker
              :model-value="form.expires_at"
              type="datetime"
              placeholder="选择过期时间"
              style="width: 100%"
              @update:model-value="(v: string) => (form.expires_at = v ?? '')"
            />
          </el-form-item>
        </el-col>
      </el-row>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="emit('submit')">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { ref, watch } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import type { ApiKey } from '@/api/api-gateway'

/**
 * API 密钥新建/编辑对话框
 * 父组件通过 v-model 双向同步 permissionsText
 * 子组件通过 emit('update:*') 通知父组件更新
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 表单实例 ref
  formRef: { value: FormInstance | undefined }
  // 表单数据
  form: Partial<ApiKey>
  // 提交中状态
  submitLoading: boolean
  // 校验规则
  rules: FormRules
  // 权限文本（父组件通过 v-model 双向同步）
  permissionsText: string
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  'update:permissionsText': [v: string]
  submit: []
}>()

// 将 el-form 的 ref 实例同步到父组件传入的 formRef.value
const formRefValue = ref<FormInstance | undefined>(undefined)
watch(
  formRefValue,
  val => {
    if (val) props.formRef.value = val
  },
  { immediate: true, flush: 'post' }
)
</script>

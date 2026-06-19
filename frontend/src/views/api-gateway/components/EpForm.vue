<!--
  EpForm.vue - 接口新建/编辑对话框
  拆分自 api-gateway/index.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    :title="form.id ? '编辑接口' : '新建接口'"
    width="700px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      :ref="(el: any) => (formRefValue = el as FormInstance)"
      :model="form"
      :rules="rules"
      label-width="100px"
    >
      <el-row :gutter="20">
        <el-col :span="16">
          <el-form-item label="接口路径" prop="path">
            <el-input
              :model-value="form.path"
              placeholder="例如：/api/v1/users"
              @update:model-value="(v: string) => (form.path = v ?? '')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item label="请求方法" prop="method">
            <el-select
              :model-value="form.method"
              style="width: 100%"
              @update:model-value="(v: string) => (form.method = v as any)"
            >
              <el-option label="GET" value="GET" />
              <el-option label="POST" value="POST" />
              <el-option label="PUT" value="PUT" />
              <el-option label="DELETE" value="DELETE" />
              <el-option label="PATCH" value="PATCH" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="描述" prop="description">
        <el-input
          :model-value="form.description"
          placeholder="请输入接口描述"
          @update:model-value="(v: string) => (form.description = v ?? '')"
        />
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="8">
          <el-form-item label="模块" prop="module">
            <el-input
              :model-value="form.module"
              placeholder="模块名称"
              @update:model-value="(v: string) => (form.module = v ?? '')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item label="限流(/秒)" prop="rate_limit">
            <el-input-number
              :model-value="form.rate_limit"
              :min="0"
              style="width: 100%"
              @update:model-value="(v: number) => (form.rate_limit = v ?? 0)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item label="超时(ms)" prop="timeout">
            <el-input-number
              :model-value="form.timeout"
              :min="0"
              style="width: 100%"
              @update:model-value="(v: number) => (form.timeout = v ?? 0)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="8">
          <el-form-item label="需要认证" prop="authentication">
            <el-switch
              :model-value="form.authentication"
              @update:model-value="(v: boolean) => (form.authentication = v)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="16">
          <el-form-item label="权限" prop="authorization">
            <el-input
              :model-value="authorizationText"
              placeholder="多个权限用逗号分隔"
              @update:model-value="(v: string) => emit('update:authorizationText', v ?? '')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="请求Schema" prop="request_schema">
        <el-input
          :model-value="requestSchemaText"
          type="textarea"
          :rows="4"
          placeholder="JSON格式请求Schema"
          @update:model-value="(v: string) => (requestSchemaText = v ?? '')"
        />
      </el-form-item>
      <el-form-item label="响应Schema" prop="response_schema">
        <el-input
          :model-value="responseSchemaText"
          type="textarea"
          :rows="4"
          placeholder="JSON格式响应Schema"
          @update:model-value="(v: string) => emit('update:responseSchemaText', v ?? '')"
        />
      </el-form-item>
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
import type { ApiEndpoint } from '@/api/api-gateway'

/**
 * 接口新建/编辑对话框组件
 * 父组件通过 v-model 双向同步 authorizationText / requestSchemaText / responseSchemaText
 * 子组件通过 emit('update:*') 通知父组件更新
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 表单实例 ref（父组件持有的 FormInstance 引用包装对象）
  formRef: { value: FormInstance | undefined }
  // 表单数据
  form: Partial<ApiEndpoint>
  // 提交中状态
  submitLoading: boolean
  // 校验规则
  rules: FormRules
  // 权限文本（父组件通过 v-model 双向同步）
  authorizationText: string
  // 请求 Schema 文本（父组件通过 v-model 双向同步）
  requestSchemaText: string
  // 响应 Schema 文本（父组件通过 v-model 双向同步）
  responseSchemaText: string
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  'update:authorizationText': [v: string]
  'update:requestSchemaText': [v: string]
  'update:responseSchemaText': [v: string]
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

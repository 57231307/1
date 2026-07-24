<!--
  ApiEndpointForm.vue - 接口新建/编辑对话框
  拆分自 api-gateway/index.vue（P14 批 1 B3 I-2）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="form?.id ? '编辑接口' : '新建接口'"
    width="700px"
    :aria-label="form?.id ? '编辑接口对话框' : '新建接口对话框'"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      :ref="(el: unknown) => (formRefValue = el as FormInstance)"
      :model="localForm"
      :rules="rules"
      label-width="100px"
      aria-label="接口表单"
    >
      <el-row :gutter="20">
        <el-col :span="16">
          <el-form-item label="接口路径" prop="path">
            <el-input
              :model-value="localForm.path ?? ''"
              placeholder="例如：/api/v1/users"
              @update:model-value="(v: string) => (localForm.path = v ?? '')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item label="请求方法" prop="method">
            <el-select
              :model-value="localForm.method"
              style="width: 100%"
              @update:model-value="(v: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH') => (localForm.method = v)"
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
          :model-value="localForm.description ?? ''"
          placeholder="请输入接口描述"
          @update:model-value="(v: string) => (localForm.description = v ?? '')"
        />
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="8">
          <el-form-item label="模块" prop="module">
            <el-input
              :model-value="localForm.module ?? ''"
              placeholder="模块名称"
              @update:model-value="(v: string) => (localForm.module = v ?? '')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item label="限流(/秒)" prop="rate_limit">
            <el-input-number
              :model-value="localForm.rate_limit"
              :min="0"
              style="width: 100%"
              @update:model-value="(v: number) => (localForm.rate_limit = v ?? 0)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item label="超时(ms)" prop="timeout">
            <el-input-number
              :model-value="localForm.timeout"
              :min="0"
              style="width: 100%"
              @update:model-value="(v: number) => (localForm.timeout = v ?? 0)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="8">
          <el-form-item label="需要认证" prop="authentication">
            <el-switch
              :model-value="!!localForm.authentication"
              @update:model-value="(v: boolean) => (localForm.authentication = v)"
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
          @update:model-value="(v: string) => emit('update:requestSchemaText', v ?? '')"
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
import { ref, watch, nextTick } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import type { ApiEndpoint } from '@/api/api-gateway'

/**
 * 接口新建/编辑对话框组件
 * 父组件通过 v-model 双向同步 authorizationText / requestSchemaText / responseSchemaText
 * 子组件通过 emit('update:*') 通知父组件更新
 * 表单数据通过本地 ref 镜像 + 双向 watch 防循环 + emit('update:form') 整体回写
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 表单实例（批次 281：改为可选，通过 v-model:formRef 双向同步）
  formRef?: FormInstance | undefined
  // 表单数据（由父组件管理，子组件通过 emit 回写）
  form?: Partial<ApiEndpoint>
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
  // 批次 281：通过 emit 通知父组件 formRef 变化（替代原 props.formRef.value 写回）
  'update:formRef': [value: FormInstance | undefined]
  'update:authorizationText': [v: string]
  'update:requestSchemaText': [v: string]
  'update:responseSchemaText': [v: string]
  // 整体回写表单（父组件监听此事件并 Object.assign 到自己的 form）
  'update:form': [form: Partial<ApiEndpoint>]
  submit: []
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<Partial<ApiEndpoint>>({ ...(props.form ?? {}) })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件打开新建/编辑时填充数据）
watch(
  () => props.form,
  newForm => {
    if (syncing) return
    syncing = true
    localForm.value = { ...(newForm ?? {}) }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true }
)

// 本地变化时通知父组件（用户输入）
watch(
  localForm,
  newForm => {
    if (syncing) return
    syncing = true
    emit('update:form', { ...newForm })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true }
)

// 批次 281：将 el-form 的 ref 实例通过 emit 通知父组件（替代原 props.formRef.value 写回）
const formRefValue = ref<FormInstance | undefined>(undefined)
watch(
  formRefValue,
  val => {
    if (val) emit('update:formRef', val)
  },
  { immediate: true, flush: 'post' }
)
</script>

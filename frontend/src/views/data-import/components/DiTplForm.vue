<!--
  DiTplForm.vue - 数据导入模板新建/编辑对话框
  拆分自 data-import/index.vue（P14 批 2 I-3 第 5 批）
  P9-3 批次 F Pattern A 重构：本地 reactive 镜像 + watch 同步 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="localForm.id ? '编辑模板' : '新建模板'"
    width="800px"
    :aria-label="localForm.id ? '编辑模板对话框' : '新建模板对话框'"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      ref="formRef"
      :model="localForm"
      :rules="rules"
      label-width="100px"
      aria-label="数据导入模板表单"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="模板编号" prop="template_code">
            <el-input
              :model-value="(localForm.template_code as string) || ''"
              :disabled="!!localForm.id"
              placeholder="请输入模板编号"
              @update:model-value="(v: string) => { localForm.template_code = v; syncFormToParent() }"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="模板名称" prop="template_name">
            <el-input
              :model-value="(localForm.template_name as string) || ''"
              placeholder="请输入模板名称"
              @update:model-value="(v: string) => { localForm.template_name = v; syncFormToParent() }"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="模块" prop="module">
            <el-select
              :model-value="(localForm.module as string) || ''"
              placeholder="请选择模块"
              style="width: 100%"
              @update:model-value="(v: string) => { localForm.module = v; syncFormToParent() }"
            >
              <el-option label="客户" value="customer" />
              <el-option label="供应商" value="supplier" />
              <el-option label="产品" value="product" />
              <el-option label="库存" value="inventory" />
              <el-option label="销售" value="sales" />
              <el-option label="采购" value="purchase" />
              <el-option label="财务" value="finance" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="文件格式" prop="file_format">
            <el-select
              :model-value="(localForm.file_format as string) || ''"
              placeholder="请选择格式"
              style="width: 100%"
              @update:model-value="(v: string) => { localForm.file_format = v; syncFormToParent() }"
            >
              <el-option label="Excel" value="xlsx" />
              <el-option label="CSV" value="csv" />
              <el-option label="JSON" value="json" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="描述" prop="description">
        <el-input
          :model-value="(localForm.description as string) || ''"
          type="textarea"
          :rows="3"
          placeholder="请输入描述"
          @update:model-value="(v: string) => { localForm.description = v; syncFormToParent() }"
        />
      </el-form-item>
      <el-form-item label="列配置" prop="columns">
        <el-input
          :model-value="localColumnsText"
          type="textarea"
          :rows="6"
          placeholder='JSON格式列配置，例如：[{"key":"name","label":"名称","type":"string","required":true}]'
          @update:model-value="(v: string) => { localColumnsText = v; emit('update:columns-text', v) }"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="emit('submit')"
        >确定</el-button
      >
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import type { DiTplForm } from '../composables/useDiProc'

// 表单默认值
const DEFAULT_FORM: DiTplForm = {
  id: undefined,
  template_code: '',
  template_name: '',
  description: '',
  module: 'customer',
  file_format: 'xlsx',
  columns: [],
  sample_data: [],
  status: 'active',
}

/**
 * 模板表单组件
 */
const props = defineProps<{
  // 可见性
  visible: boolean
  // 表单数据（由父组件管理，子组件通过 emit('update:form') 整体回写）
  form?: DiTplForm
  // 验证规则
  rules: FormRules
  // 提交加载
  submitLoading: boolean
  // 列配置 JSON 文本
  columnsText?: string
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  // 整体回写表单数据（父组件监听后 Object.assign 到自己的 form）
  'update:form': [form: DiTplForm]
  'update:columns-text': [v: string]
  submit: []
}>()

// 表单 ref
const formRef = ref<FormInstance>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = reactive<DiTplForm>({
  ...(props.form ?? DEFAULT_FORM),
})

const localColumnsText = ref(props.columnsText ?? '')

// 父组件 form 变化时同步到本地（如打开编辑对话框时重新赋值）
watch(
  () => props.form,
  newForm => {
    if (newForm) Object.assign(localForm, newForm)
  },
  { deep: true },
)

// 父组件 columnsText 变化时同步到本地
watch(
  () => props.columnsText,
  newText => {
    if (newText !== undefined) localColumnsText.value = newText
  },
)

/** 同步表单到父组件（深拷贝避免外部引用被意外修改） */
const syncFormToParent = () => {
  emit('update:form', { ...localForm })
}

// 暴露给父组件访问
defineExpose({ formRef })
</script>

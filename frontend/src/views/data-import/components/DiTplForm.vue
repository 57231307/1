<!--
  DiTplForm.vue - 数据导入模板新建/编辑对话框
  拆分自 data-import/index.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    :title="form.id ? '编辑模板' : '新建模板'"
    width="800px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="100px"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="模板编号" prop="template_code">
            <el-input
              :model-value="(form.template_code as string) || ''"
              :disabled="!!form.id"
              placeholder="请输入模板编号"
              @update:model-value="(v: string) => (form.template_code = v)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="模板名称" prop="template_name">
            <el-input
              :model-value="(form.template_name as string) || ''"
              placeholder="请输入模板名称"
              @update:model-value="(v: string) => (form.template_name = v)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="模块" prop="module">
            <el-select
              :model-value="(form.module as string) || ''"
              placeholder="请选择模块"
              style="width: 100%"
              @update:model-value="(v: string) => (form.module = v)"
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
              :model-value="(form.file_format as string) || ''"
              placeholder="请选择格式"
              style="width: 100%"
              @update:model-value="(v: string) => (form.file_format = v)"
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
          :model-value="(form.description as string) || ''"
          type="textarea"
          :rows="3"
          placeholder="请输入描述"
          @update:model-value="(v: string) => (form.description = v)"
        />
      </el-form-item>
      <el-form-item label="列配置" prop="columns">
        <el-input
          :model-value="columnsText"
          type="textarea"
          :rows="6"
          placeholder='JSON格式列配置，例如：[{"key":"name","label":"名称","type":"string","required":true}]'
          @update:model-value="(v: string) => emit('update:columns-text', v)"
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
/* eslint-disable vue/no-mutating-props */
import { ref, watch } from 'vue'
import type { FormInstance, FormRules } from 'element-plus'
import type { DiTplForm } from '../composables/useDiProc'

/**
 * 模板表单组件
 */
const props = defineProps<{
  // 可见性
  visible: boolean
  // 表单数据
  form: DiTplForm
  // 验证规则
  rules: FormRules
  // 提交加载
  submitLoading: boolean
  // 列配置 JSON 文本
  columnsText: string
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  'update:columns-text': [v: string]
  submit: []
}>()

// 表单 ref
const formRef = ref<FormInstance>()

// 暴露给父组件访问
defineExpose({ formRef })

// 同步列配置文本（父组件需要双向同步）
watch(
  () => props.columnsText,
  v => {
    // 触发 emit，让父组件知道
    if (v !== undefined) emit('update:columns-text', v)
  }
)
</script>

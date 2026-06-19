<!--
  BpmDfTplDlg.vue - BPM 流程定义保存为模板对话框
  拆分自 bpm/definitions.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    title="保存为模板"
    width="500px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form ref="formRef" :model="formData" :rules="rules" label-width="100px">
      <el-form-item label="模板名称" prop="template_name">
        <el-input
          :model-value="formData.template_name"
          placeholder="请输入模板名称"
          @update:model-value="(v: string) => (formData.template_name = v)"
        />
      </el-form-item>
      <el-form-item label="分类" prop="category">
        <el-select
          :model-value="formData.category"
          placeholder="请选择分类"
          style="width: 100%"
          @update:model-value="(v: string) => (formData.category = v)"
        >
          <el-option label="财务" value="finance" />
          <el-option label="人事" value="hr" />
          <el-option label="采购" value="purchase" />
          <el-option label="销售" value="sales" />
          <el-option label="生产" value="production" />
          <el-option label="库存" value="inventory" />
          <el-option label="其他" value="other" />
        </el-select>
      </el-form-item>
      <el-form-item label="描述">
        <el-input
          :model-value="formData.description"
          type="textarea"
          :rows="3"
          placeholder="请输入模板描述"
          @update:model-value="(v: string) => (formData.description = v)"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button type="primary" :loading="loading" @click="emit('submit')">保存</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { ref, type FormInstance, type FormRules } from 'element-plus'

// 表单数据类型
interface TplForm {
  template_name: string
  category: string
  description: string
}

/**
 * 保存为模板对话框
 */
defineProps<{
  // 可见性
  visible: boolean
  // 加载状态
  loading: boolean
  // 表单数据
  formData: TplForm
  // 验证规则
  rules: FormRules
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  submit: []
}>()

// 表单 ref
const formRef = ref<FormInstance>()

// 暴露给父组件
defineExpose({ formRef })
</script>

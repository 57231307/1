<!--
  BpmDefinitionTemplateDialog.vue - BPM 流程定义保存为模板对话框
  拆分自 bpm/definitions.vue（P14 批 2 I-3 第 5 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="保存为模板"
    aria-label="流程模板对话框"
    width="500px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form ref="formRef" :model="localFormData" :rules="rules" label-width="100px" aria-label="流程模板表单">
      <el-form-item label="模板名称" prop="template_name">
        <el-input
          v-model="localFormData.template_name"
          placeholder="请输入模板名称"
        />
      </el-form-item>
      <el-form-item label="分类" prop="category">
        <el-select
          v-model="localFormData.category"
          placeholder="请选择分类"
          style="width: 100%"
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
          v-model="localFormData.description"
          type="textarea"
          :rows="3"
          placeholder="请输入模板描述"
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
import { ref, watch, nextTick } from 'vue'
import { type FormInstance, type FormRules } from 'element-plus'

// 表单数据类型
interface TplForm {
  template_name: string
  category: string
  description: string
}

/**
 * 保存为模板对话框
 */
const props = defineProps<{
  // 可见性
  visible: boolean
  // 加载状态
  loading: boolean
  // 表单数据（由父组件管理，子组件通过 emit 回写）
  formData: TplForm
  // 验证规则
  rules: FormRules
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  // 整体回写表单数据（父组件监听此事件并 Object.assign 到自己的 formData）
  'update:formData': [v: TplForm]
  submit: []
}>()

// 表单 ref
const formRef = ref<FormInstance>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localFormData = ref<TplForm>({ ...props.formData })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件打开保存模板时填充数据）
watch(
  () => props.formData,
  (newData) => {
    if (syncing) return
    syncing = true
    localFormData.value = { ...newData }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localFormData,
  (newData) => {
    if (syncing) return
    syncing = true
    emit('update:formData', { ...newData })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 暴露给父组件
defineExpose({ formRef })
</script>

<!--
  StandardFormDialogTab.vue - 质量标准编辑对话框
  来源：原 quality/index.vue 中 质量标准编辑对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="formData.id ? '编辑标准' : '新建标准'"
    width="700px"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
      <el-form-item label="标准编号" prop="standard_code">
        <el-input
          v-model="formData.standard_code"
          :disabled="!!formData.id"
          placeholder="请输入标准编号"
        />
      </el-form-item>
      <el-form-item label="标准名称" prop="standard_name">
        <el-input v-model="formData.standard_name" placeholder="请输入标准名称" />
      </el-form-item>
      <el-form-item label="类型" prop="type">
        <el-select v-model="formData.type" placeholder="请选择类型" style="width: 100%">
          <el-option label="产品标准" value="product" />
          <el-option label="工艺标准" value="process" />
        </el-select>
      </el-form-item>
      <el-form-item label="版本" prop="version">
        <el-input v-model="formData.version" placeholder="例如：1.0" />
      </el-form-item>
      <el-form-item label="标准内容" prop="content">
        <el-input
          v-model="formData.content"
          type="textarea"
          :rows="6"
          placeholder="请输入标准内容"
        />
      </el-form-item>
      <el-form-item label="附件" prop="attachments">
        <el-input
          v-model="attachmentsText"
          type="textarea"
          placeholder='JSON格式数组，例如：["附件1.pdf", "附件2.docx"]'
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { createQualityStandard, updateQualityStandard, type QualityStandard } from '@/api/quality'
import { logger } from '@/utils/logger'

interface Props {
  modelValue: boolean
  currentRow: QualityStandard | null
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
  (e: 'submitted'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const formRef = ref<FormInstance>()
const submitLoading = ref(false)
const attachmentsText = ref('')

const formData = reactive({
  id: 0,
  standard_code: '',
  standard_name: '',
  version: '1.0',
  type: 'product' as 'product' | 'process',
  status: 'draft' as 'draft' | 'approved' | 'published' | 'rejected',
  content: '',
  attachments: [] as string[],
})

const formRules: FormRules = {
  standard_code: [{ required: true, message: '请输入标准编号', trigger: 'blur' }],
  standard_name: [{ required: true, message: '请输入标准名称', trigger: 'blur' }],
  type: [{ required: true, message: '请选择类型', trigger: 'change' }],
  version: [{ required: true, message: '请输入版本号', trigger: 'blur' }],
  content: [{ required: true, message: '请输入标准内容', trigger: 'blur' }],
}

const resetForm = () => {
  formData.id = 0
  formData.standard_code = ''
  formData.standard_name = ''
  formData.version = '1.0'
  formData.type = 'product'
  formData.status = 'draft'
  formData.content = ''
  formData.attachments = []
  attachmentsText.value = ''
  formRef.value?.clearValidate()
}

watch(
  () => props.modelValue,
  val => {
    if (val) {
      if (props.currentRow) {
        Object.assign(formData, props.currentRow)
        attachmentsText.value = JSON.stringify(props.currentRow.attachments || [], null, 2)
      } else {
        resetForm()
      }
    }
  }
)

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return
    submitLoading.value = true
    try {
      if (attachmentsText.value) {
        try {
          formData.attachments = JSON.parse(attachmentsText.value)
        } catch {
          ElMessage.error('附件格式错误，请检查JSON格式')
          return
        }
      }
      if (formData.id) {
        await updateQualityStandard(formData.id, formData as Partial<QualityStandard>)
      } else {
        await createQualityStandard(formData as Partial<QualityStandard>)
      }
      ElMessage.success('操作成功')
      emit('update:modelValue', false)
      emit('submitted')
    } catch (error) {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
      logger.error('质量标准保存失败', err.message)
    } finally {
      submitLoading.value = false
    }
  })
}
</script>

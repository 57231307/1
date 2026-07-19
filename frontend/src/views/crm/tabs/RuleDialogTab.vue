<!--
  RuleDialogTab.vue - 客户分配规则对话框
  来源：原 crm/assignment.vue 中 新建/编辑规则对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog v-model="visible" :title="title" width="700px" :close-on-click-modal="false" :aria-label="title">
    <el-form ref="formRef" :model="formData" :rules="formRules" label-width="120px" aria-label="公海规则表单">
      <el-form-item label="规则名称" prop="name">
        <el-input v-model="formData.name" placeholder="请输入规则名称" />
      </el-form-item>
      <el-form-item label="分配策略" prop="strategy">
        <el-select v-model="formData.strategy" placeholder="请选择分配策略" style="width: 100%">
          <el-option label="平均分配" value="average" />
          <el-option label="按地域分配" value="region" />
          <el-option label="按行业分配" value="industry" />
          <el-option label="按客户规模" value="scale" />
        </el-select>
      </el-form-item>
      <el-form-item label="分配对象" prop="userIds">
        <el-select
          v-model="formData.userIds"
          multiple
          filterable
          placeholder="请选择负责人们"
          style="width: 100%"
        >
          <el-option
            v-for="user in users"
            :key="user.id"
            :label="user.real_name"
            :value="user.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item label="优先级" prop="priority">
        <el-input-number v-model="formData.priority" :min="0" :max="100" style="width: 100%" />
      </el-form-item>
      <el-form-item label="是否启用" prop="enabled">
        <el-radio-group v-model="formData.enabled">
          <el-radio :value="true">启用</el-radio>
          <el-radio :value="false">禁用</el-radio>
        </el-radio-group>
      </el-form-item>
      <el-form-item label="备注" prop="remark">
        <el-input v-model="formData.remark" type="textarea" :rows="3" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">保存</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import type { User } from '@/api/user'
import { logger } from '@/utils/logger'

interface Props {
  modelValue: boolean
  title: string
  rowData: Partial<RuleRow> | null
  users: User[]
}

interface RuleRow {
  id?: number
  name?: string
  strategy?: string
  userIds?: number[]
  priority?: number
  enabled?: boolean
  remark?: string
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
  (e: 'submitted'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const visible = ref(props.modelValue)
const submitLoading = ref(false)
const formRef = ref<FormInstance>()

const formData = reactive({
  id: undefined as number | undefined,
  name: '',
  strategy: '',
  userIds: [] as number[],
  priority: 50,
  enabled: true,
  remark: '',
})

const formRules: FormRules = {
  name: [{ required: true, message: '请输入规则名称', trigger: 'blur' }],
  strategy: [{ required: true, message: '请选择分配策略', trigger: 'change' }],
  userIds: [{ required: true, message: '请选择分配对象', trigger: 'change' }],
}

watch(
  () => props.modelValue,
  val => {
    visible.value = val
    if (val) {
      resetForm()
      if (props.rowData) {
        Object.assign(formData, props.rowData)
      }
    }
  }
)

watch(visible, val => {
  emit('update:modelValue', val)
})

const resetForm = () => {
  formData.id = undefined
  formData.name = ''
  formData.strategy = ''
  formData.userIds = []
  formData.priority = 50
  formData.enabled = true
  formData.remark = ''
  formRef.value?.clearValidate()
}

const handleSubmit = async () => {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
    submitLoading.value = true
    ElMessage.success('保存成功')
    visible.value = false
    emit('submitted')
  } catch (error) {
    const err = error as Error
    logger.warn('表单验证失败', err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>

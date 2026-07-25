<!--
  RuleDialogTab.vue - 客户分配规则对话框
  来源：原 crm/assignment.vue 中 新建/编辑规则对话框
-->
<template>
  <el-dialog v-model="visible" :title="title" width="700px" :close-on-click-modal="false" :aria-label="title">
    <el-form ref="formRef" :model="formData" :rules="formRules" label-width="120px" :aria-label="t('crmRuleDialog.ariaLabel')">
      <el-form-item :label="t('crmRuleDialog.form.name')" prop="name">
        <el-input v-model="formData.name" :placeholder="t('crmRuleDialog.form.namePlaceholder')" />
      </el-form-item>
      <el-form-item :label="t('crmRuleDialog.form.strategy')" prop="strategy">
        <el-select v-model="formData.strategy" :placeholder="t('crmRuleDialog.form.strategyPlaceholder')" style="width: 100%">
          <el-option :label="t('crmRuleDialog.strategy.average')" value="average" />
          <el-option :label="t('crmRuleDialog.strategy.region')" value="region" />
          <el-option :label="t('crmRuleDialog.strategy.industry')" value="industry" />
          <el-option :label="t('crmRuleDialog.strategy.scale')" value="scale" />
        </el-select>
      </el-form-item>
      <el-form-item :label="t('crmRuleDialog.form.assignees')" prop="userIds">
        <el-select
          v-model="formData.userIds"
          multiple
          filterable
          :placeholder="t('crmRuleDialog.form.assigneesPlaceholder')"
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
      <el-form-item :label="t('crmRuleDialog.form.priority')" prop="priority">
        <el-input-number v-model="formData.priority" :min="0" :max="100" style="width: 100%" />
      </el-form-item>
      <el-form-item :label="t('crmRuleDialog.form.enabled')" prop="enabled">
        <el-radio-group v-model="formData.enabled">
          <el-radio :value="true">{{ t('crmRuleDialog.form.enabledYes') }}</el-radio>
          <el-radio :value="false">{{ t('crmRuleDialog.form.enabledNo') }}</el-radio>
        </el-radio-group>
      </el-form-item>
      <el-form-item :label="t('crmRuleDialog.form.remark')" prop="remark">
        <el-input v-model="formData.remark" type="textarea" :rows="3" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">{{ t('crmRuleDialog.form.cancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{ t('crmRuleDialog.form.save') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import type { User } from '@/api/user'
import { logger } from '@/utils/logger'

const { t } = useI18n({ useScope: 'global' })

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
  name: [{ required: true, message: t('crmRuleDialog.validation.nameRequired'), trigger: 'blur' }],
  strategy: [{ required: true, message: t('crmRuleDialog.validation.strategyRequired'), trigger: 'change' }],
  userIds: [{ required: true, message: t('crmRuleDialog.validation.assigneesRequired'), trigger: 'change' }],
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
    ElMessage.success(t('crmRuleDialog.message.saveSuccess'))
    visible.value = false
    emit('submitted')
  } catch (error) {
    const err = error as Error
    logger.warn(t('crmRuleDialog.message.validationFailed'), err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>

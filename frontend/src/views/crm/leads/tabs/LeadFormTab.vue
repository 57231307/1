<!--
  LeadFormTab.vue - 线索新建/编辑对话框
  来源：原 crm/leads/index.vue 中 新建/编辑对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog v-model="visible" :title="title" width="800px" :close-on-click-modal="false" :aria-label="title">
    <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px" :aria-label="t('crmLeads.leadForm.ariaLabel')">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('crmLeads.leadForm.leadSource')" prop="lead_source">
            <el-select v-model="formData.lead_source" :placeholder="t('crmLeads.leadForm.leadSourcePlaceholder')">
              <el-option :label="t('crmLeads.leadForm.leadSourceOption.website')" value="WEBSITE" />
              <el-option :label="t('crmLeads.leadForm.leadSourceOption.phone')" value="PHONE" />
              <el-option :label="t('crmLeads.leadForm.leadSourceOption.exhibition')" value="EXHIBITION" />
              <el-option :label="t('crmLeads.leadForm.leadSourceOption.referral')" value="REFERRAL" />
              <el-option :label="t('crmLeads.leadForm.leadSourceOption.other')" value="OTHER" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('crmLeads.leadForm.priority')" prop="priority">
            <el-select v-model="formData.priority" :placeholder="t('crmLeads.leadForm.priorityPlaceholder')">
              <el-option :label="t('crmLeads.leadForm.priorityOption.low')" value="LOW" />
              <el-option :label="t('crmLeads.leadForm.priorityOption.medium')" value="MEDIUM" />
              <el-option :label="t('crmLeads.leadForm.priorityOption.high')" value="HIGH" />
              <el-option :label="t('crmLeads.leadForm.priorityOption.urgent')" value="URGENT" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('crmLeads.leadForm.companyName')" prop="company_name">
            <el-input v-model="formData.company_name" :placeholder="t('crmLeads.leadForm.companyNamePlaceholder')" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('crmLeads.leadForm.contactName')" prop="contact_name">
            <el-input v-model="formData.contact_name" :placeholder="t('crmLeads.leadForm.contactNamePlaceholder')" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('crmLeads.leadForm.mobilePhone')" prop="mobile_phone">
            <el-input v-model="formData.mobile_phone" :placeholder="t('crmLeads.leadForm.mobilePhonePlaceholder')" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('crmLeads.leadForm.email')" prop="email">
            <el-input v-model="formData.email" :placeholder="t('crmLeads.leadForm.emailPlaceholder')" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('crmLeads.leadForm.contactTitle')" prop="contact_title">
            <el-input v-model="formData.contact_title" :placeholder="t('crmLeads.leadForm.contactTitlePlaceholder')" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('crmLeads.leadForm.owner')" prop="owner_id">
            <el-select v-model="formData.owner_id" :placeholder="t('crmLeads.leadForm.ownerPlaceholder')" filterable>
              <el-option v-for="u in users" :key="u.id" :label="u.real_name" :value="u.id" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('crmLeads.leadForm.requirementDesc')" prop="requirement_desc">
        <el-input
          v-model="formData.requirement_desc"
          type="textarea"
          :rows="3"
          :placeholder="t('crmLeads.leadForm.requirementDescPlaceholder')"
        />
      </el-form-item>
      <el-form-item :label="t('crmLeads.leadForm.remarks')" prop="remarks">
        <el-input v-model="formData.remarks" type="textarea" :rows="2" :placeholder="t('crmLeads.leadForm.remarksPlaceholder')" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">{{ t('crmLeads.leadForm.cancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{ t('crmLeads.leadForm.confirm') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import type { Lead } from '@/api/crm'
import type { User } from '@/api/user'
import { logger } from '@/utils/logger'

const { t } = useI18n({ useScope: 'global' })

interface Props {
  modelValue: boolean
  title: string
  rowData: Partial<Lead> | null
  users: User[]
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
  id: null as number | null,
  lead_source: '',
  company_name: '',
  contact_name: '',
  contact_title: '',
  mobile_phone: '',
  email: '',
  priority: 'MEDIUM',
  owner_id: '' as string | number,
  requirement_desc: '',
  remarks: '',
})

const formRules: FormRules = {
  lead_source: [{ required: true, message: t('crmLeads.leadForm.validation.leadSourceRequired'), trigger: 'change' }],
  contact_name: [{ required: true, message: t('crmLeads.leadForm.validation.contactNameRequired'), trigger: 'blur' }],
  owner_id: [{ required: true, message: t('crmLeads.leadForm.validation.ownerRequired'), trigger: 'change' }],
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
  formData.id = null
  formData.lead_source = ''
  formData.company_name = ''
  formData.contact_name = ''
  formData.contact_title = ''
  formData.mobile_phone = ''
  formData.email = ''
  formData.priority = 'MEDIUM'
  formData.owner_id = ''
  formData.requirement_desc = ''
  formData.remarks = ''
}

const handleSubmit = async () => {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
    submitLoading.value = true
    ElMessage.success(t('crmLeads.leadForm.message.saveSuccess'))
    visible.value = false
    emit('submitted')
  } catch (error) {
    const err = error as Error
    logger.warn(t('crmLeads.leadForm.message.validationFailed'), err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>

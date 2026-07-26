<!--
  CustomerFormTab.vue - 客户新建/编辑对话框
  来源：原 customer/index.vue 中 新建/编辑对话框
  拆分日期：2026-06-15 B3-3
-->
<template>
  <el-dialog
    v-model="visible"
    :title="title"
    width="700px"
    :close-on-click-modal="false"
    :aria-label="t('customer.form.ariaLabel')"
    @close="handleClose"
  >
    <el-form ref="formRef" :model="formData" :rules="formRules" label-width="120px" :aria-label="t('customer.form.formAriaLabel')">
      <el-divider content-position="left">{{ t('customer.form.section.basic') }}</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.customerCode')" prop="customer_code">
            <el-input v-model="formData.customer_code" :placeholder="t('customer.form.placeholder.customerCode')" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.customerName')" prop="customer_name">
            <el-input v-model="formData.customer_name" :placeholder="t('customer.form.placeholder.customerName')" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.contactPerson')" prop="contact_person">
            <el-input v-model="formData.contact_person" :placeholder="t('customer.form.placeholder.contactPerson')" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.contactPhone')" prop="contact_phone">
            <el-input v-model="formData.contact_phone" :placeholder="t('customer.form.placeholder.contactPhone')" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.email')" prop="contact_email">
            <el-input v-model="formData.contact_email" :placeholder="t('customer.form.placeholder.email')" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.customerType')" prop="customer_type">
            <el-select
              v-model="formData.customer_type"
              :placeholder="t('customer.form.placeholder.customerType')"
              style="width: 100%"
            >
              <el-option :label="t('customer.form.option.typeRetail')" value="retail" />
              <el-option :label="t('customer.form.option.typeWholesale')" value="wholesale" />
              <el-option :label="t('customer.form.option.typeVip')" value="vip" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.industry')" prop="customer_industry">
            <el-input v-model="formData.customer_industry" :placeholder="t('customer.form.placeholder.industry')" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.annualPurchase')" prop="annual_purchase">
            <el-input-number
              v-model="formData.annual_purchase"
              :min="0"
              :precision="2"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider content-position="left">{{ t('customer.form.section.address') }}</el-divider>
      <el-form-item :label="t('customer.form.label.address')" prop="address">
        <el-input v-model="formData.address" :placeholder="t('customer.form.placeholder.address')" />
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.province')" prop="province">
            <el-input v-model="formData.province" :placeholder="t('customer.form.placeholder.province')" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.city')" prop="city">
            <el-input v-model="formData.city" :placeholder="t('customer.form.placeholder.city')" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.postalCode')" prop="postal_code">
            <el-input v-model="formData.postal_code" :placeholder="t('customer.form.placeholder.postalCode')" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.country')" prop="country">
            <el-input v-model="formData.country" :placeholder="t('customer.form.placeholder.country')" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider content-position="left">{{ t('customer.form.section.finance') }}</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.taxId')" prop="tax_id">
            <el-input v-model="formData.tax_id" :placeholder="t('customer.form.placeholder.taxId')" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.creditLimit')" prop="credit_limit">
            <el-input-number
              v-model="formData.credit_limit"
              :min="0"
              :precision="2"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.paymentTerms')" prop="payment_terms">
            <el-input-number v-model="formData.payment_terms" :min="0" style="width: 100%" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.status')" prop="status">
            <el-radio-group v-model="formData.status">
              <el-radio value="active">{{ t('customer.form.status.active') }}</el-radio>
              <el-radio value="inactive">{{ t('customer.form.status.inactive') }}</el-radio>
            </el-radio-group>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.bankName')" prop="bank_name">
            <el-input v-model="formData.bank_name" :placeholder="t('customer.form.placeholder.bankName')" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.bankAccount')" prop="bank_account">
            <el-input v-model="formData.bank_account" :placeholder="t('customer.form.placeholder.bankAccount')" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider content-position="left">{{ t('customer.form.section.business') }}</el-divider>
      <el-form-item :label="t('customer.form.label.mainProducts')" prop="main_products">
        <el-input v-model="formData.main_products" :placeholder="t('customer.form.placeholder.mainProducts')" />
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.qualityRequirement')" prop="quality_requirement">
            <el-input v-model="formData.quality_requirement" :placeholder="t('customer.form.placeholder.qualityRequirement')" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('customer.form.label.inspectionStandard')" prop="inspection_standard">
            <el-input v-model="formData.inspection_standard" :placeholder="t('customer.form.placeholder.inspectionStandard')" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('customer.form.label.notes')" prop="notes">
        <el-input v-model="formData.notes" type="textarea" :rows="3" :placeholder="t('customer.form.placeholder.notes')" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="visible = false">{{ t('customer.form.button.cancel') }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{ t('customer.form.button.save') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import type { Customer } from '@/api/customer'
import { logger } from '@/utils/logger'

const { t } = useI18n({ useScope: 'global' })

interface Props {
  modelValue: boolean
  title: string
  rowData: Partial<Customer> | null
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
  customer_code: '',
  customer_name: '',
  contact_person: '',
  contact_phone: '',
  contact_email: '',
  address: '',
  city: '',
  province: '',
  country: '',
  postal_code: '',
  customer_type: 'retail',
  tax_id: '',
  credit_limit: 0,
  payment_terms: 30,
  bank_name: '',
  bank_account: '',
  status: 'active',
  notes: '',
  customer_industry: '',
  main_products: '',
  annual_purchase: 0,
  quality_requirement: '',
  inspection_standard: '',
})

const formRules: FormRules = {
  customer_code: [{ required: true, message: t('customer.form.validation.customerCodeRequired'), trigger: 'blur' }],
  customer_name: [{ required: true, message: t('customer.form.validation.customerNameRequired'), trigger: 'blur' }],
  contact_person: [{ required: true, message: t('customer.form.validation.contactPersonRequired'), trigger: 'blur' }],
  contact_phone: [
    { required: true, message: t('customer.form.validation.contactPhoneRequired'), trigger: 'blur' },
    { pattern: /^1[3-9]\d{9}$/, message: t('customer.form.validation.contactPhoneInvalid'), trigger: 'blur' },
  ],
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
  formData.customer_code = ''
  formData.customer_name = ''
  formData.contact_person = ''
  formData.contact_phone = ''
  formData.contact_email = ''
  formData.address = ''
  formData.city = ''
  formData.province = ''
  formData.country = ''
  formData.postal_code = ''
  formData.customer_type = 'retail'
  formData.tax_id = ''
  formData.credit_limit = 0
  formData.payment_terms = 30
  formData.bank_name = ''
  formData.bank_account = ''
  formData.status = 'active'
  formData.notes = ''
  formData.customer_industry = ''
  formData.main_products = ''
  formData.annual_purchase = 0
  formData.quality_requirement = ''
  formData.inspection_standard = ''
  formRef.value?.clearValidate()
}

const handleClose = () => {
  resetForm()
}

const handleSubmit = async () => {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
    submitLoading.value = true
    ElMessage.success(t('customer.form.message.saveSuccess'))
    visible.value = false
    emit('submitted')
  } catch (error) {
    const err = error as Error
    logger.warn(t('customer.form.message.validationFailed'), err.message)
  } finally {
    submitLoading.value = false
  }
}
</script>

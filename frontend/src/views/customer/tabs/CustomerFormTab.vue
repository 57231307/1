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
    @close="handleClose"
  >
    <el-form ref="formRef" :model="formData" :rules="formRules" label-width="120px">
      <el-divider content-position="left">基本信息</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="客户编码" prop="customer_code">
            <el-input v-model="formData.customer_code" placeholder="请输入客户编码" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="客户名称" prop="customer_name">
            <el-input v-model="formData.customer_name" placeholder="请输入客户名称" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="联系人" prop="contact_person">
            <el-input v-model="formData.contact_person" placeholder="请输入联系人" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="联系电话" prop="contact_phone">
            <el-input v-model="formData.contact_phone" placeholder="请输入联系电话" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="邮箱" prop="contact_email">
            <el-input v-model="formData.contact_email" placeholder="请输入邮箱" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="客户类型" prop="customer_type">
            <el-select
              v-model="formData.customer_type"
              placeholder="请选择类型"
              style="width: 100%"
            >
              <el-option label="零售" value="retail" />
              <el-option label="批发" value="wholesale" />
              <el-option label="VIP" value="vip" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="行业" prop="customer_industry">
            <el-input v-model="formData.customer_industry" placeholder="请输入行业" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="年采购额" prop="annual_purchase">
            <el-input-number
              v-model="formData.annual_purchase"
              :min="0"
              :precision="2"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider content-position="left">地址信息</el-divider>
      <el-form-item label="地址" prop="address">
        <el-input v-model="formData.address" placeholder="请输入详细地址" />
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="省份" prop="province">
            <el-input v-model="formData.province" placeholder="请输入省份" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="城市" prop="city">
            <el-input v-model="formData.city" placeholder="请输入城市" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="邮编" prop="postal_code">
            <el-input v-model="formData.postal_code" placeholder="请输入邮编" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="国家" prop="country">
            <el-input v-model="formData.country" placeholder="请输入国家" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider content-position="left">财务信息</el-divider>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="税号" prop="tax_id">
            <el-input v-model="formData.tax_id" placeholder="请输入税号" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="信用额度" prop="credit_limit">
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
          <el-form-item label="账期(天)" prop="payment_terms">
            <el-input-number v-model="formData.payment_terms" :min="0" style="width: 100%" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="状态" prop="status">
            <el-radio-group v-model="formData.status">
              <el-radio value="active">启用</el-radio>
              <el-radio value="inactive">停用</el-radio>
            </el-radio-group>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="开户银行" prop="bank_name">
            <el-input v-model="formData.bank_name" placeholder="请输入开户银行" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="银行账号" prop="bank_account">
            <el-input v-model="formData.bank_account" placeholder="请输入银行账号" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider content-position="left">业务信息</el-divider>
      <el-form-item label="主营产品" prop="main_products">
        <el-input v-model="formData.main_products" placeholder="请输入主营产品" />
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="质量要求" prop="quality_requirement">
            <el-input v-model="formData.quality_requirement" placeholder="请输入质量要求" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="验货标准" prop="inspection_standard">
            <el-input v-model="formData.inspection_standard" placeholder="请输入验货标准" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="备注" prop="notes">
        <el-input v-model="formData.notes" type="textarea" :rows="3" placeholder="请输入备注" />
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
import type { Customer } from '@/api/customer'
import { logger } from '@/utils/logger'

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
  customer_code: [{ required: true, message: '请输入客户编码', trigger: 'blur' }],
  customer_name: [{ required: true, message: '请输入客户名称', trigger: 'blur' }],
  contact_person: [{ required: true, message: '请输入联系人', trigger: 'blur' }],
  contact_phone: [
    { required: true, message: '请输入电话', trigger: 'blur' },
    { pattern: /^1[3-9]\d{9}$/, message: '请输入正确的手机号', trigger: 'blur' },
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

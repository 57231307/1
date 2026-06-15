<!--
  CompanyTab.vue - 公司信息 Tab
  来源：原 system/index.vue 中 公司信息 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="company-tab">
    <div class="page-header">
      <h2 class="page-title">公司信息设置</h2>
    </div>
    <el-card shadow="hover">
      <el-form
        ref="companyFormRef"
        :model="companyForm"
        :rules="companyRules"
        label-width="120px"
        style="max-width: 800px"
      >
        <el-divider content-position="left">基本信息</el-divider>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="公司名称" prop="company_name">
              <el-input v-model="companyForm.company_name" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="公司简称">
              <el-input v-model="companyForm.company_short_name" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="信用代码">
              <el-input v-model="companyForm.credit_code" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="法定代表人">
              <el-input v-model="companyForm.legal_representative" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-divider content-position="left">联系方式</el-divider>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="联系电话">
              <el-input v-model="companyForm.phone" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="邮箱">
              <el-input v-model="companyForm.email" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="地址">
          <el-input v-model="companyForm.address" />
        </el-form-item>
        <el-divider content-position="left">银行信息</el-divider>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="开户银行">
              <el-input v-model="companyForm.bank_name" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="银行账号">
              <el-input v-model="companyForm.bank_account" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item>
          <el-button type="primary" :loading="companySubmitLoading" @click="saveCompanyInfo"
            >保存</el-button
          >
          <el-button @click="resetCompanyForm">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'

interface CompanyForm {
  company_name: string
  company_short_name: string
  credit_code: string
  legal_representative: string
  registered_capital: number
  establishment_date: string
  phone: string
  fax: string
  email: string
  website: string
  address: string
  bank_name: string
  bank_account: string
  taxpayer_type: string
  tax_registration_number: string
  logo: string
  remarks: string
}

const companyFormRef = ref<FormInstance>()
const companySubmitLoading = ref(false)
const companyForm = reactive<CompanyForm>({
  company_name: '',
  company_short_name: '',
  credit_code: '',
  legal_representative: '',
  registered_capital: 0,
  establishment_date: '',
  phone: '',
  fax: '',
  email: '',
  website: '',
  address: '',
  bank_name: '',
  bank_account: '',
  taxpayer_type: 'general',
  tax_registration_number: '',
  logo: '',
  remarks: '',
})

const companyRules: FormRules = {
  company_name: [{ required: true, message: '请输入公司名称', trigger: 'blur' }],
}

const fetchCompanyInfo = async () => {
  try {
    const s = localStorage.getItem('company_info')
    if (s) Object.assign(companyForm, JSON.parse(s))
  } catch (_e) {
    // 静默：本地存储读取失败时使用表单默认值
  }
}

const saveCompanyInfo = async () => {
  if (!companyFormRef.value) return
  try {
    const valid = await companyFormRef.value.validate()
    if (!valid) return
  } catch (_e) {
    return
  }
  companySubmitLoading.value = true
  try {
    localStorage.setItem('company_info', JSON.stringify(companyForm))
    ElMessage.success('保存成功')
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '保存失败')
  } finally {
    companySubmitLoading.value = false
  }
}

const resetCompanyForm = () => {
  companyFormRef.value?.resetFields()
}

defineExpose({ refresh: fetchCompanyInfo })

onMounted(() => {
  fetchCompanyInfo()
})
</script>

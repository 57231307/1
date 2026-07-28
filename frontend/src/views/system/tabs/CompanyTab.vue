<!--
  CompanyTab.vue - 公司信息 Tab
  来源：原 system/index.vue 中 公司信息 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="company-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('system.company.title') }}</h2>
    </div>
    <el-card shadow="hover">
      <el-form
        ref="companyFormRef"
        :model="companyForm"
        :rules="companyRules"
        label-width="120px"
        style="max-width: 800px"
        :aria-label="t('system.company.aria.form')"
      >
        <el-divider content-position="left">{{ t('system.company.divider.basic') }}</el-divider>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('system.company.label.companyName')" prop="company_name">
              <el-input v-model="companyForm.company_name" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('system.company.label.companyShortName')">
              <el-input v-model="companyForm.company_short_name" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('system.company.label.creditCode')">
              <el-input v-model="companyForm.credit_code" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('system.company.label.legalRep')">
              <el-input v-model="companyForm.legal_representative" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-divider content-position="left">{{ t('system.company.divider.contact') }}</el-divider>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('system.company.label.phone')">
              <el-input v-model="companyForm.phone" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('system.company.label.email')">
              <el-input v-model="companyForm.email" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="t('system.company.label.address')">
          <el-input v-model="companyForm.address" />
        </el-form-item>
        <el-divider content-position="left">{{ t('system.company.divider.bank') }}</el-divider>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('system.company.label.bankName')">
              <el-input v-model="companyForm.bank_name" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('system.company.label.bankAccount')">
              <el-input v-model="companyForm.bank_account" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item>
          <el-button type="primary" :loading="companySubmitLoading" @click="saveCompanyInfo">{{
            t('system.company.button.save')
          }}</el-button>
          <el-button @click="resetCompanyForm">{{ t('system.company.button.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';

const { t } = useI18n({ useScope: 'global' });

interface CompanyForm {
  company_name: string;
  company_short_name: string;
  credit_code: string;
  legal_representative: string;
  registered_capital: number;
  establishment_date: string;
  phone: string;
  fax: string;
  email: string;
  website: string;
  address: string;
  bank_name: string;
  bank_account: string;
  taxpayer_type: string;
  tax_registration_number: string;
  logo: string;
  remarks: string;
}

const companyFormRef = ref<FormInstance>();
const companySubmitLoading = ref(false);
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
});

const companyRules: FormRules = {
  company_name: [
    { required: true, message: t('system.company.message.requiredName'), trigger: 'blur' },
  ],
};

const fetchCompanyInfo = async () => {
  try {
    const s = localStorage.getItem('company_info');
    if (s) Object.assign(companyForm, JSON.parse(s));
  } catch (_e) {
    // 静默：本地存储读取失败时使用表单默认值
  }
};

const saveCompanyInfo = async () => {
  if (!companyFormRef.value) return;
  try {
    const valid = await companyFormRef.value.validate();
    if (!valid) return;
  } catch (_e) {
    return;
  }
  companySubmitLoading.value = true;
  try {
    // FE-P2-4 修复（v12 前端复审）：过滤敏感字段，仅缓存非敏感信息到 localStorage
    // 敏感字段（credit_code/legal_representative/bank_name/bank_account/tax_registration_number 等）
    // 不写入 localStorage，防止 XSS 攻击读取企业敏感信息
    const nonSensitiveFields: Partial<CompanyForm> = {
      company_name: companyForm.company_name,
      company_short_name: companyForm.company_short_name,
      phone: companyForm.phone,
      fax: companyForm.fax,
      email: companyForm.email,
      website: companyForm.website,
      address: companyForm.address,
      logo: companyForm.logo,
      remarks: companyForm.remarks,
    };
    localStorage.setItem('company_info', JSON.stringify(nonSensitiveFields));
    ElMessage.success(t('system.company.message.saveSuccess'));
  } catch (e) {
    const err = e as { message?: string };
    ElMessage.error(err.message || t('system.company.message.saveFailed'));
  } finally {
    companySubmitLoading.value = false;
  }
};

const resetCompanyForm = () => {
  companyFormRef.value?.resetFields();
};

defineExpose({ refresh: fetchCompanyInfo });

onMounted(() => {
  fetchCompanyInfo();
});
</script>

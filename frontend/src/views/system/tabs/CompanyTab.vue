<!--
  CompanyTab.vue - 公司信息 Tab
  来源：原 system/index.vue 中 公司信息 tab 内容
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
            <el-form-item :label="t('system.company.label.companyName')" prop="name">
              <el-input v-model="companyForm.name" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('system.company.label.companyShortName')">
              <el-input v-model="companyForm.short_name" />
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
        <el-form-item :label="t('system.company.label.logo')">
          <div class="logo-upload">
            <img v-if="logoPreviewSrc" :src="logoPreviewSrc" class="logo-preview" alt="" />
            <label class="logo-file-button">
              <span>{{ t('system.company.button.uploadLogo') }}</span>
              <input
                ref="logoFileInputRef"
                type="file"
                accept="image/png"
                class="logo-file-input"
                @change="onLogoFileChange"
              />
            </label>
            <el-button v-if="companyForm.logo" link type="danger" @click="removeLogo">{{
              t('system.company.button.removeLogo')
            }}</el-button>
          </div>
        </el-form-item>
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
import { computed, reactive, ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import {
  getCompanyProfileFull,
  updateCompanyProfile,
  type CompanyProfileUpdatePayload,
} from '@/api/company-profile';
import { useCompanyProfile } from '@/composables/useCompanyProfile';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });
const { refresh } = useCompanyProfile();

// PNG 上传本地校验上限：512KB（与后端 ≤512KB 约束一致）
const LOGO_MAX_BYTES = 512 * 1024;

// 表单模型逐键对齐后端全量可编辑列（snake_case），不再持有任何后端不读的遗留键。
// 可空列初值 null（对齐后端 Option 载荷语义），name 必填初值空串。
const companyForm = reactive<CompanyProfileUpdatePayload>({
  name: '',
  short_name: null,
  credit_code: null,
  legal_representative: null,
  tax_registration_number: null,
  bank_name: null,
  bank_account: null,
  taxpayer_type: null,
  phone: null,
  fax: null,
  email: null,
  website: null,
  address: null,
  registered_capital: null,
  establishment_date: null,
  logo: null,
  description: null,
});

// 预览用完整 data URL（base64 裸串补回前缀仅供 <img> 显示；落库/上送仍用去前缀的裸串）
const logoPreviewSrc = computed(() =>
  companyForm.logo ? `data:image/png;base64,${companyForm.logo}` : ''
);

const companyRules: FormRules = {
  name: [{ required: true, message: t('system.company.message.requiredName'), trigger: 'blur' }],
};

const companyFormRef = ref<FormInstance>();
const companySubmitLoading = ref(false);

// logo 文件输入 DOM 引用（label 包裹隐藏 input，点击 label 触发原生文件选择）
const logoFileInputRef = ref<HTMLInputElement>();

const fetchCompanyInfo = async () => {
  try {
    const res = await getCompanyProfileFull();
    const f = res.data;
    // 全量回填：可空列原样赋值（可能为 null），不做 ?? 默认掩盖缺键。
    companyForm.name = f.name;
    companyForm.short_name = f.short_name;
    companyForm.credit_code = f.credit_code;
    companyForm.legal_representative = f.legal_representative;
    companyForm.tax_registration_number = f.tax_registration_number;
    companyForm.bank_name = f.bank_name;
    companyForm.bank_account = f.bank_account;
    companyForm.taxpayer_type = f.taxpayer_type;
    companyForm.phone = f.phone;
    companyForm.fax = f.fax;
    companyForm.email = f.email;
    companyForm.website = f.website;
    companyForm.address = f.address;
    companyForm.registered_capital = f.registered_capital;
    companyForm.establishment_date = f.establishment_date;
    companyForm.logo = f.logo;
    companyForm.description = f.description;
  } catch (error) {
    logger.error(t('system.company.message.loadFailed'), error);
    ElMessage.error(t('system.company.message.loadFailed'));
  }
};

// logo 选择：仅接受 PNG（本地按 MIME 校验，后端再校验 PNG 魔数），≤512KB；
// FileReader 读 data URL 后剥离 `data:...;base64,` 前缀，仅把裸 base64 绑到 form.logo。
// 校验不通过只给 i18n 提示、不发后端。
const onLogoFileChange = (event: Event) => {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  if (file.type !== 'image/png') {
    ElMessage.error(t('system.company.message.logoNotPng'));
    if (logoFileInputRef.value) logoFileInputRef.value.value = '';
    return;
  }
  if (file.size > LOGO_MAX_BYTES) {
    ElMessage.error(t('system.company.message.logoTooLarge'));
    if (logoFileInputRef.value) logoFileInputRef.value.value = '';
    return;
  }
  const reader = new FileReader();
  reader.onload = () => {
    const dataUrl = reader.result as string;
    companyForm.logo = dataUrl.replace(/^data:[^;]+;base64,/, '');
    ElMessage.success(t('system.company.message.logoUploaded'));
  };
  reader.onerror = () => {
    logger.error(t('system.company.message.logoReadFailed'), reader.error);
    ElMessage.error(t('system.company.message.logoReadFailed'));
  };
  reader.readAsDataURL(file);
  // 允许重复选择同一文件时再次触发 change
  if (logoFileInputRef.value) logoFileInputRef.value.value = '';
};

const removeLogo = () => {
  companyForm.logo = null;
};

const saveCompanyInfo = async () => {
  if (!companyFormRef.value) return;
  try {
    const valid = await companyFormRef.value.validate();
    if (!valid) return;
  } catch {
    // 表单校验不通过（Element Plus 以 reject 表达），属预期交互，不再重复提示
    return;
  }
  companySubmitLoading.value = true;
  try {
    await updateCompanyProfile({ ...companyForm });
    ElMessage.success(t('system.company.message.saveSuccess'));
    // 保存成功后回填共享档案副本，头部品牌名/简称/标志随服务端值即时更新
    await refresh();
  } catch (error) {
    const err = error as { message?: string };
    ElMessage.error(err.message || t('system.company.message.saveFailed'));
  } finally {
    companySubmitLoading.value = false;
  }
};

const resetCompanyForm = () => {
  // 重置语义为“放弃本地改动、恢复服务端当前值”，故重新拉取全量而非清空为初始空值
  void fetchCompanyInfo();
};

defineExpose({ refresh: fetchCompanyInfo });

onMounted(() => {
  void fetchCompanyInfo();
});
</script>

<style scoped>
.logo-upload {
  display: flex;
  align-items: center;
  gap: 12px;
}
.logo-preview {
  width: 48px;
  height: 48px;
  object-fit: contain;
  border: 1px solid var(--el-border-color);
  border-radius: 4px;
}
.logo-file-button {
  display: inline-flex;
  align-items: center;
  cursor: pointer;
  color: var(--el-color-primary);
}
.logo-file-input {
  display: none;
}
</style>

<!--
  公司信息页（路由 /company，门控 users:read）
  调用方：MainLayout 侧栏 组织与身份 分组下的 公司信息 菜单项
  职责：读写公司档案主体字段并维护标志图片
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
import {
  detectImageFormat,
  readFileHead,
  stripDataUrlPrefix,
  type ImageFormatToken,
} from '@/utils/logo-file';

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
// 档案是否已读取成功：没读到现状就禁止提交。保存端点是全量替换（提交字段即档案终态、
// 未提交的可空列一律清空为 NULL），单例表只有一行无回退，若在读档失败（表单停在全 null
// 初值）后放行保存，会把信用代码、法定代表人、税务登记号、开户行与账号、电话、地址、
// 注册资本、成立日期、logo、备注全部清成 NULL，属毁数据的静默降级。
const profileLoaded = ref(false);

// logo 文件输入 DOM 引用（label 包裹隐藏 input，点击 label 触发原生文件选择）
const logoFileInputRef = ref<HTMLInputElement>();

const fetchCompanyInfo = async () => {
  // 每次拉取先撤回已加载标记：失败或出参契约异常时不得保留上一轮的 true，
  // 否则保存门闸会被陈旧的成功状态误放行
  profileLoaded.value = false;
  try {
    const res = await getCompanyProfileFull();
    const f = res.data;
    // 出参校验（不能仅凭 HTTP 200 就判定已加载）：单例档案的 NOT NULL 列 id/name/updated_at/
    // updated_by 恒有真实类型值，任一缺失或类型不符即契约异常，抛错交下方 catch 显式提示，
    // 绝不带残缺表单置 loaded（一旦放行保存，全量替换会把可空列清成 NULL）
    const isRealProfile =
      !!f &&
      typeof f === 'object' &&
      !Array.isArray(f) &&
      typeof f.id === 'number' &&
      typeof f.name === 'string' &&
      typeof f.updated_at === 'string' &&
      typeof f.updated_by === 'number';
    if (!isRealProfile) {
      throw new Error(t('system.company.message.loadFailed'));
    }
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
    profileLoaded.value = true;
  } catch (error) {
    logger.error(t('system.company.message.loadFailed'), error);
    ElMessage.error(t('system.company.message.loadFailed'));
  }
};

// logo 选择：按**文件头魔数**判类型（File.type 是浏览器按扩展名查表的结果，
// 大写后缀、系统 MIME 表异常返回 image/x-png 或空串都会误拒真实 PNG；扩展名
// 与内容不符的伪 png 则会漏到后端被魔数拒绝，用户见到"上传的就是 png 却被拒"）。
// 判定与后端 company_profile_service 的 PNG 魔数判据同源：真实 PNG 一律放行，
// 非 PNG 按检测到的真实格式点名拒绝、不发后端；≤512KB 与后端上限一致。
const onLogoFileChange = async (event: Event) => {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  // 允许重复选择同一文件时再次触发 change（先复位，异步读取期间用户重选同文件也生效）
  if (logoFileInputRef.value) logoFileInputRef.value.value = '';
  let format: ImageFormatToken;
  try {
    format = detectImageFormat(await readFileHead(file));
  } catch (error) {
    logger.error(t('system.company.message.logoReadFailed'), error);
    ElMessage.error(t('system.company.message.logoReadFailed'));
    return;
  }
  if (format !== 'png') {
    ElMessage.error(t('system.company.message.logoNotPng', { detected: format }));
    return;
  }
  if (file.size > LOGO_MAX_BYTES) {
    ElMessage.error(t('system.company.message.logoTooLarge'));
    return;
  }
  const reader = new FileReader();
  reader.onload = () => {
    const dataUrl = reader.result as string;
    companyForm.logo = stripDataUrlPrefix(dataUrl);
    ElMessage.success(t('system.company.message.logoUploaded'));
  };
  reader.onerror = () => {
    logger.error(t('system.company.message.logoReadFailed'), reader.error);
    ElMessage.error(t('system.company.message.logoReadFailed'));
  };
  reader.readAsDataURL(file);
};

const removeLogo = () => {
  companyForm.logo = null;
};

const saveCompanyInfo = async () => {
  if (!companyFormRef.value) return;
  // 保存门闸（对齐同型全量替换端点的角色授权页范式）：档案未读取成功即拒绝提交，
  // 不能仅凭 name 必填拦下——读档失败时表单停在除 name 外全 null 的初值，
  // 一旦放行 PUT 会按全量替换把其余可空列静默清空
  if (!profileLoaded.value) {
    logger.warn(t('system.company.message.notLoadedSaveBlocked'));
    ElMessage.error(t('system.company.message.notLoadedSaveBlocked'));
    return;
  }
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

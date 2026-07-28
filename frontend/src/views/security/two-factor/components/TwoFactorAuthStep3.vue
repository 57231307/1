<!--
  TwoFactorAuthStep3.vue - 2FA Step 3 验证并启用面板（自包含表单 + 暴露 validate/setError 给父组件）
  拆分自 security/TwoFactorSetup.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="step-content">
    <h3 class="step-title">{{ t('security.twoFactor.step3.title') }}</h3>
    <p class="step-desc">{{ t('security.twoFactor.step3.desc') }}</p>

    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="0"
      class="verify-form"
      :aria-label="t('security.twoFactor.step3.ariaLabel.form')"
      @submit.prevent
    >
      <el-form-item prop="token">
        <el-input
          v-model="form.token"
          :placeholder="t('security.twoFactor.step3.placeholder.token')"
          maxlength="6"
          size="large"
          class="token-input"
        >
          <template #prefix>
            <el-icon><Key /></el-icon>
          </template>
        </el-input>
      </el-form-item>
    </el-form>

    <el-alert
      v-if="errorMsg"
      :title="errorMsg"
      type="error"
      show-icon
      :closable="false"
      class="error-alert"
    />

    <div class="tips">
      <el-icon><InfoFilled /></el-icon>
      <span>{{ t('security.twoFactor.step3.tip.tokenUpdate') }}</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import { Key, InfoFilled } from '@element-plus/icons-vue';
import type { FormInstance, FormRules } from 'element-plus';
import { verifyRules } from '../composables/tfaFmts';

const { t } = useI18n({ useScope: 'global' });

// 自包含的表单状态（避免 prop mutation 问题）
const formRef = ref<FormInstance>();
const form = reactive({ token: '' });
const errorMsg = ref<string>('');
const rules: FormRules = verifyRules;

// 暴露给父组件的验证方法
const validate = (): Promise<{ valid: boolean; token: string }> => {
  return new Promise(resolve => {
    if (!formRef.value) {
      resolve({ valid: false, token: '' });
      return;
    }
    formRef.value.validate((valid: boolean) => {
      resolve({ valid, token: form.token });
    });
  });
};

// 暴露给父组件的设置错误方法
const setError = (msg: string) => {
  errorMsg.value = msg;
};

// 暴露给父组件的清除错误方法
const clearError = () => {
  errorMsg.value = '';
};

defineExpose({ validate, setError, clearError, formRef, form, errorMsg });
</script>

<style scoped>
.step-content {
  max-width: 600px;
  margin: 0 auto;
  padding: 30px 20px;
}

.step-title {
  text-align: center;
  font-size: 22px;
  margin-bottom: 8px;
  color: #303133;
}

.step-desc {
  text-align: center;
  color: #606266;
  margin-bottom: 24px;
}

.verify-form {
  margin-bottom: 16px;
}

.token-input {
  text-align: center;
  font-size: 18px;
  letter-spacing: 4px;
}

.error-alert {
  margin-bottom: 16px;
}

.tips {
  display: flex;
  align-items: center;
  gap: 6px;
  color: #909399;
  font-size: 13px;
  text-align: center;
  justify-content: center;
}
</style>

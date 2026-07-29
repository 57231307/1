<!--
  TwoFactorStepBar.vue - 2FA 设置步骤条（4 步）
  拆分自 security/TwoFactorSetup.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-steps :active="currentStep" finish-status="success" align-center>
    <el-step :title="t('security.twoFactor.stepBar.step1')" />
    <el-step :title="t('security.twoFactor.stepBar.step2')" />
    <el-step :title="t('security.twoFactor.stepBar.step3')" />
    <el-step :title="t('security.twoFactor.stepBar.step4')" />
  </el-steps>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';

const { t } = useI18n({ useScope: 'global' });

defineProps<{ currentStep: number }>();
</script>

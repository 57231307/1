<!--
  TwoFactorAuthStep1.vue - 2FA Step 1 启动设置（状态/账户名/启用按钮）
  拆分自 security/TwoFactorSetup.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="step-content">
    <h3 class="step-title">{{ t('security.twoFactor.step1.title') }}</h3>
    <p class="step-desc">{{ t('security.twoFactor.step1.desc') }}</p>

    <el-alert
      v-if="isEnabled"
      :title="t('security.twoFactor.step1.alert.enabled.title')"
      type="success"
      :description="t('security.twoFactor.step1.alert.enabled.desc')"
      show-icon
      :closable="false"
      class="status-alert"
    />
    <el-alert
      v-else
      :title="t('security.twoFactor.step1.alert.disabled.title')"
      type="warning"
      :description="t('security.twoFactor.step1.alert.disabled.desc')"
      show-icon
      :closable="false"
      class="status-alert"
    />

    <div class="info-block">
      <div class="info-row">
        <span class="info-label">{{ t('security.twoFactor.step1.label.username') }}</span>
        <span class="info-value">{{ username }}</span>
      </div>
      <div class="info-row">
        <span class="info-label">{{ t('security.twoFactor.step1.label.status') }}</span>
        <el-tag v-if="isEnabled" type="success">{{ t('security.twoFactor.step1.status.enabled') }}</el-tag>
        <el-tag v-else type="warning">{{ t('security.twoFactor.step1.status.disabled') }}</el-tag>
      </div>
    </div>

    <div class="action-row">
      <el-button
        type="primary"
        size="large"
        :loading="setupLoading"
        :disabled="isEnabled"
        @click="emit('start-setup')"
      >
        <el-icon><Setting /></el-icon>
        {{ isEnabled ? t('security.twoFactor.step1.button.enabledText') : t('security.twoFactor.step1.button.setup') }}
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { Setting } from '@element-plus/icons-vue'

const { t } = useI18n({ useScope: 'global' })

defineProps<{
  isEnabled: boolean
  username: string
  setupLoading: boolean
}>()

const emit = defineEmits<{ 'start-setup': [] }>()
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

.status-alert {
  margin-bottom: 24px;
}

.info-block {
  background: #f5f7fa;
  padding: 20px;
  border-radius: 8px;
  margin-bottom: 24px;
}

.info-row {
  display: flex;
  align-items: center;
  margin-bottom: 12px;
}

.info-row:last-child {
  margin-bottom: 0;
}

.info-label {
  font-weight: 500;
  color: #606266;
  min-width: 80px;
}

.info-value {
  color: #303133;
}

.action-row {
  text-align: center;
}
</style>

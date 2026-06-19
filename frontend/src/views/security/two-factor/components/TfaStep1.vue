<!--
  TfaStep1.vue - 2FA Step 1 启动设置（状态/账户名/启用按钮）
  拆分自 security/TwoFactorSetup.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="step-content">
    <h3 class="step-title">设置两步验证</h3>
    <p class="step-desc">两步验证为您的账户提供额外的安全保障</p>

    <el-alert
      v-if="isEnabled"
      title="两步验证已启用"
      type="success"
      description="您的账户已启用两步验证，重新设置将需要先关闭现有设置。"
      show-icon
      :closable="false"
      class="status-alert"
    />
    <el-alert
      v-else
      title="两步验证未启用"
      type="warning"
      description="建议您启用两步验证以提高账户安全性。"
      show-icon
      :closable="false"
      class="status-alert"
    />

    <div class="info-block">
      <div class="info-row">
        <span class="info-label">账户名：</span>
        <span class="info-value">{{ username }}</span>
      </div>
      <div class="info-row">
        <span class="info-label">当前状态：</span>
        <el-tag v-if="isEnabled" type="success">已启用</el-tag>
        <el-tag v-else type="warning">未启用</el-tag>
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
        {{ isEnabled ? '已启用' : '启动设置' }}
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Setting } from '@element-plus/icons-vue'

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

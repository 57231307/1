<!--
  TwoFactorAuthStep2.vue - 2FA Step 2 扫描二维码面板
  拆分自 security/TwoFactorSetup.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="step-content">
    <h3 class="step-title">扫描二维码</h3>
    <p class="step-desc">
      使用 Google Authenticator、Microsoft Authenticator 或其他兼容的应用扫描以下二维码
    </p>

    <div class="qr-container">
      <el-image
        v-if="qrCodeDataUrl"
        :src="qrCodeDataUrl"
        fit="contain"
        class="qr-image"
        alt="二步验证二维码"
      />
      <div v-else class="qr-loading">
        <el-icon class="is-loading"><Loading /></el-icon>
        <p>正在生成二维码...</p>
      </div>
    </div>

    <el-form label-position="top" class="info-form" aria-label="双因素认证信息表单">
      <el-form-item label="账户名">
        <el-input :model-value="username" readonly />
      </el-form-item>
      <el-form-item label="发行方">
        <el-input model-value="ERP System" readonly />
      </el-form-item>
      <el-form-item label="手动输入密钥">
        <el-input
          :model-value="secretText"
          readonly
          class="secret-input"
        >
          <template #append>
            <el-button @click="emit('copy-secret')">
              <el-icon><CopyDocument /></el-icon>
              复制
            </el-button>
          </template>
        </el-input>
      </el-form-item>
    </el-form>
  </div>
</template>

<script setup lang="ts">
import { CopyDocument, Loading } from '@element-plus/icons-vue'

defineProps<{
  qrCodeDataUrl: string
  secretText: string
  username: string
}>()

const emit = defineEmits<{ 'copy-secret': [] }>()
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

.qr-container {
  display: flex;
  justify-content: center;
  margin-bottom: 24px;
  min-height: 200px;
  align-items: center;
}

.qr-image {
  width: 200px;
  height: 200px;
  border: 1px solid #ebeef5;
  padding: 10px;
  background: white;
  border-radius: 8px;
}

.qr-loading {
  text-align: center;
  color: #909399;
}

.qr-loading .el-icon {
  font-size: 36px;
  margin-bottom: 8px;
}

.info-form {
  margin-top: 20px;
}

.secret-input :deep(.el-input__inner) {
  font-family: 'Courier New', monospace;
  letter-spacing: 1px;
}
</style>

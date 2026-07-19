<!--
  TfaStep3.vue - 2FA Step 3 验证并启用面板（自包含表单 + 暴露 validate/setError 给父组件）
  拆分自 security/TwoFactorSetup.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="step-content">
    <h3 class="step-title">验证并启用</h3>
    <p class="step-desc">请输入身份验证器应用显示的 6 位动态口令以完成启用</p>

    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="0"
      class="verify-form"
      aria-label="双因素验证码表单"
      @submit.prevent
    >
      <el-form-item prop="token">
        <el-input
          v-model="form.token"
          placeholder="请输入 6 位动态口令"
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
      <span>动态口令每 30 秒更新一次，请输入最新口令</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue'
import { Key, InfoFilled } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import { verifyRules } from '../composables/tfaFmts'

// 自包含的表单状态（避免 prop mutation 问题）
const formRef = ref<FormInstance>()
const form = reactive({ token: '' })
const errorMsg = ref<string>('')
const rules: FormRules = verifyRules

// 暴露给父组件的验证方法
const validate = (): Promise<{ valid: boolean; token: string }> => {
  return new Promise(resolve => {
    if (!formRef.value) {
      resolve({ valid: false, token: '' })
      return
    }
    formRef.value.validate((valid: boolean) => {
      resolve({ valid, token: form.token })
    })
  })
}

// 暴露给父组件的设置错误方法
const setError = (msg: string) => {
  errorMsg.value = msg
}

// 暴露给父组件的清除错误方法
const clearError = () => {
  errorMsg.value = ''
}

defineExpose({ validate, setError, clearError, formRef, form, errorMsg })
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

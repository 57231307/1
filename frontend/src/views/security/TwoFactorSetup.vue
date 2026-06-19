<!--
  security/TwoFactorSetup.vue - 双因素认证（拆分重构版）
  任务编号: P14 批 2 I-3 第 6 批
  拆分：540 行 → ~110 行 + 5 子组件（步骤条 + 4 步） + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
  注：路由直接引用 views/security/TwoFactorSetup.vue，组件放在子目录 two-factor/
-->
<template>
  <div class="two-factor-page">
    <div class="page-header">
      <h1 class="page-title">双因素认证</h1>
      <el-breadcrumb separator="/">
        <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
        <el-breadcrumb-item>系统管理</el-breadcrumb-item>
        <el-breadcrumb-item>双因素认证</el-breadcrumb-item>
      </el-breadcrumb>
    </div>

    <el-card shadow="hover">
      <TfaStepBar :current-step="tfa.currentStep" />

      <div class="step-wrapper">
        <TfaStep1
          v-show="tfa.currentStep === 0"
          :is-enabled="tfa.isEnabled"
          :username="tfa.username"
          :setup-loading="tfa.setupLoading"
          @start-setup="tfaProc.handleStartSetup(tfa)"
        />
        <TfaStep2
          v-show="tfa.currentStep === 1"
          :qr-code-data-url="tfa.qrCodeDataUrl"
          :secret-text="tfa.secretText"
          :username="tfa.username"
          @copy-secret="tfaProc.handleCopySecret(tfa)"
        />
        <TfaStep3
          v-show="tfa.currentStep === 2"
          ref="tfaStep3Ref"
        />
        <TfaStep4
          v-show="tfa.currentStep === 3"
          :recovery-codes="tfa.recoveryCodes"
          @copy-recovery="tfaProc.handleCopyRecovery(tfa)"
          @finish="tfaProc.handleFinish"
        />
      </div>

      <div v-if="tfa.currentStep > 0 && tfa.currentStep < 3" class="step-actions">
        <el-button @click="tfaProc.handlePrevStep(tfa)">
          <el-icon><ArrowLeft /></el-icon>
          上一步
        </el-button>
        <el-button v-if="tfa.currentStep < 2" type="primary" @click="tfaProc.handleNextStep(tfa)">
          下一步
          <el-icon><ArrowRight /></el-icon>
        </el-button>
        <el-button
          v-else
          type="primary"
          :loading="tfa.enableLoading"
          @click="onVerify"
        >
          验证并启用
          <el-icon><Check /></el-icon>
        </el-button>
      </div>

      <div v-if="tfa.currentStep === 3" class="step-actions">
        <el-button type="primary" @click="tfaProc.handleFinish">
          完成
          <el-icon><Check /></el-icon>
        </el-button>
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { ArrowLeft, ArrowRight, Check } from '@element-plus/icons-vue'
import { useTfa } from './two-factor/composables/useTfa'
import { useTfaProc, type TfaStep3Instance } from './two-factor/composables/useTfaProc'
import TfaStepBar from './two-factor/components/TfaStepBar.vue'
import TfaStep1 from './two-factor/components/TfaStep1.vue'
import TfaStep2 from './two-factor/components/TfaStep2.vue'
import TfaStep3 from './two-factor/components/TfaStep3.vue'
import TfaStep4 from './two-factor/components/TfaStep4.vue'

// 业务状态
const tfa = useTfa()
const tfaProc = useTfaProc()

// TfaStep3 子组件 ref（用于调其 validate() 和 setError() 方法）
const tfaStep3Ref = ref<TfaStep3Instance | null>(null)

// Step 3 验证事件
const onVerify = () => tfaProc.handleVerifyAndEnable(tfa, tfaStep3Ref.value)

onMounted(() => {
  tfaProc.initOnMount()
})
</script>

<style scoped>
.two-factor-page {
  padding: 20px;
}

.page-header {
  margin-bottom: 20px;
}

.page-title {
  margin: 0 0 8px 0;
  font-size: 24px;
  font-weight: 600;
}

.step-wrapper {
  margin-top: 30px;
  min-height: 300px;
}

.step-actions {
  display: flex;
  justify-content: center;
  gap: 12px;
  margin-top: 24px;
  padding-top: 20px;
  border-top: 1px solid #ebeef5;
}
</style>

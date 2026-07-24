<!--
  TwoFactorAuthStep4.vue - 2FA Step 4 完成面板（含 10 个恢复码 + 复制全部）
  拆分自 security/TwoFactorSetup.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="step-content">
    <el-result icon="success" title="两步验证已启用" sub-title="您的账户现在受到两步验证保护">
      <template #icon>
        <el-icon class="success-icon"><CircleCheckFilled /></el-icon>
      </template>
    </el-result>

    <el-alert
      title="恢复码（请妥善保管）"
      type="warning"
      description="当您无法使用身份验证器时，可使用以下恢复码登录。每个恢复码仅可使用一次。"
      show-icon
      :closable="false"
      class="recovery-alert"
    />

    <div class="recovery-codes">
      <el-input
        :model-value="recoveryCodesText"
        type="textarea"
        :rows="6"
        readonly
        resize="none"
      />
      <el-button class="copy-button" type="primary" @click="emit('copy-recovery')">
        <el-icon><CopyDocument /></el-icon>
        复制全部恢复码
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { CircleCheckFilled, CopyDocument } from '@element-plus/icons-vue'

const props = defineProps<{ recoveryCodes: string[] }>()
const emit = defineEmits<{ 'copy-recovery': []; 'finish': [] }>()

// 恢复码文本（用换行拼接，便于 textarea 显示）
const recoveryCodesText = computed<string>(() => {
  return (props.recoveryCodes || []).join('\n')
})
</script>

<style scoped>
.step-content {
  max-width: 600px;
  margin: 0 auto;
  padding: 30px 20px;
}

.success-icon {
  font-size: 60px;
  color: #67c23a;
}

.recovery-alert {
  margin-bottom: 16px;
  text-align: left;
}

.recovery-codes {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-top: 16px;
}

.copy-button {
  align-self: center;
}
</style>

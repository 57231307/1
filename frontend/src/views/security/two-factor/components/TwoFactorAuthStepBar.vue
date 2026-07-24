<!--
  TwoFactorAuthStepBar.vue - 2FA 设置步骤条（4 步）
  拆分自 security/TwoFactorSetup.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-steps :active="currentStep" finish-status="success" align-center>
    <el-step title="启动设置" />
    <el-step title="扫描二维码" />
    <el-step title="验证并启用" />
    <el-step title="完成" />
  </el-steps>
</template>

<script setup lang="ts">
defineProps<{ currentStep: number }>()
</script>

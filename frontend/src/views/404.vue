<template>
  <div class="error-page">
    <div class="error-content">
      <h1 class="error-code">404</h1>
      <p class="error-desc">{{ $t('error.page.notFoundDesc') }}</p>
      <el-button type="primary" @click="$router.push('/')">{{ $t('error.page.backHome') }}</el-button>
    </div>
  </div>
</template>

<script setup lang="ts"></script>

<style scoped>
.error-page {
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #f0f2f5;
}
.error-content {
  text-align: center;
}
.error-code {
  font-size: 120px;
  color: #409eff;
  margin: 0;
}
.error-desc {
  font-size: 24px;
  color: #606266;
  margin: 20px 0 40px;
}
</style>

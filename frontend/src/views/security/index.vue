<!--
  security/index.vue - 登录安全（拆分重构版）
  任务编号: P14 批 2 I-3 第 6 批
  拆分：547 行 → ~100 行 + 4 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="security-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">登录安全</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>系统管理</el-breadcrumb-item>
          <el-breadcrumb-item>登录安全</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button @click="secProc.handleExport(sec)">
          <el-icon><Download /></el-icon>
          导出日志
        </el-button>
      </div>
    </div>

    <SecStat :stats="sec.stats" />

    <SecLogTbl
      :data="sec.loginLogs"
      :loading="sec.loading"
      :total="sec.total"
      :query-params="sec.queryParams"
      @update:query-params="(v) => Object.assign(sec.queryParams, v)"
      @query="secProc.handleQuery(sec)"
      @size-or-current="(val, type) => type === 'size' ? secProc.handleSizeChange(val, sec) : secProc.handleCurrentChange(val, sec)"
    />

    <SecLockTbl
      :data="sec.lockedAccounts"
      :loading="sec.lockLoading"
      @unlock="(row) => secProc.handleUnlock(row, sec)"
    />

    <SecAlertTbl :data="sec.securityAlerts" :loading="sec.alertLoading" />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { Download } from '@element-plus/icons-vue'
import { useSec } from './composables/useSec'
import { useSecProc } from './composables/useSecProc'
import SecStat from './components/SecStat.vue'
import SecLogTbl from './components/SecLogTbl.vue'
import SecLockTbl from './components/SecLockTbl.vue'
import SecAlertTbl from './components/SecAlertTbl.vue'

// 业务状态
const sec = useSec()
const secProc = useSecProc()

onMounted(() => {
  sec.getStats()
  sec.getLoginLogs()
  sec.getLockedAccounts()
  sec.getSecurityAlerts()
})
</script>

<style scoped>
.security-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.page-title {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.table-card {
  margin-bottom: 20px;
}
</style>

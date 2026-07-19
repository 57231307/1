<!--
  SecAlertTbl.vue - 安全告警表（含类型/状态标签）
  拆分自 security/index.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover" class="table-card">
    <template #header>
      <div class="card-header">
        <span>安全告警</span>
      </div>
    </template>

    <el-table v-loading="loading" :data="data" border stripe aria-label="安全告警表">
      <el-table-column type="index" label="序号" width="60" align="center" />
      <el-table-column prop="alert_type" label="告警类型" width="120" align="center">
        <template #default="{ row }">
          <el-tag :type="getAlertType(row.alert_type)">{{ getAlertLabel(row.alert_type) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="username" label="用户名" width="120" show-overflow-tooltip />
      <el-table-column prop="ip_address" label="IP地址" width="150" show-overflow-tooltip />
      <el-table-column prop="description" label="告警描述" min-width="200" show-overflow-tooltip />
      <el-table-column prop="created_at" label="告警时间" width="180" align="center" />
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getAlertStatusType(row.status)">{{
            getAlertStatusLabel(row.status)
          }}</el-tag>
        </template>
      </el-table-column>
    </el-table>
  </el-card>
</template>

<script setup lang="ts">
import type { SecurityAlert } from '@/api/security'
import { getAlertType, getAlertLabel, getAlertStatusType, getAlertStatusLabel } from '../composables/secFmts'

defineProps<{ data: SecurityAlert[]; loading: boolean }>()
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>

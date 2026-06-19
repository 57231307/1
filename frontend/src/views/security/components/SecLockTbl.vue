<!--
  SecLockTbl.vue - 锁定账户管理表（含解锁操作）
  拆分自 security/index.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover" class="table-card">
    <template #header>
      <div class="card-header">
        <span>锁定账户管理</span>
      </div>
    </template>

    <el-table v-loading="loading" :data="data" border stripe>
      <el-table-column type="index" label="序号" width="60" align="center" />
      <el-table-column prop="username" label="用户名" width="120" show-overflow-tooltip />
      <el-table-column prop="lock_reason" label="锁定原因" min-width="200" show-overflow-tooltip />
      <el-table-column prop="locked_at" label="锁定时间" width="180" align="center" />
      <el-table-column prop="unlock_at" label="解锁时间" width="180" align="center" />
      <el-table-column label="操作" width="120" align="center">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="emit('unlock', row as any)">解锁</el-button>
        </template>
      </el-table-column>
    </el-table>
  </el-card>
</template>

<script setup lang="ts">
import type { LockedAccount } from '@/api/security'

defineProps<{ data: LockedAccount[]; loading: boolean }>()
const emit = defineEmits<{ unlock: [row: any] }>()
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>

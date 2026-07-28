<!--
  SecurityLockTable.vue - 锁定账户管理表（含解锁操作）
  拆分自 security/index.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
  批次 D05 B4：接入 useI18n
-->
<template>
  <el-card shadow="hover" class="table-card">
    <template #header>
      <div class="card-header">
        <span>{{ t('security.lockTable.title') }}</span>
      </div>
    </template>

    <el-table
      v-loading="loading"
      :data="data"
      border
      stripe
      :aria-label="t('security.lockTable.ariaLabel')"
    >
      <el-table-column
        type="index"
        :label="t('security.lockTable.column.index')"
        width="60"
        align="center"
      />
      <el-table-column
        prop="username"
        :label="t('security.lockTable.column.username')"
        width="120"
        show-overflow-tooltip
      />
      <el-table-column
        prop="lock_reason"
        :label="t('security.lockTable.column.lockReason')"
        min-width="200"
        show-overflow-tooltip
      />
      <el-table-column
        prop="locked_at"
        :label="t('security.lockTable.column.lockedAt')"
        width="180"
        align="center"
      />
      <el-table-column
        prop="unlock_at"
        :label="t('security.lockTable.column.unlockAt')"
        width="180"
        align="center"
      />
      <el-table-column :label="t('security.lockTable.column.action')" width="120" align="center">
        <template #default="{ row }">
          <el-button
            type="primary"
            link
            size="small"
            @click="emit('unlock', row as LockedAccount)"
            >{{ t('security.lockTable.button.unlock') }}</el-button
          >
        </template>
      </el-table-column>
    </el-table>
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { LockedAccount } from '@/api/security';

const { t } = useI18n({ useScope: 'global' });

defineProps<{ data: LockedAccount[]; loading: boolean }>();
const emit = defineEmits<{ unlock: [row: LockedAccount] }>();
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
</style>

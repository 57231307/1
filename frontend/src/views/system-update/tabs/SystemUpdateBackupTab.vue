<!--
  SystemUpdateBackupTab.vue - 系统备份 Tab
  来源：原 system-update/index.vue 中 backups tab
  拆分日期：2026-06-17 P1-3-Batch-5
  批次 283：接入 useTableApi 模式（page/pageSize props + v-model 绑定分页）
-->
<template>
  <el-card shadow="hover">
    <el-table
      v-loading="loading"
      :data="backups"
      stripe
      :aria-label="t('systemUpdate.backupTab.tableAriaLabel')"
    >
      <el-table-column
        prop="backup_code"
        :label="t('systemUpdate.backupTab.columnBackupCode')"
        width="140"
      />
      <el-table-column
        prop="backup_type"
        :label="t('systemUpdate.backupTab.columnBackupType')"
        width="100"
      >
        <template #default="{ row }">
          {{ getBackupTypeLabel(row.backup_type) }}
        </template>
      </el-table-column>
      <el-table-column
        prop="description"
        :label="t('systemUpdate.backupTab.columnDescription')"
        min-width="150"
        show-overflow-tooltip
      />
      <el-table-column
        prop="file_size"
        :label="t('systemUpdate.backupTab.columnFileSize')"
        width="100"
      >
        <template #default="{ row }">
          {{ formatFileSize(row.file_size) }}
        </template>
      </el-table-column>
      <el-table-column
        prop="status"
        :label="t('systemUpdate.backupTab.columnStatus')"
        width="100"
        align="center"
      >
        <template #default="{ row }">
          <el-tag :type="backupStatusTypeMap[row.status]" size="small">
            {{ getBackupStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column
        prop="created_by_name"
        :label="t('systemUpdate.backupTab.columnCreatedBy')"
        width="100"
      />
      <el-table-column
        prop="created_at"
        :label="t('systemUpdate.backupTab.columnCreatedAt')"
        width="160"
      />
      <el-table-column :label="t('systemUpdate.backupTab.columnActions')" width="250" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'completed'"
            type="primary"
            link
            size="small"
            @click="emit('download', row)"
            >{{ t('systemUpdate.backupTab.buttonDownload') }}</el-button
          >
          <el-button
            v-if="row.status === 'completed'"
            type="success"
            link
            size="small"
            @click="emit('restore', row)"
            >{{ t('systemUpdate.backupTab.buttonRestore') }}</el-button
          >
          <el-button type="danger" link size="small" @click="emit('delete', row)">{{
            t('systemUpdate.backupTab.buttonDelete')
          }}</el-button>
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-container">
      <el-pagination
        :current-page="page"
        :page-size="pageSize"
        :page-sizes="[10, 20, 50]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        :aria-label="t('systemUpdate.backupTab.paginationAriaLabel')"
        @update:current-page="(v: number) => emit('update:page', v)"
        @update:page-size="(v: number) => emit('update:page-size', v)"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { SystemBackup } from '@/api/system-update';

const { t } = useI18n({ useScope: 'global' });

defineProps<{
  backups: SystemBackup[];
  loading: boolean;
  total: number;
  page: number;
  pageSize: number;
  backupStatusTypeMap: Record<string, string>;
  formatFileSize: (size: number) => string;
}>();

const emit = defineEmits<{
  download: [row: SystemBackup];
  restore: [row: SystemBackup];
  delete: [row: SystemBackup];
  'update:page': [v: number];
  'update:page-size': [v: number];
}>();

/** 备份类型 → i18n 标签（语言切换响应） */
const getBackupTypeLabel = (type: string): string => {
  switch (type) {
    case 'full':
      return t('systemUpdate.common.backupTypeFull');
    case 'incremental':
      return t('systemUpdate.common.backupTypeIncremental');
    case 'database':
      return t('systemUpdate.common.backupTypeDatabase');
    case 'files':
      return t('systemUpdate.common.backupTypeFiles');
    default:
      return type;
  }
};

/** 备份状态 → i18n 标签（语言切换响应） */
const getBackupStatusLabel = (status: string): string => {
  switch (status) {
    case 'creating':
      return t('systemUpdate.common.backupStatusCreating');
    case 'completed':
      return t('systemUpdate.common.backupStatusCompleted');
    case 'failed':
      return t('systemUpdate.common.backupStatusFailed');
    default:
      return status;
  }
};
</script>

<style scoped>
.pagination-container {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>

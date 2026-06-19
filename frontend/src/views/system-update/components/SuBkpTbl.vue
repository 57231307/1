<!--
  SuBkpTbl.vue - 系统备份列表表格
  拆分自 system-update/index.vue（P14 批 2 I-3 第 1 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-card shadow="hover">
    <el-table v-loading="loading" :data="backups" stripe>
      <el-table-column prop="backup_code" label="备份编号" width="140" />
      <el-table-column prop="backup_type" label="备份类型" width="100">
        <template #default="{ row }">
          {{ getBackupTypeLabel(row.backup_type) }}
        </template>
      </el-table-column>
      <el-table-column
        prop="description"
        label="描述"
        min-width="150"
        show-overflow-tooltip
      />
      <el-table-column prop="file_size" label="文件大小" width="100">
        <template #default="{ row }">
          {{ formatFileSize(row.file_size) }}
        </template>
      </el-table-column>
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getBackupStatusType(row.status)" size="small">
            {{ getBackupStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="created_by_name" label="创建人" width="100" />
      <el-table-column prop="created_at" label="创建时间" width="160" />
      <el-table-column label="操作" width="250" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'completed'"
            type="primary"
            link
            size="small"
            @click="emit('download-backup', row as SystemBackup)"
            >下载</el-button
          >
          <el-button
            v-if="row.status === 'completed'"
            type="success"
            link
            size="small"
            @click="emit('restore', row as SystemBackup)"
            >恢复</el-button
          >
          <el-button
            type="danger"
            link
            size="small"
            @click="emit('delete', row as SystemBackup)"
            >删除</el-button
          >
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-container">
      <el-pagination
        :current-page="query.page"
        :page-size="query.page_size"
        :page-sizes="[10, 20, 50]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="emit('refresh')"
        @current-change="emit('refresh')"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import type { SystemBackup } from '@/api/system-update'
import {
  formatFileSize,
  getBackupTypeLabel,
  getBackupStatusLabel,
  getBackupStatusType,
} from '../composables/sysUpdFmts'

interface BkpQuery {
  page: number
  page_size: number
}

/**
 * 系统备份列表表格组件
 */
const props = defineProps<{
  backups: SystemBackup[]
  loading: boolean
  total: number
  query: BkpQuery
}>()

const emit = defineEmits<{
  'download-backup': [row: SystemBackup]
  restore: [row: SystemBackup]
  delete: [row: SystemBackup]
  refresh: []
}>()

void props
</script>

<style scoped>
.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>

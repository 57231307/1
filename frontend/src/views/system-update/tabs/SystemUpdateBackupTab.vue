<!--
  SystemUpdateBackupTab.vue - 系统备份 Tab
  来源：原 system-update/index.vue 中 backups tab
  拆分日期：2026-06-17 P1-3-Batch-5
-->
<template>
  <el-card shadow="hover">
    <el-table v-loading="loading" :data="backups" stripe>
      <el-table-column prop="backup_code" label="备份编号" width="140" />
      <el-table-column prop="backup_type" label="备份类型" width="100">
        <template #default="{ row }">
          {{ backupTypeMap[row.backup_type] }}
        </template>
      </el-table-column>
      <el-table-column prop="description" label="描述" min-width="150" show-overflow-tooltip />
      <el-table-column prop="file_size" label="文件大小" width="100">
        <template #default="{ row }">
          {{ formatFileSize(row.file_size) }}
        </template>
      </el-table-column>
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="backupStatusTypeMap[row.status]" size="small">
            {{ backupStatusMap[row.status] }}
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
            @click="emit('download', row)"
            >下载</el-button
          >
          <el-button
            v-if="row.status === 'completed'"
            type="success"
            link
            size="small"
            @click="emit('restore', row)"
            >恢复</el-button
          >
          <el-button type="danger" link size="small" @click="emit('delete', row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-container">
      <el-pagination
        v-model:current-page="localQuery.page"
        v-model:page-size="localQuery.page_size"
        :page-sizes="[10, 20, 50]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="emit('fetch')"
        @current-change="emit('fetch')"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue'
import type { SystemBackup } from '@/api/system-update'

export interface BackupQuery {
  page: number
  page_size: number
}

const props = defineProps<{
  backups: SystemBackup[]
  loading: boolean
  total: number
  queryParams: BackupQuery
  backupTypeMap: Record<string, string>
  backupStatusTypeMap: Record<string, string>
  backupStatusMap: Record<string, string>
  formatFileSize: (size: number) => string
}>()

const emit = defineEmits<{
  download: [row: SystemBackup]
  restore: [row: SystemBackup]
  delete: [row: SystemBackup]
  fetch: []
  'update:queryParams': [value: BackupQuery]
}>()

const localQuery = reactive<BackupQuery>({ ...props.queryParams })

watch(
  () => props.queryParams,
  newQuery => {
    Object.assign(localQuery, newQuery)
  },
  { deep: true }
)
</script>

<style scoped>
.pagination-container {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>

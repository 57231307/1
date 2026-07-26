<!--
  SystemUpdateVersionTab.vue - 系统更新版本列表 Tab
  来源：原 system-update/index.vue 中 versions tab
  拆分日期：2026-06-17 P1-3-Batch-5
  批次 283：接入 useTableApi 模式（page/pageSize props + v-model 绑定分页）
-->
<template>
  <el-card shadow="hover">
    <el-table v-loading="loading" :data="versions" stripe :aria-label="t('systemUpdate.versionTab.tableAriaLabel')">
      <el-table-column prop="version" :label="t('systemUpdate.versionTab.columnVersion')" width="120" />
      <el-table-column prop="release_date" :label="t('systemUpdate.versionTab.columnReleaseDate')" width="120" />
      <el-table-column
        prop="release_notes"
        :label="t('systemUpdate.versionTab.columnReleaseNotes')"
        min-width="200"
        show-overflow-tooltip
      />
      <el-table-column prop="file_size" :label="t('systemUpdate.versionTab.columnFileSize')" width="100">
        <template #default="{ row }">
          {{ formatFileSize(row.file_size) }}
        </template>
      </el-table-column>
      <el-table-column prop="status" :label="t('systemUpdate.versionTab.columnStatus')" width="120" align="center">
        <template #default="{ row }">
          <el-tag :type="versionStatusTypeMap[row.status]" size="small">
            {{ getVersionStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column :label="t('systemUpdate.versionTab.columnActions')" width="200" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'available'"
            type="primary"
            link
            size="small"
            @click="emit('download', row)"
            >{{ t('systemUpdate.versionTab.buttonDownload') }}</el-button
          >
          <el-button
            v-if="row.status === 'downloaded'"
            type="success"
            link
            size="small"
            @click="emit('install', row)"
            >{{ t('systemUpdate.versionTab.buttonInstall') }}</el-button
          >
          <el-button type="info" link size="small" @click="emit('view-detail', row)"
            >{{ t('systemUpdate.versionTab.buttonDetail') }}</el-button
          >
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
        @update:current-page="(v: number) => emit('update:page', v)"
        @update:page-size="(v: number) => emit('update:page-size', v)"
        :aria-label="t('systemUpdate.versionTab.paginationAriaLabel')"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { SystemVersion } from '@/api/system-update'

const { t } = useI18n({ useScope: 'global' })

defineProps<{
  versions: SystemVersion[]
  loading: boolean
  total: number
  page: number
  pageSize: number
  versionStatusTypeMap: Record<string, string>
  formatFileSize: (size: number) => string
}>()

const emit = defineEmits<{
  download: [row: SystemVersion]
  install: [row: SystemVersion]
  'view-detail': [row: SystemVersion]
  'update:page': [v: number]
  'update:page-size': [v: number]
}>()

/** 版本状态 → i18n 标签（语言切换响应） */
const getVersionStatusLabel = (status: string): string => {
  switch (status) {
    case 'available':
      return t('systemUpdate.common.versionStatusAvailable')
    case 'downloading':
      return t('systemUpdate.common.versionStatusDownloading')
    case 'downloaded':
      return t('systemUpdate.common.versionStatusDownloaded')
    case 'installing':
      return t('systemUpdate.common.versionStatusInstalling')
    case 'installed':
      return t('systemUpdate.common.versionStatusInstalled')
    case 'failed':
      return t('systemUpdate.common.versionStatusFailed')
    default:
      return status
  }
}
</script>

<style scoped>
.pagination-container {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>

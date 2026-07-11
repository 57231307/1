<!--
  SystemUpdateVersionTab.vue - 系统更新版本列表 Tab
  来源：原 system-update/index.vue 中 versions tab
  拆分日期：2026-06-17 P1-3-Batch-5
  批次 283：接入 useTableApi 模式（page/pageSize props + v-model 绑定分页）
-->
<template>
  <el-card shadow="hover">
    <el-table v-loading="loading" :data="versions" stripe>
      <el-table-column prop="version" label="版本号" width="120" />
      <el-table-column prop="release_date" label="发布日期" width="120" />
      <el-table-column
        prop="release_notes"
        label="更新说明"
        min-width="200"
        show-overflow-tooltip
      />
      <el-table-column prop="file_size" label="文件大小" width="100">
        <template #default="{ row }">
          {{ formatFileSize(row.file_size) }}
        </template>
      </el-table-column>
      <el-table-column prop="status" label="状态" width="120" align="center">
        <template #default="{ row }">
          <el-tag :type="versionStatusTypeMap[row.status]" size="small">
            {{ versionStatusMap[row.status] }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="200" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'available'"
            type="primary"
            link
            size="small"
            @click="emit('download', row)"
            >下载</el-button
          >
          <el-button
            v-if="row.status === 'downloaded'"
            type="success"
            link
            size="small"
            @click="emit('install', row)"
            >安装</el-button
          >
          <el-button type="info" link size="small" @click="emit('view-detail', row)"
            >详情</el-button
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
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import type { SystemVersion } from '@/api/system-update'

defineProps<{
  versions: SystemVersion[]
  loading: boolean
  total: number
  page: number
  pageSize: number
  versionStatusTypeMap: Record<string, string>
  versionStatusMap: Record<string, string>
  formatFileSize: (size: number) => string
}>()

const emit = defineEmits<{
  download: [row: SystemVersion]
  install: [row: SystemVersion]
  'view-detail': [row: SystemVersion]
  'update:page': [v: number]
  'update:page-size': [v: number]
}>()
</script>

<style scoped>
.pagination-container {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>

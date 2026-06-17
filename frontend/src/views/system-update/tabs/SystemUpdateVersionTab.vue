<!--
  SystemUpdateVersionTab.vue - 系统更新版本列表 Tab
  来源：原 system-update/index.vue 中 versions tab
  拆分日期：2026-06-17 P1-3-Batch-5
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

export interface VersionQuery {
  page: number
  page_size: number
}

const props = defineProps<{
  versions: any[]
  loading: boolean
  total: number
  queryParams: VersionQuery
  versionStatusTypeMap: Record<string, string>
  versionStatusMap: Record<string, string>
  formatFileSize: (size: number) => string
}>()

const emit = defineEmits<{
  download: [row: any]
  install: [row: any]
  'view-detail': [row: any]
  fetch: []
  'update:queryParams': [value: VersionQuery]
}>()

const localQuery = reactive<VersionQuery>({ ...props.queryParams })

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

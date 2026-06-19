<!--
  SuVerTbl.vue - 系统版本列表表格
  拆分自 system-update/index.vue（P14 批 2 I-3 第 1 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
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
          <el-tag :type="getVersionStatusType(row.status)" size="small">
            {{ getVersionStatusLabel(row.status) }}
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
            @click="emit('download', row as SystemVersion)"
            >下载</el-button
          >
          <el-button
            v-if="row.status === 'downloaded'"
            type="success"
            link
            size="small"
            @click="emit('install', row as SystemVersion)"
            >安装</el-button
          >
          <el-button
            type="info"
            link
            size="small"
            @click="emit('view-detail', row as SystemVersion)"
            >详情</el-button
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
import type { SystemVersion } from '@/api/system-update'
import { formatFileSize, getVersionStatusLabel, getVersionStatusType } from '../composables/sysUpdFmts'

interface VerQuery {
  page: number
  page_size: number
}

/**
 * 系统版本列表表格组件
 * 仅做展示，行内操作通过 emit 通知父组件
 */
const props = defineProps<{
  // 列表数据
  versions: SystemVersion[]
  // 加载中
  loading: boolean
  // 总数
  total: number
  // 分页
  query: VerQuery
}>()

const emit = defineEmits<{
  // 下载
  download: [row: SystemVersion]
  // 安装
  install: [row: SystemVersion]
  // 查看详情
  'view-detail': [row: SystemVersion]
  // 刷新
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

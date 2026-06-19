<!--
  BpmDfTbl.vue - BPM 流程定义列表
  拆分自 bpm/definitions.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card class="table-card">
    <el-table v-loading="loading" :data="data" border stripe>
      <el-table-column prop="process_key" label="流程标识" min-width="140" />
      <el-table-column prop="process_name" label="流程名称" min-width="180" />
      <el-table-column prop="category" label="分类" width="100">
        <template #default="{ row }">
          {{ getCategoryText(row.category) }}
        </template>
      </el-table-column>
      <el-table-column prop="version" label="当前版本" width="100" align="center" />
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">
            {{ getStatusText(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="description" label="描述" min-width="150" show-overflow-tooltip />
      <el-table-column prop="created_at" label="创建时间" min-width="160" />
      <el-table-column label="操作" width="320" fixed="right">
        <template #default="{ row }">
          <el-button size="small" @click="emit('edit', row)">编辑</el-button>
          <el-button size="small" type="primary" @click="emit('versions', row)">版本</el-button>
          <el-button size="small" type="success" @click="emit('save-as-template', row)">
            保存为模板
          </el-button>
          <el-button size="small" type="danger" @click="emit('delete', row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-pagination
      :current-page="pagination.page"
      :page-size="pagination.page_size"
      :total="total"
      :page-sizes="[10, 20, 50, 100]"
      layout="total, sizes, prev, pager, next, jumper"
      @update:current-page="(v: number) => emit('update:page', v)"
      @update:page-size="(v: number) => emit('update:size', v)"
      @size-change="emit('reload')"
      @current-change="emit('reload')"
    />
  </el-card>
</template>

<script setup lang="ts">
import { getCategoryText, getStatusType, getStatusText } from '../composables/bpmDfFmts'
import type { ProcessDefinition } from '@/api/bpm-enhanced'

interface Pagination {
  page: number
  page_size: number
}

/**
 * 列表组件
 */
defineProps<{
  // 列表数据
  data: ProcessDefinition[]
  // 总数
  total: number
  // 加载状态
  loading: boolean
  // 分页
  pagination: Pagination
}>()

const emit = defineEmits<{
  edit: [row: ProcessDefinition]
  versions: [row: ProcessDefinition]
  'save-as-template': [row: ProcessDefinition]
  delete: [row: ProcessDefinition]
  reload: []
  'update:page': [v: number]
  'update:size': [v: number]
}>()
</script>

<style scoped>
.table-card {
  margin-bottom: 20px;
}
.el-pagination {
  margin-top: 20px;
  justify-content: flex-end;
}
</style>

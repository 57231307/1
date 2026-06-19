<!--
  DiTplTbl.vue - 数据导入模板列表 + 过滤栏
  拆分自 data-import/index.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-card shadow="hover">
    <div class="filter-container">
      <el-input
        :model-value="query.keyword"
        placeholder="搜索模板编号/名称"
        style="width: 200px"
        clearable
        @update:model-value="(v: string) => (query.keyword = v)"
        @clear="emit('search')"
        @keyup.enter="emit('search')"
      />
      <el-select
        :model-value="query.module"
        placeholder="模块"
        clearable
        style="width: 120px"
        @update:model-value="(v: string) => (query.module = v)"
      >
        <el-option label="客户" value="customer" />
        <el-option label="供应商" value="supplier" />
        <el-option label="产品" value="product" />
        <el-option label="库存" value="inventory" />
        <el-option label="销售" value="sales" />
        <el-option label="采购" value="purchase" />
        <el-option label="财务" value="finance" />
      </el-select>
      <el-button type="primary" @click="emit('search')">
        <el-icon><Search /></el-icon>
        搜索
      </el-button>
    </div>

    <el-table v-loading="loading" :data="data" stripe>
      <el-table-column prop="template_code" label="模板编号" width="140" />
      <el-table-column prop="template_name" label="模板名称" min-width="180" />
      <el-table-column prop="module" label="模块" width="100">
        <template #default="{ row }">
          {{ MODULE_MAP[row.module] }}
        </template>
      </el-table-column>
      <el-table-column prop="file_format" label="文件格式" width="100">
        <template #default="{ row }">
          {{ row.file_format.toUpperCase() }}
        </template>
      </el-table-column>
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
            {{ row.status === 'active' ? '启用' : '停用' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="created_at" label="创建时间" width="160" />
      <el-table-column label="操作" width="250" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="emit('download', row)"
            >下载模板</el-button
          >
          <el-button type="primary" link size="small" @click="emit('upload', row)"
            >导入数据</el-button
          >
          <el-button type="primary" link size="small" @click="emit('edit', row)">编辑</el-button>
          <el-button type="danger" link size="small" @click="emit('delete', row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-container">
      <el-pagination
        :current-page="query.page"
        :page-size="query.page_size"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @update:current-page="onPageChange"
        @update:page-size="onSizeChange"
        @size-change="emit('search')"
        @current-change="emit('search')"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { Search } from '@element-plus/icons-vue'
import type { ImportTemplate } from '@/api/data-import'
import { MODULE_MAP } from '../composables/diFmts'

// 查询参数类型
interface TplQuery {
  page: number
  page_size: number
  keyword: string
  module: string
}

/**
 * 模板列表组件（含过滤栏）
 */
defineProps<{
  // 模板数据
  data: ImportTemplate[]
  // 总数
  total: number
  // 加载状态
  loading: boolean
  // 查询参数
  query: TplQuery
}>()

const emit = defineEmits<{
  search: []
  edit: [row: ImportTemplate]
  delete: [row: ImportTemplate]
  download: [row: ImportTemplate]
  upload: [row: ImportTemplate]
  // 分页
  'update:page': [v: number]
  'update:size': [v: number]
}>()

/** 页码变更 */
const onPageChange = (page: number) => {
  emit('update:page', page)
}

/** 每页大小变更 */
const onSizeChange = (size: number) => {
  emit('update:size', size)
}
</script>

<style scoped>
.filter-container {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}
.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>

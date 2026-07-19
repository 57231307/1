<!--
  DiTplTbl.vue - 数据导入模板列表 + 过滤栏
  拆分自 data-import/index.vue（P14 批 2 I-3 第 5 批）
  批次 289：改造为 localQuery + handleSearch 模式，接入 useTableApi queryParams
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <div class="filter-container">
      <el-input
        v-model="localQuery.keyword"
        placeholder="搜索模板编号/名称"
        style="width: 200px"
        clearable
        @keyup.enter="handleSearch"
      />
      <el-select
        v-model="localQuery.module"
        placeholder="模块"
        clearable
        style="width: 120px"
      >
        <el-option label="客户" value="customer" />
        <el-option label="供应商" value="supplier" />
        <el-option label="产品" value="product" />
        <el-option label="库存" value="inventory" />
        <el-option label="销售" value="sales" />
        <el-option label="采购" value="purchase" />
        <el-option label="财务" value="finance" />
      </el-select>
      <el-button type="primary" @click="handleSearch">
        <el-icon><Search /></el-icon>
        搜索
      </el-button>
    </div>

    <el-table v-loading="loading" :data="data" stripe aria-label="数据导入模板列表">
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
        :current-page="page"
        :page-size="pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @update:current-page="(v: number) => emit('update:page', v)"
        @update:page-size="(v: number) => emit('update:page-size', v)"
        aria-label="数据导入模板分页"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import { Search } from '@element-plus/icons-vue'
import type { ImportTemplate } from '@/api/data-import'
import { MODULE_MAP } from '../composables/diFmts'

/**
 * 模板列表组件（含过滤栏）
 * 接收父组件传入的 queryParams + page/pageSize，通过 emit 同步变更
 * 查询时先同步 queryParams 再触发 fetch
 */
const props = defineProps<{
  // 模板数据
  data: ImportTemplate[]
  // 总数
  total: number
  // 加载状态
  loading: boolean
  // 查询条件（由父组件 useTableApi 管理，类型放宽为 Record 兼容 useTableApi）
  queryParams: Record<string, unknown>
  // 当前页码
  page: number
  // 每页大小
  pageSize: number
}>()

const emit = defineEmits<{
  edit: [row: ImportTemplate]
  delete: [row: ImportTemplate]
  download: [row: ImportTemplate]
  upload: [row: ImportTemplate]
  // 触发查询（父组件监听后调用 handleTemplateSearch 重置页码并加载）
  fetch: []
  // 同步查询条件到父组件
  'update:queryParams': [params: Record<string, unknown>]
  // 分页变化（由 useTableApi watch 自动加载）
  'update:page': [page: number]
  'update:page-size': [pageSize: number]
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localQuery = reactive({
  keyword: (props.queryParams.keyword as string) ?? '',
  module: (props.queryParams.module as string) ?? '',
})

/** 查询：先同步筛选条件到父组件，再触发 fetch */
const handleSearch = () => {
  emit('update:queryParams', { ...localQuery })
  emit('fetch')
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

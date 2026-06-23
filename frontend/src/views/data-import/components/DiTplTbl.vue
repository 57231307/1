<!--
  DiTplTbl.vue - 数据导入模板列表 + 过滤栏
  拆分自 data-import/index.vue（P14 批 2 I-3 第 5 批）
  P9-3 批次 F Pattern A 重构：本地 reactive 镜像 + watch 同步 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <div class="filter-container">
      <el-input
        :model-value="localParams.keyword"
        placeholder="搜索模板编号/名称"
        style="width: 200px"
        clearable
        @update:model-value="(v: string) => { localParams.keyword = v; syncToParent() }"
        @clear="emit('search')"
        @keyup.enter="emit('search')"
      />
      <el-select
        :model-value="localParams.module"
        placeholder="模块"
        clearable
        style="width: 120px"
        @update:model-value="(v: string) => { localParams.module = v; syncToParent() }"
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
        :current-page="localParams.page"
        :page-size="localParams.page_size"
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
import { reactive, watch } from 'vue'
import { Search } from '@element-plus/icons-vue'
import type { ImportTemplate } from '@/api/data-import'
import { MODULE_MAP } from '../composables/diFmts'
import type { TplQuery } from '../composables/useDi'

// 查询参数默认值（父组件未传入时使用）
const DEFAULT_PARAMS: TplQuery = {
  page: 1,
  page_size: 20,
  keyword: '',
  module: '',
}

/**
 * 模板列表组件（含过滤栏）
 */
const props = defineProps<{
  // 模板数据
  data: ImportTemplate[]
  // 总数
  total: number
  // 加载状态
  loading: boolean
  // 查询参数（由父组件管理，子组件通过 emit('update:params') 整体回写）
  params?: TplQuery
}>()

const emit = defineEmits<{
  search: []
  edit: [row: ImportTemplate]
  delete: [row: ImportTemplate]
  download: [row: ImportTemplate]
  upload: [row: ImportTemplate]
  // 整体回写查询参数（父组件监听后 Object.assign 到自己的 params）
  'update:params': [params: TplQuery]
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localParams = reactive<TplQuery>({
  ...(props.params ?? DEFAULT_PARAMS),
})

// 父组件传参变化时同步到本地（如父组件重置分页/过滤条件）
watch(
  () => props.params,
  newParams => {
    if (newParams) Object.assign(localParams, newParams)
  },
  { deep: true },
)

/** 同步本地到父组件（深拷贝避免外部引用被意外修改） */
const syncToParent = () => {
  emit('update:params', { ...localParams })
}

/** 页码变更 */
const onPageChange = (page: number) => {
  localParams.page = page
  syncToParent()
}

/** 每页大小变更 */
const onSizeChange = (size: number) => {
  localParams.page_size = size
  syncToParent()
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

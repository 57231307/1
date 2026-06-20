<!--
  SupplierList.vue - 供应商列表子组件
  来源：原 supplier/index.vue 中 列表+筛选区（line 28-99）
  拆分日期：2026-06-17 P1-3-Batch-6
-->
<template>
  <div>
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="localQuery" class="filter-form">
        <el-form-item label="关键词">
          <el-input v-model="localQuery.keyword" placeholder="供应商编码/名称" clearable />
        </el-form-item>
        <el-form-item label="等级">
          <el-select v-model="localQuery.grade" placeholder="选择等级" clearable>
            <el-option label="A级" value="A" />
            <el-option label="B级" value="B" />
            <el-option label="C级" value="C" />
            <el-option label="D级" value="D" />
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="localQuery.status" placeholder="选择状态" clearable>
            <el-option label="启用" value="active" />
            <el-option label="禁用" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="suppliers" stripe>
        <el-table-column prop="supplier_code" label="供应商编码" width="120" fixed />
        <el-table-column prop="supplier_name" label="供应商名称" min-width="180" fixed />
        <el-table-column prop="supplier_short_name" label="简称" width="100" />
        <el-table-column prop="contact_phone" label="联系电话" width="130" />
        <el-table-column prop="email" label="邮箱" width="180" show-overflow-tooltip />
        <el-table-column prop="grade" label="等级" width="80">
          <template #default="{ row }">
            <el-tag :type="getGradeTag(row.grade)" size="small">
              {{ row.grade || '-' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="supplier_type" label="类型" width="100" />
        <el-table-column prop="status" label="状态" width="80">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="$emit('edit', row)"
              >编辑</el-button
            >
            <el-button type="danger" link size="small" @click="$emit('delete', row)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="localQuery.page"
          v-model:page-size="localQuery.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleQuery"
          @current-change="handleQuery"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue'

const props = defineProps<{
  suppliers: any[]
  total: number
  loading: boolean
  queryParams: any
  dialogMode: 'list' | 'view' | 'add' | 'edit'
}>()

const emit = defineEmits<{
  search: []
  reset: []
  'update:queryParams': [value: any]
  add: []
  view: [row: any]
  edit: [row: any]
  delete: [row: any]
}>()

const localQuery = reactive({ ...props.queryParams })

/** 等级 → Element Plus Tag 类型映射 */
const getGradeTag = (grade: string): 'success' | 'warning' | 'danger' | 'info' => {
  if (grade === 'A') return 'success'
  if (grade === 'D') return 'danger'
  return 'warning'
}

watch(
  () => props.queryParams,
  newQ => Object.assign(localQuery, newQ),
  { deep: true }
)

const handleQuery = () => {
  emit('update:queryParams', { ...localQuery, page: 1 })
  emit('search')
}

const handleReset = () => {
  localQuery.keyword = ''
  localQuery.grade = ''
  localQuery.status = ''
  localQuery.page = 1
  localQuery.page_size = 20
  handleQuery()
}
</script>

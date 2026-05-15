<template>
  <div class="departments-page">
    <div class="header">
      <h2>部门管理</h2>
      <el-button type="primary" @click="handleCreate">新建部门</el-button>
    </div>

    <el-table :data="departmentList" v-loading="loading" border>
      <el-table-column prop="name" label="部门名称" />
      <el-table-column prop="code" label="部门编码" />
      <el-table-column prop="parentId" label="上级部门" />
      <el-table-column prop="manager" label="负责人" />
      <el-table-column prop="status" label="状态">
        <template #default="{ row }">
          <el-tag :type="row.status === 'active' ? 'success' : 'danger'">
            {{ row.status === 'active' ? '启用' : '禁用' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="200">
        <template #default="{ row }">
          <el-button size="small" @click="handleEdit(row)">编辑</el-button>
          <el-button size="small" type="danger" @click="handleDelete(row)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { departmentApi } from '@/api/department'

const loading = ref(false)
const departmentList = ref<any[]>([])

const loadDepartments = async () => {
  loading.value = true
  try {
    const res = await departmentApi.list()
    departmentList.value = res.data.list || []
  } finally {
    loading.value = false
  }
}

const handleCreate = () => {
  // TODO: 打开新建对话框
}

const handleEdit = (row: any) => {
  // TODO: 打开编辑对话框
}

const handleDelete = async (row: any) => {
  if (!row.id) return
  try {
    await departmentApi.delete(row.id)
    await loadDepartments()
  } catch (error) {
    console.error('删除失败:', error)
  }
}

onMounted(() => {
  loadDepartments()
})
</script>

<style scoped>
.departments-page {
  padding: 20px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
</style>

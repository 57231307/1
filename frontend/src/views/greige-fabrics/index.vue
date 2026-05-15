<template>
  <div class="greige-fabrics-page">
    <div class="header">
      <h2>坯布管理</h2>
      <el-button type="primary" @click="handleCreate">新建坯布</el-button>
    </div>

    <el-table :data="greigeList" v-loading="loading" border>
      <el-table-column prop="batchNo" label="坯布批号" />
      <el-table-column prop="productId" label="产品 ID" />
      <el-table-column prop="quantity" label="数量" />
      <el-table-column prop="unit" label="单位" />
      <el-table-column prop="warehouseId" label="仓库 ID" />
      <el-table-column prop="status" label="状态">
        <template #default="{ row }">
          <el-tag :type="row.status === 'available' ? 'success' : 'warning'">
            {{ row.status === 'available' ? '可用' : '已占用' }}
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
import { greigeFabricApi } from '@/api/greige-fabric'

const loading = ref(false)
const greigeList = ref<any[]>([])

const loadGreigeFabrics = async () => {
  loading.value = true
  try {
    const res = await greigeFabricApi.list()
    greigeList.value = res.data.list || []
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
    await greigeFabricApi.delete(row.id)
    await loadGreigeFabrics()
  } catch (error) {
    console.error('删除失败:', error)
  }
}

onMounted(() => {
  loadGreigeFabrics()
})
</script>

<style scoped>
.greige-fabrics-page {
  padding: 20px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
</style>

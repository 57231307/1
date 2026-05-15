<template>
  <div class="fixed-assets-page">
    <div class="header">
      <h2>固定资产管理</h2>
      <el-button type="primary" @click="handleCreate">新增资产</el-button>
    </div>

    <el-table :data="assetList" v-loading="loading" border>
      <el-table-column prop="assetCode" label="资产编码" />
      <el-table-column prop="assetName" label="资产名称" />
      <el-table-column prop="category" label="资产类别" />
      <el-table-column prop="originalValue" label="原值" />
      <el-table-column prop="netValue" label="净值" />
      <el-table-column prop="purchaseDate" label="购置日期" />
      <el-table-column prop="status" label="状态">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">
            {{ getStatusLabel(row.status) }}
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
import { fixedAssetApi } from '@/api/asset'

const loading = ref(false)
const assetList = ref<any[]>([])

const loadAssets = async () => {
  loading.value = true
  try {
    const res = await fixedAssetApi.list()
    assetList.value = res.data.list || []
  } finally {
    loading.value = false
  }
}

const getStatusType = (status: string) => {
  const types: Record<string, any> = {
    'IN_USE': 'success',
    'IDLE': 'warning',
    'SCRAPPED': 'danger'
  }
  return types[status] || 'info'
}

const getStatusLabel = (status: string) => {
  const labels: Record<string, string> = {
    'IN_USE': '使用中',
    'IDLE': '闲置',
    'SCRAPPED': '已报废'
  }
  return labels[status] || status
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
    await fixedAssetApi.delete(row.id)
    await loadAssets()
  } catch (error) {
    console.error('删除失败:', error)
  }
}

onMounted(() => {
  loadAssets()
})
</script>

<style scoped>
.fixed-assets-page {
  padding: 20px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
</style>

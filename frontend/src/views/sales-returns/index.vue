<template>
  <div class="sales-returns-page">
    <div class="header">
      <h2>销售退货管理</h2>
      <el-button type="primary" @click="handleCreate">新建退货单</el-button>
    </div>

    <el-table :data="returnList" v-loading="loading" border>
      <el-table-column prop="returnNo" label="退货单号" />
      <el-table-column prop="salesOrderNo" label="销售订单号" />
      <el-table-column prop="customerName" label="客户名称" />
      <el-table-column prop="returnDate" label="退货日期" />
      <el-table-column prop="totalAmount" label="退货金额" />
      <el-table-column prop="status" label="状态">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">
            {{ getStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="250">
        <template #default="{ row }">
          <el-button size="small" @click="handleView(row)">详情</el-button>
          <el-button size="small" @click="handleEdit(row)">编辑</el-button>
          <el-button size="small" type="primary" @click="handleApprove(row)" v-if="row.status === 'PENDING'">审核</el-button>
        </template>
      </el-table-column>
    </el-table>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { salesReturnApi } from '@/api/sales-return'

const loading = ref(false)
const returnList = ref<any[]>([])

const loadReturns = async () => {
  loading.value = true
  try {
    const res = await salesReturnApi.list()
    returnList.value = res.data.list || []
  } finally {
    loading.value = false
  }
}

const getStatusType = (status: string) => {
  const types: Record<string, any> = {
    'PENDING': 'warning',
    'APPROVED': 'success',
    'REJECTED': 'danger',
    'COMPLETED': 'info'
  }
  return types[status] || 'info'
}

const getStatusLabel = (status: string) => {
  const labels: Record<string, string> = {
    'PENDING': '待审核',
    'APPROVED': '已通过',
    'REJECTED': '已拒绝',
    'COMPLETED': '已完成'
  }
  return labels[status] || status
}

const handleCreate = () => {
  // TODO: 打开新建对话框
}

const handleView = (row: any) => {
  // TODO: 查看详情
}

const handleEdit = (row: any) => {
  // TODO: 打开编辑对话框
}

const handleApprove = async (row: any) => {
  if (!row.id) return
  try {
    await salesReturnApi.approve(row.id)
    await loadReturns()
  } catch (error) {
    console.error('审核失败:', error)
  }
}

onMounted(() => {
  loadReturns()
})
</script>

<style scoped>
.sales-returns-page {
  padding: 20px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
</style>

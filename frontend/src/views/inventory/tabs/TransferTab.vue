<!--
  TransferTab.vue - 库存调拨 Tab
  来源：原 inventory/index.vue 中 库存调拨 tab 内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="transfer-tab">
    <el-card shadow="hover">
      <div class="transfer-actions">
        <el-button type="primary" @click="handleNewTransfer">
          <el-icon><Plus /></el-icon>
          新建调拨单
        </el-button>
      </div>
      <el-table v-loading="loading" :data="transfers" stripe>
        <el-table-column prop="transfer_no" label="调拨单号" width="160" />
        <el-table-column prop="from_warehouse_name" label="调出仓库" width="120" />
        <el-table-column prop="to_warehouse_name" label="调入仓库" width="120" />
        <el-table-column prop="total_quantity" label="调拨数量" width="100" align="right" />
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="getTransferStatusType(row.status)" size="small">
              {{ getTransferStatusText(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="creator_name" label="创建人" width="100" />
        <el-table-column prop="created_at" label="创建时间" width="160" />
        <el-table-column label="操作" width="150">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleViewTransfer(row)"
              >详情</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="handleApproveTransfer(row)"
              >审批</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, defineEmits } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'

interface TransferRow {
  id: number
  transfer_no: string
  from_warehouse_name: string
  to_warehouse_name: string
  total_quantity: number
  status: string
  creator_name: string
  created_at: string
}

const emit = defineEmits<{ 'new-transfer': [] }>()

const transfers = ref<TransferRow[]>([])
const loading = ref(false)

const getTransferStatusType = (status: string) => {
  const typeMap: Record<string, string> = {
    pending: 'warning',
    approved: 'success',
    executed: 'primary',
    cancelled: 'info',
  }
  return typeMap[status] || 'info'
}

const getTransferStatusText = (status: string) => {
  const textMap: Record<string, string> = {
    pending: '待审批',
    approved: '已审批',
    executed: '已执行',
    cancelled: '已取消',
  }
  return textMap[status] || status
}

const fetchTransfers = async () => {
  loading.value = true
  try {
    const { inventoryApi } = await import('@/api/inventory')
    const res = await inventoryApi.getTransfers({ page: 1, page_size: 50 })
    transfers.value = res.data?.list || []
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '获取调拨记录失败')
    transfers.value = []
  } finally {
    loading.value = false
  }
}

const handleNewTransfer = () => {
  emit('new-transfer')
}

const handleViewTransfer = (row: TransferRow) => {
  ElMessage.info(`查看调拨单 ${row.transfer_no}`)
}

const handleApproveTransfer = (row: TransferRow) => {
  ElMessage.success(`审批通过调拨单 ${row.transfer_no}`)
}

onMounted(() => {
  fetchTransfers()
})
</script>

<style scoped>
.transfer-actions {
  margin-bottom: 16px;
}
</style>

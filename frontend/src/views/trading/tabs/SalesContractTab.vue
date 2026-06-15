<!--
  SalesContractTab.vue - 销售合同 Tab
  来源：原 trading/index.vue 中 销售合同 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="sales-contract-tab">
    <div class="page-header">
      <h2 class="page-title">销售合同管理</h2>
      <el-button type="primary" @click="openSalesContractDialog()">
        <el-icon><Plus /></el-icon> 新建合同
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table v-loading="salesContractLoading" :data="salesContracts" stripe>
        <el-table-column prop="contract_no" label="合同编号" width="140" />
        <el-table-column prop="customer_name" label="客户" width="150" />
        <el-table-column prop="contract_date" label="合同日期" width="120" />
        <el-table-column prop="total_amount" label="总金额" width="120" align="right">
          <template #default="{ row }">{{ formatMoney(row.total_amount) }}</template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getContractStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button
              type="primary"
              link
              size="small"
              @click="viewSalesContract(row as unknown as TradingContract)"
              >查看</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              type="success"
              link
              size="small"
              @click="approveSalesContract(row as unknown as TradingContract)"
              >审批</el-button
            >
            <el-button
              v-if="row.status === 'approved'"
              type="warning"
              link
              size="small"
              @click="executeSalesContract(row as unknown as TradingContract)"
              >执行</el-button
            >
            <el-button
              type="danger"
              link
              size="small"
              @click="deleteSalesContract(row as unknown as TradingContract)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  listTradingContracts,
  createTradingContract,
  approveTradingContract,
  executeTradingContract,
  deleteTradingContract,
  type TradingContract,
} from '@/api/trading-contract'

const salesContracts = ref<TradingContract[]>([])
const salesContractLoading = ref(false)

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    pending: 'warning',
    approved: 'primary',
    executed: 'success',
    completed: 'success',
    cancelled: 'danger',
  }
  return map[status] || 'info'
}

const getContractStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    pending: '待审核',
    approved: '已审核',
    executed: '已执行',
    completed: '已完成',
    cancelled: '已取消',
  }
  return map[status] || status
}

const fetchSalesContracts = async () => {
  salesContractLoading.value = true
  try {
    const res = await listTradingContracts({ type: 'sales' })
    const d = res.data as
      | { list?: TradingContract[]; items?: TradingContract[] }
      | TradingContract[]
      | undefined
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      salesContracts.value = d.list || d.items || []
    } else {
      salesContracts.value = (d as TradingContract[]) || []
    }
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '获取销售合同失败')
  } finally {
    salesContractLoading.value = false
  }
}

const openSalesContractDialog = async () => {
  try {
    await createTradingContract({ type: 'sales', status: 'draft' })
    ElMessage.success('已创建草稿')
    fetchSalesContracts()
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '创建失败')
  }
}

const viewSalesContract = (row: TradingContract) => {
  ElMessage.info(`查看销售合同: ${row.contract_no}`)
}

const approveSalesContract = async (row: TradingContract) => {
  try {
    await ElMessageBox.confirm('确定审批该销售合同吗？', '确认', { type: 'info' })
    await approveTradingContract(row.id)
    ElMessage.success('审批成功')
    fetchSalesContracts()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const executeSalesContract = async (row: TradingContract) => {
  try {
    await ElMessageBox.confirm('确定执行该销售合同吗？', '确认', { type: 'info' })
    await executeTradingContract(row.id)
    ElMessage.success('执行成功')
    fetchSalesContracts()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const deleteSalesContract = async (row: TradingContract) => {
  try {
    await ElMessageBox.confirm('确定删除该销售合同吗？', '确认', { type: 'warning' })
    await deleteTradingContract(row.id)
    ElMessage.success('删除成功')
    fetchSalesContracts()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

defineExpose({ refresh: fetchSalesContracts })

onMounted(() => {
  fetchSalesContracts()
})
</script>

<style scoped>
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
</style>

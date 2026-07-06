<!--
  PurchaseContractTab.vue - 采购合同 Tab
  来源：原 trading/index.vue 中 采购合同 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="purchase-contract-tab">
    <div class="page-header">
      <h2 class="page-title">采购合同管理</h2>
      <el-button type="primary" @click="openPurchaseContractDialog()">
        <el-icon><Plus /></el-icon> 新建合同
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table v-loading="purchaseContractLoading" :data="purchaseContracts" stripe>
        <el-table-column prop="contract_no" label="合同编号" width="140" />
        <el-table-column prop="supplier_name" label="供应商" width="150" />
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
              @click="viewPurchaseContract(row as unknown as TradingContract)"
              >查看</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              type="success"
              link
              size="small"
              @click="approvePurchaseContract(row as unknown as TradingContract)"
              >审批</el-button
            >
            <el-button
              v-if="row.status === 'approved'"
              type="warning"
              link
              size="small"
              @click="executePurchaseContract(row as unknown as TradingContract)"
              >执行</el-button
            >
            <el-button
              type="danger"
              link
              size="small"
              @click="deletePurchaseContract(row as unknown as TradingContract)"
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
  getTradingContract,
  createTradingContract,
  approveTradingContract,
  executeTradingContract,
  deleteTradingContract,
  type TradingContract,
} from '@/api/trading-contract'

const purchaseContracts = ref<TradingContract[]>([])
const purchaseContractLoading = ref(false)

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

const fetchPurchaseContracts = async () => {
  purchaseContractLoading.value = true
  try {
    const res = await listTradingContracts({ type: 'purchase' })
    const d = res.data as
      | { list?: TradingContract[]; items?: TradingContract[] }
      | TradingContract[]
      | undefined
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      purchaseContracts.value = d.list || d.items || []
    } else {
      purchaseContracts.value = (d as TradingContract[]) || []
    }
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '获取采购合同失败')
  } finally {
    purchaseContractLoading.value = false
  }
}

const openPurchaseContractDialog = async () => {
  try {
    await createTradingContract({ type: 'purchase', status: 'draft' })
    ElMessage.success('已创建草稿')
    fetchPurchaseContracts()
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '创建失败')
  }
}

// 批次 157a P1-1 修复：接入 getTradingContract API 展示采购合同详情
const viewPurchaseContract = async (row: TradingContract) => {
  try {
    const res = await getTradingContract(row.id)
    const d = res.data
    if (!d) {
      ElMessage.warning('未找到合同详情')
      return
    }
    const lines = [
      `合同编号：${d.contract_no}`,
      `供应商：${d.supplier_name || '-'}`,
      `合同日期：${d.contract_date}`,
      `合同金额：¥${formatMoney(d.total_amount)}`,
      `当前状态：${getContractStatusLabel(d.status)}`,
    ]
    await ElMessageBox.alert(lines.join('\n'), '采购合同详情', {
      confirmButtonText: '关闭',
    })
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '获取合同详情失败')
  }
}

const approvePurchaseContract = async (row: TradingContract) => {
  try {
    await ElMessageBox.confirm('确定审批该采购合同吗？', '确认', { type: 'info' })
    await approveTradingContract(row.id)
    ElMessage.success('审批成功')
    fetchPurchaseContracts()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const executePurchaseContract = async (row: TradingContract) => {
  try {
    await ElMessageBox.confirm('确定执行该采购合同吗？', '确认', { type: 'info' })
    await executeTradingContract(row.id)
    ElMessage.success('执行成功')
    fetchPurchaseContracts()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const deletePurchaseContract = async (row: TradingContract) => {
  try {
    await ElMessageBox.confirm('确定删除该采购合同吗？', '确认', { type: 'warning' })
    await deleteTradingContract(row.id)
    ElMessage.success('删除成功')
    fetchPurchaseContracts()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

defineExpose({ refresh: fetchPurchaseContracts })

onMounted(() => {
  fetchPurchaseContracts()
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

<!--
  SalesReturnTab.vue - 销售退货 Tab
  来源：原 trading/index.vue 中 销售退货 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="sales-return-tab">
    <div class="page-header">
      <h2 class="page-title">销售退货管理</h2>
      <el-button type="primary" @click="openSalesReturnDialog()">
        <el-icon><Plus /></el-icon> 新建退货
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table v-loading="salesReturnLoading" :data="salesReturns" stripe>
        <el-table-column prop="return_no" label="退货单号" width="140" />
        <el-table-column prop="customer_name" label="客户" width="150" />
        <el-table-column prop="return_date" label="退货日期" width="120" />
        <el-table-column prop="total_amount" label="金额" width="120" align="right">
          <template #default="{ row }">{{ formatMoney(row.total_amount) }}</template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getReturnStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button
              type="primary"
              link
              size="small"
              @click="viewSalesReturn(row as unknown as TradingReturn)"
              >查看</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              type="success"
              link
              size="small"
              @click="approveSalesReturn(row as unknown as TradingReturn)"
              >审批</el-button
            >
            <el-button
              type="danger"
              link
              size="small"
              @click="deleteSalesReturn(row as unknown as TradingReturn)"
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
  listTradingReturns,
  createTradingReturn,
  approveTradingReturn,
  deleteTradingReturn,
  type TradingReturn,
} from '@/api/trading-return'

const salesReturns = ref<TradingReturn[]>([])
const salesReturnLoading = ref(false)

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    pending: 'warning',
    approved: 'primary',
    completed: 'success',
    cancelled: 'danger',
  }
  return map[status] || 'info'
}

const getReturnStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    pending: '待审核',
    approved: '已审核',
    completed: '已完成',
    cancelled: '已取消',
  }
  return map[status] || status
}

const fetchSalesReturns = async () => {
  salesReturnLoading.value = true
  try {
    const res = await listTradingReturns({ type: 'sales' })
    const d = res.data as
      | { list?: TradingReturn[]; items?: TradingReturn[] }
      | TradingReturn[]
      | undefined
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      salesReturns.value = d.list || d.items || []
    } else {
      salesReturns.value = (d as TradingReturn[]) || []
    }
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '获取销售退货失败')
  } finally {
    salesReturnLoading.value = false
  }
}

const openSalesReturnDialog = async () => {
  try {
    await createTradingReturn({ type: 'sales', status: 'draft' })
    ElMessage.success('已创建草稿')
    fetchSalesReturns()
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '创建失败')
  }
}

const viewSalesReturn = (row: TradingReturn) => {
  ElMessage.info(`查看销售退货: ${row.return_no}`)
}

const approveSalesReturn = async (row: TradingReturn) => {
  try {
    await ElMessageBox.confirm('确定审批该销售退货吗？', '确认', { type: 'info' })
    await approveTradingReturn(row.id)
    ElMessage.success('审批成功')
    fetchSalesReturns()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const deleteSalesReturn = async (row: TradingReturn) => {
  try {
    await ElMessageBox.confirm('确定删除该销售退货吗？', '确认', { type: 'warning' })
    await deleteTradingReturn(row.id)
    ElMessage.success('删除成功')
    fetchSalesReturns()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

defineExpose({ refresh: fetchSalesReturns })

onMounted(() => {
  fetchSalesReturns()
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

<template>
  <div class="purchase-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">采购管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>采购管理</el-breadcrumb-item>
          <el-breadcrumb-item>采购订单</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建采购单
        </el-button>
        <el-button @click="handlePrint">
          <el-icon><Printer /></el-icon>
          打印
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon order-icon">
              <el-icon><Document /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">本月采购</div>
              <div class="stat-value">{{ stats.monthOrders }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card highlight">
          <div class="stat-content">
            <div class="stat-icon amount-icon">
              <el-icon><Money /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">采购金额</div>
              <div class="stat-value">{{ formatCurrency(stats.monthAmount) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card warning">
          <div class="stat-content">
            <div class="stat-icon pending-icon">
              <el-icon><Clock /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">待收货</div>
              <div class="stat-value">{{ stats.pendingReceipt }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon supplier-icon">
              <el-icon><OfficeBuilding /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">合作供应商</div>
              <div class="stat-value">{{ stats.supplierCount }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input v-model="queryParams.keyword" placeholder="订单号/供应商名" clearable @clear="handleQuery" />
        </el-form-item>
        <el-form-item label="供应商">
          <el-select v-model="queryParams.supplier_id" placeholder="选择供应商" clearable @change="handleQuery">
            <el-option v-for="s in suppliers" :key="s.id" :label="s.name" :value="s.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="订单状态">
          <el-select v-model="queryParams.status" placeholder="选择状态" clearable @change="handleQuery">
            <el-option label="待审批" value="pending" />
            <el-option label="已审批" value="approved" />
            <el-option label="部分收货" value="partial" />
            <el-option label="已完成" value="completed" />
            <el-option label="已取消" value="cancelled" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            查询
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            重置
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="orders" stripe>
        <el-table-column prop="order_no" label="订单号" width="160" fixed>
          <template #default="{ row }">
            <el-link type="primary" @click="handleView(row)">{{ row.order_no }}</el-link>
          </template>
        </el-table-column>
        <el-table-column prop="supplier_name" label="供应商" width="180" fixed />
        <el-table-column prop="order_date" label="订单日期" width="120" />
        <el-table-column prop="required_date" label="要求交货日期" width="120" />
        <el-table-column prop="total_amount" label="订单金额" width="120" align="right">
          <template #default="{ row }">
            <span class="amount">¥{{ row.total_amount.toLocaleString() }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="received_amount" label="已收货金额" width="120" align="right">
          <template #default="{ row }">
            <span>¥{{ (row.received_amount || 0).toLocaleString() }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="payment_status" label="付款状态" width="100">
          <template #default="{ row }">
            <el-tag :type="getPaymentStatusType(row.payment_status)" size="small">
              {{ getPaymentStatusText(row.payment_status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="订单状态" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getStatusText(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="creator_name" label="创建人" width="100" />
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">详情</el-button>
            <el-button v-if="row.status === 'approved'" type="warning" link size="small" @click="handleReceive(row)">收货</el-button>
            <el-button v-if="row.status === 'pending'" type="success" link size="small" @click="handleApprove(row)">审批</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="queryParams.page"
          v-model:page-size="queryParams.page_size"
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
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Search, Refresh, Document, Money, Clock, OfficeBuilding, Printer, Download } from '@element-plus/icons-vue'
import printJS from 'print-js'
import { purchaseApi, type PurchaseOrder } from '@/api/purchase'
import { supplierApi, type Supplier } from '@/api/supplier'

const loading = ref(false)
const orders = ref<PurchaseOrder[]>([])
const suppliers = ref<Supplier[]>([])
const total = ref(0)

const stats = ref({
  monthOrders: 0,
  monthAmount: 0,
  pendingReceipt: 0,
  supplierCount: 0
})

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  supplier_id: undefined as number | undefined,
  status: ''
})

const formatCurrency = (amount: number) => {
  return new Intl.NumberFormat('zh-CN', { style: 'currency', currency: 'CNY', minimumFractionDigits: 0 }).format(amount)
}

const getStatusType = (status: string) => {
  const typeMap: Record<string, any> = { pending: 'warning', approved: 'primary', partial: 'info', completed: 'success', cancelled: 'danger' }
  return typeMap[status] || 'info'
}

const getStatusText = (status: string) => {
  const textMap: Record<string, string> = { pending: '待审批', approved: '已审批', partial: '部分收货', completed: '已完成', cancelled: '已取消' }
  return textMap[status] || status
}

const getPaymentStatusType = (status: string) => {
  const typeMap: Record<string, any> = { unpaid: 'danger', partial: 'warning', paid: 'success' }
  return typeMap[status] || 'info'
}

const getPaymentStatusText = (status: string) => {
  const textMap: Record<string, string> = { unpaid: '未付款', partial: '部分付款', paid: '已付款' }
  return textMap[status] || status
}

const fetchData = async () => {
  loading.value = true
  try {
    const res = await purchaseApi.getOrderList(queryParams)
    orders.value = res.data?.list || []
    total.value = res.data?.total || 0
    
    // 计算统计数据
    stats.value.monthOrders = total.value
    stats.value.monthAmount = orders.value.reduce((sum, o) => sum + (o.total_amount || 0), 0)
    stats.value.pendingReceipt = orders.value.filter(o => o.status === 'approved').length
  } catch (error: any) {
    ElMessage.error(error.message || '获取采购单列表失败')
    orders.value = []
    total.value = 0
  } finally {
    loading.value = false
  }
}

const fetchSuppliers = async () => {
  try {
    const res = await supplierApi.list({ page_size: 1000 })
    suppliers.value = res.data?.list || []
    stats.value.supplierCount = suppliers.value.length
  } catch (error) {
    console.error('获取供应商列表失败:', error)
  }
}

const handleQuery = () => { queryParams.page = 1; fetchData() }
const handleReset = () => { queryParams.keyword = ''; queryParams.supplier_id = undefined; queryParams.status = ''; handleQuery() }

const handleCreate = () => { 
  ElMessage.info('新建采购单功能开发中') 
}

const handleView = (row: PurchaseOrder) => {
  ElMessage.info(`查看采购单: ${row.order_no}`)
}

const handleApprove = async (row: PurchaseOrder) => {
  try {
    await ElMessageBox.confirm(`确定审批通过采购单 ${row.order_no} 吗？`, '审批确认', { type: 'success' })
    await purchaseApi.approveOrder(row.id)
    ElMessage.success(`采购单 ${row.order_no} 审批成功`)
    fetchData()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '审批失败')
    }
  }
}

const handleReceive = (row: PurchaseOrder) => {
  ElMessage.info(`收货功能开发中: ${row.order_no}`)
}
const handlePrint = () => {
  const printData = orders.value.map((item: any, index: number) => ({
    '序号': index + 1,
    '订单号': item.order_no,
    '供应商': item.supplier_name,
    '金额': `¥${item.total_amount}`,
    '状态': getStatusText(item.status),
    '创建时间': item.created_at
  }))
  printJS({
    printable: printData,
    properties: Object.keys(printData[0] || {}),
    type: 'table' as any,
    header: '采购订单列表',
    style: 'padding: 20px; font-size: 14px;',
    headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
    gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
    gridStyle: 'border-collapse: collapse; width: 100%;'
  })
}

const handleExport = () => {
  const csvContent = [
    ['订单号', '供应商', '金额', '状态', '创建时间'],
    ...orders.value.map((item: any) => [item.order_no, item.supplier_name, item.total_amount, getStatusText(item.status), item.created_at])
  ].map((row: any[]) => row.map((cell: any) => `"${cell}"`).join(',')).join('\n')
  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `采购订单_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
}
const handleView = (row: any) => {
  ElMessageBox({
    title: '采购单详情',
    message: `订单号: ${row.order_no}`,
    confirmButtonText: '关闭'
  })
}
const handleApprove = async (row: any) => {
  try {
    await ElMessageBox.confirm(`确定审批通过采购单 ${row.order_no} 吗？`, '审批确认', { type: 'success' })
    ElMessage.success(`采购单 ${row.order_no} 审批成功`)
    fetchData()
  } catch {}
}
const handleReceive = (_row: any) => { ElMessage.info('创建收货单功能开发中') }

onMounted(() => { fetchData(); fetchSuppliers() })
</script>

<style scoped>
.purchase-page { padding: 24px; background-color: #f5f7fa; min-height: 100%; }
.page-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 24px; }
.header-left .page-title { font-size: 28px; font-weight: 600; color: #303133; margin: 0 0 12px 0; }
.header-actions { display: flex; gap: 12px; }
.stats-row { margin-bottom: 20px; }
.stat-card { border-radius: 12px; transition: all 0.3s ease; }
.stat-card:hover { transform: translateY(-4px); box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12); }
.stat-card.highlight { background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); }
.stat-card.highlight .stat-icon { background: rgba(255, 255, 255, 0.2); color: white; }
.stat-card.highlight .stat-label, .stat-card.highlight .stat-value { color: white; }
.stat-card.warning { background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%); }
.stat-card.warning .stat-icon { background: rgba(255, 255, 255, 0.2); color: white; }
.stat-card.warning .stat-label, .stat-card.warning .stat-value { color: white; }
.stat-content { display: flex; align-items: center; gap: 16px; }
.stat-icon { width: 56px; height: 56px; border-radius: 12px; display: flex; align-items: center; justify-content: center; font-size: 28px; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; }
.stat-icon.order-icon { background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%); }
.stat-icon.amount-icon { background: rgba(255, 255, 255, 0.2); color: white; }
.stat-icon.pending-icon { background: rgba(255, 255, 255, 0.2); color: white; }
.stat-icon.supplier-icon { background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%); }
.stat-info { flex: 1; }
.stat-label { font-size: 14px; color: #909399; margin-bottom: 4px; }
.stat-value { font-size: 28px; font-weight: 700; color: #303133; line-height: 1.2; }
.filter-card { margin-bottom: 20px; }
.table-card { margin-bottom: 20px; }
.pagination-wrapper { margin-top: 20px; display: flex; justify-content: flex-end; }
.amount { font-weight: 600; color: #f56c6c; }
:deep(.el-card__header) { padding: 16px 20px; border-bottom: 1px solid #ebeef5; }
:deep(.el-card__body) { padding: 20px; }
</style>

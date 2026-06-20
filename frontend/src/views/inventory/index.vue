<!--
  inventory/index.vue - 库存管理主入口（拆分重构版）
  任务编号: P14 批 2 I-3 第 8 批
  拆分：600 行 → ~280 行 + 3 子组件 + 1 工具
  原 899 行已拆为 tabs/ 子组件，本批再拆统计卡片 + 2 个对话框 + 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="inventory-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">库存管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>仓储管理</el-breadcrumb-item>
          <el-breadcrumb-item>库存台账</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleAdjustment">
          <el-icon><Edit /></el-icon>
          库存调整
        </el-button>
        <el-button @click="handleTransfer">
          <el-icon><RefreshRight /></el-icon>
          库存调拨
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

    <StatCards :stats="stats" />

    <el-tabs v-model="activeTab" @tab-change="handleTabChange">
      <el-tab-pane label="库存台账" name="stock">
        <InventoryStockTab
          :stocks="stocks"
          :total="total"
          :loading="loading"
          :query-params="queryParams"
          :warehouses="warehouses"
          @view="handleView"
          @query="fetchData"
          @reset="handleReset"
          @update:query-params="(v: StockQuery) => Object.assign(queryParams, v)"
        />
      </el-tab-pane>

      <el-tab-pane label="库存预警" name="alert">
        <InventoryAlertTab :alerts="alerts" @purchase="handlePurchase" />
      </el-tab-pane>

      <el-tab-pane label="库存调拨" name="transfer">
        <InventoryTransferTab
          :transfers="transfers"
          @new-transfer="handleNewTransfer"
          @view-transfer="handleViewTransfer"
          @approve-transfer="handleApproveTransfer"
        />
      </el-tab-pane>
    </el-tabs>

    <AdjustmentDialog
      v-model:visible="adjustmentDialogVisible"
      :initial-form="adjustmentForm"
      @submit="onSubmitAdjustment"
    />

    <TransferDialog
      v-model:visible="transferDialogVisible"
      :initial-form="transferForm"
      :warehouses="warehouses"
      @add-item="handleAddTransferItem"
      @remove-item="handleRemoveTransferItem"
      @submit="onSubmitTransfer"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Edit, RefreshRight, Download, Printer } from '@element-plus/icons-vue'
import printJS from 'print-js'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { escapeCsvCell } from './composables/invFmts'
import InventoryStockTab, { type StockQuery } from './tabs/InventoryStockTab.vue'
import InventoryAlertTab from './tabs/InventoryAlertTab.vue'
import InventoryTransferTab from './tabs/InventoryTransferTab.vue'
import StatCards from './components/StatCards.vue'
import AdjustmentDialog from './components/AdjustmentDialog.vue'
import TransferDialog from './components/TransferDialog.vue'

const hasLoaded = createLazyLoader()

const loading = ref(false)
const activeTab = ref('stock')
const stocks = ref<any[]>([])
const alerts = ref<any[]>([])
const transfers = ref<any[]>([])
const warehouses = ref<any[]>([])
const total = ref(0)

const stats = ref({
  totalQuantity: 0,
  alertCount: 0,
  warehouseCount: 0,
  lowStockCount: 0,
})

const queryParams = reactive<StockQuery>({
  page: 1,
  page_size: 20,
  keyword: '',
  warehouse_id: undefined,
  status: '',
})

const fetchData = async () => {
  loading.value = true
  try {
    const { inventoryApi } = await import('@/api/inventory')
    const res = await inventoryApi.getStockList(queryParams)
    stocks.value = res.data?.list || []
    total.value = res.data?.total || 0

    const summaryRes = await inventoryApi.getInventoryReport({})
    const summary = summaryRes.data?.summary || {}
    stats.value = {
      totalQuantity: summary.total_quantity || 0,
      alertCount: summary.alert_count || 0,
      warehouseCount: summary.warehouse_count || 0,
      lowStockCount: summary.low_stock_count || 0,
    }
  } catch (error: any) {
    ElMessage.error(error.message || '获取库存列表失败')
    stocks.value = []
    total.value = 0
  } finally {
    loading.value = false
  }
}

const fetchAlerts = async () => {
  try {
    const { inventoryApi } = await import('@/api/inventory')
    const res = await inventoryApi.getStockAlerts()
    alerts.value = res.data || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取库存预警失败')
    alerts.value = []
  }
}

const fetchTransfers = async () => {
  try {
    const { inventoryApi } = await import('@/api/inventory')
    const res = await inventoryApi.getTransfers(queryParams)
    transfers.value = res.data?.list || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取调拨记录失败')
    transfers.value = []
  }
}

const fetchWarehouses = async () => {
  try {
    const { warehouseApi } = await import('@/api/warehouse')
    const res = await warehouseApi.list({ page: 1, page_size: 1000 })
    warehouses.value = res.data?.list || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取仓库列表失败')
    warehouses.value = []
  }
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.warehouse_id = undefined
  queryParams.status = ''
  queryParams.page = 1
  fetchData()
}

const handleTabChange = (tabName: string) => {
  if (tabName === 'alert') {
    fetchAlerts()
  } else if (tabName === 'transfer') {
    fetchTransfers()
  }
}

const adjustmentDialogVisible = ref(false)
const adjustmentForm = ref({
  stock_id: null as number | null,
  product_id: null as number | null,
  warehouse_id: null as number | null,
  product_name: '',
  warehouse_name: '',
  current_quantity: 0,
  adjustment_type: 'increase',
  adjustment_quantity: 0,
  reason: '',
})

const transferDialogVisible = ref(false)
const transferForm = ref({
  from_warehouse_id: null as number | null,
  to_warehouse_id: null as number | null,
  items: [{ product_id: null as number | null, quantity: 0 }],
  remark: '',
})

const handleAdjustment = () => {
  adjustmentForm.value = {
    stock_id: null,
    product_id: null,
    warehouse_id: null,
    product_name: '',
    warehouse_name: '',
    current_quantity: 0,
    adjustment_type: 'increase',
    adjustment_quantity: 0,
    reason: '',
  }
  adjustmentDialogVisible.value = true
}

const handleSubmitAdjustment = async (form: any) => {
  if (!form.adjustment_quantity || form.adjustment_quantity <= 0) {
    ElMessage.warning('请输入有效的调整数量')
    return
  }
  if (!form.reason) {
    ElMessage.warning('请输入调整原因')
    return
  }
  try {
    const { inventoryApi } = await import('@/api/inventory')
    await inventoryApi.createStockAdjustment({
      warehouse_id: form.warehouse_id!,
      product_id: form.product_id!,
      adjustment_type: form.adjustment_type as 'increase' | 'decrease',
      adjustment_quantity: form.adjustment_quantity,
      reason: form.reason,
    })
    ElMessage.success('库存调整成功')
    adjustmentDialogVisible.value = false
    fetchData()
  } catch (error: any) {
    ElMessage.error(error.message || '库存调整失败')
  }
}

const handleTransfer = () => {
  transferForm.value = {
    from_warehouse_id: null,
    to_warehouse_id: null,
    items: [{ product_id: null, quantity: 0 }],
    remark: '',
  }
  transferDialogVisible.value = true
}

const handleAddTransferItem = () => {
  transferForm.value.items.push({ product_id: null, quantity: 0 })
}
const handleRemoveTransferItem = (index: number) => {
  if (transferForm.value.items.length > 1) {
    transferForm.value.items.splice(index, 1)
  }
}
const handleSubmitTransfer = async (form: any) => {
  if (!form.from_warehouse_id || !form.to_warehouse_id) {
    ElMessage.warning('请选择调出/调入仓库')
    return
  }
  try {
    const { inventoryApi } = await import('@/api/inventory')
    await inventoryApi.createTransfer(form as any)
    ElMessage.success('调拨单创建成功')
    transferDialogVisible.value = false
    if (activeTab.value === 'transfer') {
      fetchTransfers()
    }
  } catch (error: any) {
    ElMessage.error(error.message || '创建调拨单失败')
  }
}

const handleNewTransfer = () => handleTransfer()
const handleViewTransfer = (row: any) => {
  ElMessage.info(`查看调拨单：${row.transfer_no}`)
}
const handleApproveTransfer = (row: any) => {
  ElMessage.info(`审批调拨单：${row.transfer_no}`)
}
const handleView = (row: any) => {
  ElMessage.info(`查看库存：${row.product_name}`)
}
const handlePurchase = (row: any) => {
  ElMessage.info(`采购：${row.product_name}`)
}
const handlePrint = () => {
  printJS({
    printable: stocks.value,
    properties: ['product_code', 'product_name', 'warehouse_name', 'quantity'],
    type: 'table' as any,
    header: '库存台账',
  })
}
const handleExport = () => {
  const csv = [
    ['产品编码', '产品名称', '仓库', '数量', '状态'],
    ...stocks.value.map(s => [
      s.product_code,
      s.product_name,
      s.warehouse_name,
      s.quantity,
      s.status,
    ]),
  ]
    .map(r => r.map(escapeCsvCell).join(','))
    .join('\n')
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `库存台账_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
}

const initPage = () => {
  loadIfNot('fetchData', fetchData, hasLoaded)
  loadIfNot('fetchWarehouses', fetchWarehouses, hasLoaded)
}

onMounted(() => {
  initPage()
})
</script>

<style scoped>
.inventory-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}

.header-left .page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}

.header-actions {
  display: flex;
  gap: 12px;
}
</style>

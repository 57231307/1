<!--
  inventory/index.vue - 库存管理主入口（Tab 容器）
  ----------------------------------------------------------------
  拆分说明（2026-06-17 P1-3-Batch-3）：
  原 899 行"上帝组件"已拆分为以下独立子组件：
  - tabs/InventoryStockTab.vue（库存台账 Tab，158 行）
  - tabs/InventoryAlertTab.vue（库存预警 Tab，51 行）
  - tabs/InventoryTransferTab.vue（库存调拨 Tab，84 行）

  本主入口承担：Tab 容器 + 统计卡片 + 2 个对话框（库存调整/新建调拨单）。
  通过 props/emit 通信。
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

    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon total-icon">
              <el-icon><Box /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">库存总量</div>
              <div class="stat-value">{{ formatNumber(stats.totalQuantity) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card warning">
          <div class="stat-content">
            <div class="stat-icon alert-icon">
              <el-icon><Warning /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">库存预警</div>
              <div class="stat-value">{{ stats.alertCount }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon warehouse-icon">
              <el-icon><OfficeBuilding /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">仓库数量</div>
              <div class="stat-value">{{ stats.warehouseCount }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card danger">
          <div class="stat-content">
            <div class="stat-icon low-icon">
              <el-icon><WarningFilled /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">低于最小库存</div>
              <div class="stat-value">{{ stats.lowStockCount }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

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

    <!-- 库存调整对话框 -->
    <el-dialog
      v-model="adjustmentDialogVisible"
      title="库存调整"
      width="500px"
      :close-on-click-modal="false"
    >
      <el-form :model="adjustmentForm" label-width="100px">
        <el-form-item v-if="adjustmentForm.product_name" label="产品">
          <el-input :value="adjustmentForm.product_name" disabled />
        </el-form-item>
        <el-form-item v-if="adjustmentForm.warehouse_name" label="仓库">
          <el-input :value="adjustmentForm.warehouse_name" disabled />
        </el-form-item>
        <el-form-item v-if="adjustmentForm.current_quantity" label="当前库存">
          <el-input :value="adjustmentForm.current_quantity" disabled />
        </el-form-item>
        <el-form-item label="调整类型">
          <el-radio-group v-model="adjustmentForm.adjustment_type">
            <el-radio value="increase">增加</el-radio>
            <el-radio value="decrease">减少</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="调整数量">
          <el-input-number
            v-model="adjustmentForm.adjustment_quantity"
            :min="1"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="调整原因">
          <el-input
            v-model="adjustmentForm.reason"
            type="textarea"
            :rows="3"
            placeholder="请输入调整原因"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="adjustmentDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmitAdjustment">确定</el-button>
      </template>
    </el-dialog>

    <!-- 新建调拨单对话框 -->
    <el-dialog
      v-model="transferDialogVisible"
      title="新建调拨单"
      width="700px"
      :close-on-click-modal="false"
    >
      <el-form :model="transferForm" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="调出仓库">
              <el-select
                v-model="transferForm.from_warehouse_id"
                placeholder="请选择调出仓库"
                style="width: 100%"
              >
                <el-option
                  v-for="wh in warehouses"
                  :key="wh.id"
                  :label="wh.warehouse_name || wh.name"
                  :value="wh.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="调入仓库">
              <el-select
                v-model="transferForm.to_warehouse_id"
                placeholder="请选择调入仓库"
                style="width: 100%"
              >
                <el-option
                  v-for="wh in warehouses"
                  :key="wh.id"
                  :label="wh.warehouse_name || wh.name"
                  :value="wh.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-divider content-position="left">调拨产品</el-divider>
        <div
          v-for="(item, index) in transferForm.items"
          :key="index"
          style="display: flex; gap: 10px; margin-bottom: 10px"
        >
          <el-input-number v-model="item.quantity" :min="1" placeholder="数量" style="flex: 1" />
          <el-button
            type="danger"
            :icon="Delete"
            circle
            :disabled="transferForm.items.length <= 1"
            @click="handleRemoveTransferItem(index)"
          />
        </div>
        <el-button type="primary" link @click="handleAddTransferItem">
          <el-icon><Plus /></el-icon>
          添加产品
        </el-button>
        <el-form-item label="备注" style="margin-top: 16px">
          <el-input
            v-model="transferForm.remark"
            type="textarea"
            :rows="2"
            placeholder="请输入备注"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="transferDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmitTransfer">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import {
  Box,
  Warning,
  Edit,
  RefreshRight,
  Download,
  Printer,
  OfficeBuilding,
  WarningFilled,
  Plus,
  Delete,
} from '@element-plus/icons-vue'
import printJS from 'print-js'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import InventoryStockTab, { type StockQuery } from './tabs/InventoryStockTab.vue'
import InventoryAlertTab from './tabs/InventoryAlertTab.vue'
import InventoryTransferTab from './tabs/InventoryTransferTab.vue'

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

const formatNumber = (num: number) => num.toLocaleString()

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

const handleQuery = () => {
  queryParams.page = 1
  fetchData()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.warehouse_id = undefined
  queryParams.status = ''
  handleQuery()
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

const handleSubmitAdjustment = async () => {
  if (!adjustmentForm.value.adjustment_quantity || adjustmentForm.value.adjustment_quantity <= 0) {
    ElMessage.warning('请输入有效的调整数量')
    return
  }
  if (!adjustmentForm.value.reason) {
    ElMessage.warning('请输入调整原因')
    return
  }
  try {
    const { inventoryApi } = await import('@/api/inventory')
    await inventoryApi.createStockAdjustment({
      warehouse_id: adjustmentForm.value.warehouse_id!,
      product_id: adjustmentForm.value.product_id!,
      adjustment_type: adjustmentForm.value.adjustment_type as 'increase' | 'decrease',
      adjustment_quantity: adjustmentForm.value.adjustment_quantity,
      reason: adjustmentForm.value.reason,
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
const handleSubmitTransfer = async () => {
  if (!transferForm.value.from_warehouse_id || !transferForm.value.to_warehouse_id) {
    ElMessage.warning('请选择调出/调入仓库')
    return
  }
  try {
    const { inventoryApi } = await import('@/api/inventory')
    await inventoryApi.createTransfer(transferForm.value)
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
    ...stocks.value.map(s => [s.product_code, s.product_name, s.warehouse_name, s.quantity, s.status]),
  ]
    .map(r => r.map(c => `"${c}"`).join(','))
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

.stats-row {
  margin-bottom: 20px;
}

.stat-card {
  border-radius: 12px;
  transition: all 0.3s ease;
}

.stat-card.warning {
  background: linear-gradient(135deg, #f5576c 0%, #ff6f6f 100%);
  color: white;
}

.stat-card.danger {
  background: linear-gradient(135deg, #fa709a 0%, #fee140 100%);
}

.stat-content {
  display: flex;
  align-items: center;
  gap: 16px;
}

.stat-icon {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 28px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.stat-info {
  flex: 1;
}

.stat-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 4px;
}

.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}
</style>

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
        <el-card shadow="hover" class="filter-card">
          <el-form :inline="true" :model="queryParams" class="filter-form">
            <el-form-item label="关键词">
              <el-input
                v-model="queryParams.keyword"
                placeholder="产品编码/名称"
                clearable
                @clear="handleQuery"
              />
            </el-form-item>
            <el-form-item label="仓库">
              <el-select
                v-model="queryParams.warehouse_id"
                placeholder="选择仓库"
                clearable
                @change="handleQuery"
              >
                <el-option
                  v-for="wh in warehouses"
                  :key="wh.id"
                  :label="wh.warehouse_name || wh.name"
                  :value="wh.id"
                />
              </el-select>
            </el-form-item>
            <el-form-item label="状态">
              <el-select
                v-model="queryParams.status"
                placeholder="选择状态"
                clearable
                @change="handleQuery"
              >
                <el-option label="正常" value="normal" />
                <el-option label="预警" value="warning" />
                <el-option label="冻结" value="frozen" />
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
          <el-table v-loading="loading" :data="stocks" stripe>
            <el-table-column prop="product_code" label="产品编码" width="140" fixed />
            <el-table-column prop="product_name" label="产品名称" min-width="180" fixed />
            <el-table-column prop="warehouse_name" label="仓库" width="120" />
            <el-table-column prop="batch_no" label="批次号" width="120" />
            <el-table-column prop="color_code" label="颜色编码" width="100" />
            <el-table-column prop="location" label="库位" width="100" />
            <el-table-column prop="quantity" label="库存数量" width="100" align="right">
              <template #default="{ row }">
                <span :class="{ 'low-stock': row.quantity < row.min_quantity }">
                  {{ row.quantity }}
                </span>
              </template>
            </el-table-column>
            <el-table-column prop="unit" label="单位" width="60" />
            <el-table-column prop="gram_weight" label="克重" width="80" />
            <el-table-column prop="width" label="门幅" width="80" />
            <el-table-column prop="status" label="状态" width="80">
              <template #default="{ row }">
                <el-tag :type="getStatusType(row.status)" size="small">
                  {{ getStatusText(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="150" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="handleView(row)"
                  >详情</el-button
                >
                <el-button type="warning" link size="small" @click="handleAdjust(row)"
                  >调整</el-button
                >
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
      </el-tab-pane>

      <el-tab-pane label="库存预警" name="alert">
        <el-card shadow="hover">
          <el-table :data="alerts" stripe>
            <el-table-column prop="product_code" label="产品编码" width="140" />
            <el-table-column prop="product_name" label="产品名称" min-width="180" />
            <el-table-column prop="warehouse_name" label="仓库" width="120" />
            <el-table-column prop="current_quantity" label="当前库存" width="100" align="right">
              <template #default="{ row }">
                <span class="low-stock">{{ row.current_quantity }}</span>
              </template>
            </el-table-column>
            <el-table-column prop="min_quantity" label="最小库存" width="100" align="right" />
            <el-table-column prop="unit" label="单位" width="60" />
            <el-table-column prop="alert_level" label="预警级别" width="100">
              <template #default="{ row }">
                <el-tag :type="row.alert_level === 'danger' ? 'danger' : 'warning'" size="small">
                  {{ row.alert_level === 'danger' ? '紧急' : '警告' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="100">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="handlePurchase(row)"
                  >采购</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="库存调拨" name="transfer">
        <el-card shadow="hover">
          <div class="transfer-actions">
            <el-button type="primary" @click="handleNewTransfer">
              <el-icon><Plus /></el-icon>
              新建调拨单
            </el-button>
          </div>
          <el-table :data="transfers" stripe>
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
          <el-input v-model="item.product_name" placeholder="产品名称" style="flex: 2" />
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
  Search,
  Refresh,
  Printer,
  OfficeBuilding,
  WarningFilled,
  Plus,
  Delete,
} from '@element-plus/icons-vue'
import printJS from 'print-js'

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

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  warehouse_id: undefined as number | undefined,
  status: '',
})

const formatNumber = (num: number) => {
  return num.toLocaleString()
}

const getStatusType = (status: string) => {
  const typeMap: Record<string, any> = {
    normal: 'success',
    warning: 'warning',
    frozen: 'info',
  }
  return typeMap[status] || 'info'
}

const getStatusText = (status: string) => {
  const textMap: Record<string, string> = {
    normal: '正常',
    warning: '预警',
    frozen: '冻结',
  }
  return textMap[status] || status
}

const getTransferStatusType = (status: string) => {
  const typeMap: Record<string, any> = {
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
  items: [{ product_id: null as number | null, product_name: '', quantity: 0 }],
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

const handleAdjust = (row: any) => {
  adjustmentForm.value = {
    stock_id: row.id,
    product_id: row.product_id,
    warehouse_id: row.warehouse_id,
    product_name: row.product_name,
    warehouse_name: row.warehouse_name,
    current_quantity: row.quantity,
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
  activeTab.value = 'transfer'
}

const handlePrint = () => {
  const printData = stocks.value.map((item: any, index: number) => ({
    序号: index + 1,
    产品编码: item.product_code,
    产品名称: item.product_name,
    规格: item.specification,
    单位: item.unit,
    库存数量: item.quantity,
    仓库: item.warehouse_name,
    库存金额: `¥${item.stock_value}`,
  }))
  printJS({
    printable: printData,
    properties: Object.keys(printData[0] || {}),
    type: 'table' as any,
    header: '库存台账列表',
    style: 'padding: 20px; font-size: 14px;',
    headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
    gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
    gridStyle: 'border-collapse: collapse; width: 100%;',
  })
}

const handleExport = () => {
  const csvContent = [
    ['产品编码', '产品名称', '规格', '单位', '库存数量', '仓库', '库存金额'],
    ...stocks.value.map((item: any) => [
      item.product_code,
      item.product_name,
      item.specification,
      item.unit,
      item.quantity,
      item.warehouse_name,
      item.stock_value,
    ]),
  ]
    .map((row: any[]) => row.map((cell: any) => `"${cell}"`).join(','))
    .join('\n')
  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `库存台账_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
}

const handleView = (row: any) => {
  ElMessage.info(`查看 ${row.product_name} 详情`)
}

const handlePurchase = (row: any) => {
  ElMessage.info(`为 ${row.product_name} 创建采购单`)
}

const handleNewTransfer = () => {
  transferForm.value = {
    from_warehouse_id: null,
    to_warehouse_id: null,
    items: [{ product_id: null, product_name: '', quantity: 0 }],
    remark: '',
  }
  transferDialogVisible.value = true
}

const handleAddTransferItem = () => {
  transferForm.value.items.push({ product_id: null, product_name: '', quantity: 0 })
}

const handleRemoveTransferItem = (index: number) => {
  if (transferForm.value.items.length > 1) {
    transferForm.value.items.splice(index, 1)
  }
}

const handleSubmitTransfer = async () => {
  if (!transferForm.value.from_warehouse_id) {
    ElMessage.warning('请选择调出仓库')
    return
  }
  if (!transferForm.value.to_warehouse_id) {
    ElMessage.warning('请选择调入仓库')
    return
  }
  if (transferForm.value.from_warehouse_id === transferForm.value.to_warehouse_id) {
    ElMessage.warning('调出仓库和调入仓库不能相同')
    return
  }

  const validItems = transferForm.value.items
    .filter((item) => item.product_id && item.quantity > 0)
    .map((item) => ({
      product_id: item.product_id!,
      quantity: item.quantity,
    }))
  if (validItems.length === 0) {
    ElMessage.warning('请添加至少一个调拨产品')
    return
  }

  try {
    const { inventoryApi } = await import('@/api/inventory')
    await inventoryApi.createTransfer({
      from_warehouse_id: transferForm.value.from_warehouse_id,
      to_warehouse_id: transferForm.value.to_warehouse_id,
      items: validItems,
      remark: transferForm.value.remark,
    })
    ElMessage.success('调拨单创建成功')
    transferDialogVisible.value = false
    fetchTransfers()
  } catch (error: any) {
    ElMessage.error(error.message || '创建调拨单失败')
  }
}

const handleViewTransfer = (row: any) => {
  ElMessage.info(`查看调拨单 ${row.transfer_no}`)
}

const handleApproveTransfer = (row: any) => {
  ElMessage.success(`审批通过调拨单 ${row.transfer_no}`)
}

onMounted(() => {
  fetchData()
  fetchWarehouses()
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

.stat-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}

.stat-card.warning {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}

.stat-card.warning .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.warning .stat-label,
.stat-card.warning .stat-value {
  color: white;
}

.stat-card.danger {
  background: linear-gradient(135deg, #ff9a9e 0%, #fecfef 100%);
}

.stat-card.danger .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.danger .stat-label,
.stat-card.danger .stat-value {
  color: white;
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

.stat-icon.total-icon {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}

.stat-icon.alert-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-icon.warehouse-icon {
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
}

.stat-icon.low-icon {
  background: rgba(255, 255, 255, 0.2);
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

.filter-card {
  margin-bottom: 20px;
}

.table-card {
  margin-bottom: 20px;
}

.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}

.low-stock {
  color: #f56c6c;
  font-weight: 600;
}

.transfer-actions {
  margin-bottom: 16px;
}

:deep(.el-card__header) {
  padding: 16px 20px;
  border-bottom: 1px solid #ebeef5;
}

:deep(.el-card__body) {
  padding: 20px;
}
</style>

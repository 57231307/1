<!--
  inventory/index.vue - 库存管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-4）：
  原 915 行"上帝组件"已拆分为以下结构：

  - tabs/StockTab.vue          （库存台账）
  - tabs/AlertTab.vue          （库存预警）
  - tabs/TransferTab.vue       （库存调拨）
  - tabs/AdjustmentDialogTab.vue（调整弹窗）
  - tabs/TransferDialogTab.vue （调拨弹窗）

  本主入口仅承担：统计卡片 + Tab 切换 < 200 行。
-->
<template>
  <div class="inventory-page">
    <div class="page-header">
      <h1 class="page-title">库存管理</h1>
      <div class="header-actions">
        <el-button type="primary" @click="handleAdjustment">
          <el-icon><Edit /></el-icon>库存调整
        </el-button>
        <el-button @click="handleTransfer">
          <el-icon><RefreshRight /></el-icon>库存调拨
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

    <el-tabs v-model="activeTab">
      <el-tab-pane label="库存台账" name="stock"><StockTab ref="stockRef" /></el-tab-pane>
      <el-tab-pane label="库存预警" name="alert"><AlertTab /></el-tab-pane>
      <el-tab-pane label="库存调拨" name="transfer">
        <TransferTab @new-transfer="openTransferDialog" />
      </el-tab-pane>
    </el-tabs>

    <AdjustmentDialogTab
      v-model="adjustmentDialogVisible"
      :current-row="currentAdjustRow"
      @submitted="handleAdjustmentSubmitted"
    />
    <TransferDialogTab
      v-model="transferDialogVisible"
      :warehouses="warehouses"
      @submitted="handleTransferSubmitted"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import {
  Box,
  Warning,
  Edit,
  RefreshRight,
  OfficeBuilding,
  WarningFilled,
} from '@element-plus/icons-vue'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
import StockTab from './tabs/StockTab.vue'
import AlertTab from './tabs/AlertTab.vue'
import TransferTab from './tabs/TransferTab.vue'
import AdjustmentDialogTab from './tabs/AdjustmentDialogTab.vue'
import TransferDialogTab from './tabs/TransferDialogTab.vue'

interface Warehouse {
  id: number
  warehouse_name?: string
  name?: string
}
interface StockRow {
  id: number
  product_id: number
  warehouse_id: number
  product_name: string
  warehouse_name: string
  quantity: number
}

const activeTab = ref('stock')
const stockRef = ref()
const warehouses = ref<Warehouse[]>([])
const adjustmentDialogVisible = ref(false)
const transferDialogVisible = ref(false)
const currentAdjustRow = ref<StockRow | null>(null)

const stats = reactive({
  totalQuantity: 0,
  alertCount: 0,
  warehouseCount: 0,
  lowStockCount: 0,
})

const formatNumber = (num: number) => num.toLocaleString()

const fetchStats = async () => {
  try {
    const { inventoryApi } = await import('@/api/inventory')
    const res = await inventoryApi.getInventoryReport({})
    const s = (res.data?.summary || {}) as Record<string, number>
    stats.totalQuantity = s.total_quantity || 0
    stats.alertCount = s.alert_count || 0
    stats.warehouseCount = s.warehouse_count || 0
    stats.lowStockCount = s.low_stock_count || 0
  } catch (error) {
    logger.error('获取库存汇总失败', (error as Error).message)
  }
}

const fetchWarehouses = async () => {
  try {
    const { warehouseApi } = await import('@/api/warehouse')
    const res = await warehouseApi.list({ page: 1, page_size: 1000 })
    warehouses.value = (res.data?.list as Warehouse[] | undefined) || []
  } catch (error) {
    logger.error('获取仓库列表失败', (error as Error).message)
  }
}

const handleAdjustment = () => {
  currentAdjustRow.value = null
  adjustmentDialogVisible.value = true
}
const handleTransfer = () => {
  activeTab.value = 'transfer'
}
const openTransferDialog = () => {
  transferDialogVisible.value = true
}
const handleAdjustmentSubmitted = () => {
  fetchStats()
  stockRef.value?.fetchData()
}
const handleTransferSubmitted = () => {
  fetchStats()
}

const hasLoaded = createLazyLoader()
onMounted(() => {
  fetchStats()
  loadIfNot('warehouses', fetchWarehouses, hasLoaded)
})
</script>

<style scoped>
.inventory-page {
  padding: 24px;
  background: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}
.page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0;
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
  transition: all 0.3s;
}
.stat-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}
.stat-card.warning,
.stat-card.danger :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}
.stat-card.warning,
.stat-card.danger :deep(.stat-label),
.stat-card.warning,
.stat-card.danger :deep(.stat-value) {
  color: white;
}
.stat-card.warning {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}
.stat-card.danger {
  background: linear-gradient(135deg, #ff9a9e 0%, #fecfef 100%);
}
:deep(.stat-content) {
  display: flex;
  align-items: center;
  gap: 16px;
}
:deep(.stat-icon) {
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
:deep(.stat-icon.total-icon) {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}
:deep(.stat-icon.warehouse-icon) {
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
}
:deep(.stat-icon.alert-icon),
:deep(.stat-icon.low-icon) {
  background: rgba(255, 255, 255, 0.2);
}
:deep(.stat-info) {
  flex: 1;
}
:deep(.stat-label) {
  font-size: 14px;
  color: #909399;
  margin-bottom: 4px;
}
:deep(.stat-value) {
  font-size: 28px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}
</style>

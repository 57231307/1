<template>
  <div class="trading-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="采购合同" name="purchase-contract">
        <div class="page-header">
          <h2 class="page-title">采购合同管理</h2>
          <el-button type="primary" @click="openPurchaseContractDialog">
            <el-icon><Plus /></el-icon>
            新建合同
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table :data="purchaseContracts" v-loading="purchaseContractLoading" stripe>
            <el-table-column prop="contract_no" label="合同编号" width="140" />
            <el-table-column prop="supplier_name" label="供应商" width="150" />
            <el-table-column prop="contract_date" label="合同日期" width="120" />
            <el-table-column prop="total_amount" label="总金额" width="120" align="right">
              <template #default="{ row }">{{ formatMoney(row.total_amount) }}</template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="getStatusType(row.status)" size="small">{{ getContractStatusLabel(row.status) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="200" fixed="right">
              <template #default>
                <el-button type="primary" link size="small" @click="viewPurchaseContract">查看</el-button>
                <el-button type="success" link size="small" @click="approvePurchaseContract">审批</el-button>
                <el-button type="warning" link size="small" @click="executePurchaseContract">执行</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="采购价格" name="purchase-price">
        <div class="page-header">
          <h2 class="page-title">采购价格管理</h2>
          <el-button type="primary" @click="openPurchasePriceDialog">
            <el-icon><Plus /></el-icon>
            新建价格
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table :data="purchasePrices" v-loading="purchasePriceLoading" stripe>
            <el-table-column prop="product_name" label="产品" width="150" />
            <el-table-column prop="supplier_name" label="供应商" width="150" />
            <el-table-column prop="price" label="价格" width="100" align="right">
              <template #default="{ row }">{{ formatMoney(row.price) }}</template>
            </el-table-column>
            <el-table-column prop="currency" label="币种" width="80" />
            <el-table-column prop="unit" label="单位" width="80" />
            <el-table-column prop="effective_date" label="生效日期" width="120" />
            <el-table-column prop="expiry_date" label="失效日期" width="120" />
            <el-table-column prop="status" label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">{{ row.status === 'active' ? '有效' : '无效' }}</el-tag>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="销售合同" name="sales-contract">
        <div class="page-header">
          <h2 class="page-title">销售合同管理</h2>
          <el-button type="primary" @click="openSalesContractDialog">
            <el-icon><Plus /></el-icon>
            新建合同
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table :data="salesContracts" v-loading="salesContractLoading" stripe>
            <el-table-column prop="contract_no" label="合同编号" width="140" />
            <el-table-column prop="customer_name" label="客户" width="150" />
            <el-table-column prop="contract_date" label="合同日期" width="120" />
            <el-table-column prop="total_amount" label="总金额" width="120" align="right">
              <template #default="{ row }">{{ formatMoney(row.total_amount) }}</template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="getStatusType(row.status)" size="small">{{ getContractStatusLabel(row.status) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="200" fixed="right">
              <template #default>
                <el-button type="primary" link size="small" @click="viewSalesContract">查看</el-button>
                <el-button type="success" link size="small" @click="approveSalesContract">审批</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="销售价格" name="sales-price">
        <div class="page-header">
          <h2 class="page-title">销售价格管理</h2>
          <el-button type="primary" @click="openSalesPriceDialog">
            <el-icon><Plus /></el-icon>
            新建价格
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table :data="salesPrices" v-loading="salesPriceLoading" stripe>
            <el-table-column prop="product_name" label="产品" width="150" />
            <el-table-column prop="customer_name" label="客户" width="150" />
            <el-table-column prop="price" label="价格" width="100" align="right">
              <template #default="{ row }">{{ formatMoney(row.price) }}</template>
            </el-table-column>
            <el-table-column prop="currency" label="币种" width="80" />
            <el-table-column prop="unit" label="单位" width="80" />
            <el-table-column prop="effective_date" label="生效日期" width="120" />
            <el-table-column prop="status" label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">{{ row.status === 'active' ? '有效' : '无效' }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="120">
              <template #default>
                <el-button type="success" link size="small" @click="approveSalesPrice">审批</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="销售退货" name="sales-return">
        <div class="page-header">
          <h2 class="page-title">销售退货管理</h2>
          <el-button type="primary" @click="openReturnDialog">
            <el-icon><Plus /></el-icon>
            新建退货
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table :data="salesReturns" v-loading="returnLoading" stripe>
            <el-table-column prop="return_no" label="退货单号" width="140" />
            <el-table-column prop="customer_name" label="客户" width="150" />
            <el-table-column prop="order_no" label="销售单号" width="140" />
            <el-table-column prop="return_date" label="退货日期" width="120" />
            <el-table-column prop="total_amount" label="退货金额" width="120" align="right">
              <template #default="{ row }">{{ formatMoney(row.total_amount) }}</template>
            </el-table-column>
            <el-table-column prop="reason" label="退货原因" min-width="150" />
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="getStatusType(row.status)" size="small">{{ getReturnStatusLabel(row.status) }}</el-tag>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { listPurchaseContracts, type PurchaseContract, listPurchasePrices, type PurchasePrice, listSalesContracts, type SalesContract, listSalesPrices, type SalesPrice, listSalesReturns, type SalesReturn } from '@/api/trading'

const activeTab = ref('purchase-contract')
const purchaseContracts = ref<PurchaseContract[]>([])
const purchasePrices = ref<PurchasePrice[]>([])
const salesContracts = ref<SalesContract[]>([])
const salesPrices = ref<SalesPrice[]>([])
const salesReturns = ref<SalesReturn[]>([])
const purchaseContractLoading = ref(false)
const purchasePriceLoading = ref(false)
const salesContractLoading = ref(false)
const salesPriceLoading = ref(false)
const returnLoading = ref(false)

const fetchPurchaseContracts = async () => {
  purchaseContractLoading.value = true
  try {
    const res = await listPurchaseContracts()
    purchaseContracts.value = res.data || []
  } finally {
    purchaseContractLoading.value = false
  }
}

const fetchPurchasePrices = async () => {
  purchasePriceLoading.value = true
  try {
    const res = await listPurchasePrices()
    purchasePrices.value = res.data || []
  } finally {
    purchasePriceLoading.value = false
  }
}

const fetchSalesContracts = async () => {
  salesContractLoading.value = true
  try {
    const res = await listSalesContracts()
    salesContracts.value = res.data || []
  } finally {
    salesContractLoading.value = false
  }
}

const fetchSalesPrices = async () => {
  salesPriceLoading.value = true
  try {
    const res = await listSalesPrices()
    salesPrices.value = res.data || []
  } finally {
    salesPriceLoading.value = false
  }
}

const fetchSalesReturns = async () => {
  returnLoading.value = true
  try {
    const res = await listSalesReturns()
    salesReturns.value = res.data || []
  } finally {
    returnLoading.value = false
  }
}

const formatMoney = (amount: number) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getStatusType = (status: string) => {
  const map: Record<string, any> = { draft: 'info', pending: 'warning', approved: 'success', active: 'primary', completed: 'success', cancelled: 'danger' }
  return map[status] || 'info'
}

const getContractStatusLabel = (status: string) => {
  const map: Record<string, string> = { draft: '草稿', pending: '待审批', approved: '已审批', active: '执行中', completed: '已完成', cancelled: '已取消' }
  return map[status] || status
}

const getReturnStatusLabel = (status: string) => {
  const map: Record<string, string> = { draft: '草稿', pending: '待审批', approved: '已审批', rejected: '已拒绝', completed: '已完成' }
  return map[status] || status
}

const openPurchaseContractDialog = () => ElMessage.info('功能开发中')
const viewPurchaseContract = () => ElMessage.info('功能开发中')
const approvePurchaseContract = async (_row: PurchaseContract) => {
  try {
    await ElMessageBox.confirm('确定审批此采购合同吗？', '确认', { type: 'info' })
    ElMessage.success('审批成功')
    fetchPurchaseContracts()
  } catch (e) { if (e !== 'cancel') console.error(e) }
}
const executePurchaseContract = async (_row: PurchaseContract) => {
  try {
    await ElMessageBox.confirm('确定执行此采购合同吗？', '确认', { type: 'info' })
    ElMessage.success('执行成功')
    fetchPurchaseContracts()
  } catch (e) { if (e !== 'cancel') console.error(e) }
}
const openPurchasePriceDialog = () => ElMessage.info('功能开发中')
const openSalesContractDialog = () => ElMessage.info('功能开发中')
const viewSalesContract = () => ElMessage.info('功能开发中')
const approveSalesContract = async (_row: SalesContract) => {
  try {
    await ElMessageBox.confirm('确定审批此销售合同吗？', '确认', { type: 'info' })
    ElMessage.success('审批成功')
    fetchSalesContracts()
  } catch (e) { if (e !== 'cancel') console.error(e) }
}
const openSalesPriceDialog = () => ElMessage.info('功能开发中')
const approveSalesPrice = async (_row: SalesPrice) => {
  try {
    await ElMessageBox.confirm('确定审批此销售价格吗？', '确认', { type: 'info' })
    ElMessage.success('审批成功')
    fetchSalesPrices()
  } catch (e) { if (e !== 'cancel') console.error(e) }
}
const openReturnDialog = () => ElMessage.info('功能开发中')

onMounted(() => {
  fetchPurchaseContracts()
  fetchPurchasePrices()
  fetchSalesContracts()
  fetchSalesPrices()
  fetchSalesReturns()
})
</script>

<style scoped>
.trading-page { padding: 24px; background-color: #f5f7fa; min-height: 100%; }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
.page-title { font-size: 20px; font-weight: 600; color: #303133; margin: 0; }
</style>

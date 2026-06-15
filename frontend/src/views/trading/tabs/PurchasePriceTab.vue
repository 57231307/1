<!--
  PurchasePriceTab.vue - 采购价格 Tab
  来源：原 trading/index.vue 中 采购价格 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="purchase-price-tab">
    <div class="page-header">
      <h2 class="page-title">采购价格管理</h2>
      <el-button type="primary" @click="openPurchasePriceDialog()">
        <el-icon><Plus /></el-icon> 新建价格
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table v-loading="purchasePriceLoading" :data="purchasePrices" stripe>
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
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? '有效' : '无效' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="120">
          <template #default="{ row }">
            <el-button
              type="primary"
              link
              size="small"
              @click="openPurchasePriceDialog(row as unknown as TradingPrice)"
              >编辑</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  listTradingPrices,
  getTradingPrice,
  createTradingPrice,
  type TradingPrice,
} from '@/api/trading-price'

const purchasePrices = ref<TradingPrice[]>([])
const purchasePriceLoading = ref(false)

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const fetchPurchasePrices = async () => {
  purchasePriceLoading.value = true
  try {
    const res = await listTradingPrices({ type: 'purchase' })
    const d = res.data as
      | { list?: TradingPrice[]; items?: TradingPrice[] }
      | TradingPrice[]
      | undefined
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      purchasePrices.value = d.list || d.items || []
    } else {
      purchasePrices.value = (d as TradingPrice[]) || []
    }
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '获取采购价格失败')
  } finally {
    purchasePriceLoading.value = false
  }
}

const openPurchasePriceDialog = async (row?: TradingPrice) => {
  try {
    if (row) {
      const res = await getTradingPrice(row.id)
      ElMessage.info(`编辑价格: ${res.data?.product_name || row.product_name}`)
    } else {
      await createTradingPrice({ type: 'purchase', status: 'active' })
      ElMessage.success('已创建')
      fetchPurchasePrices()
    }
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '操作失败')
  }
}

defineExpose({ refresh: fetchPurchasePrices })

onMounted(() => {
  fetchPurchasePrices()
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

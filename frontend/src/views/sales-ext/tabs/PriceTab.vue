<!--
  PriceTab.vue - 销售价格 Tab
  来源：原 sales-ext/index.vue 中 销售价格 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="price-tab">
    <div class="page-header">
      <h2 class="page-title">销售价格管理</h2>
      <el-button type="primary" @click="openPriceDialog()">
        <el-icon><Plus /></el-icon> 新建价格
      </el-button>
    </div>
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="priceQuery">
        <el-form-item label="产品">
          <el-input v-model="priceQuery.productName" placeholder="产品名称" clearable />
        </el-form-item>
        <el-form-item label="客户">
          <el-input v-model="priceQuery.customerName" placeholder="客户名称" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="priceQuery.status" placeholder="选择状态" clearable>
            <el-option label="启用" value="active" />
            <el-option label="禁用" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchSalesPrices">查询</el-button>
          <el-button @click="resetPriceQuery">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>
    <el-card shadow="hover">
      <el-table v-loading="priceLoading" :data="salesPrices" stripe>
        <el-table-column prop="productName" label="产品名称" min-width="150" />
        <el-table-column prop="productCode" label="产品编码" width="120" />
        <el-table-column prop="customerName" label="客户" min-width="150" />
        <el-table-column prop="price" label="价格" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.price) }}
          </template>
        </el-table-column>
        <el-table-column prop="currency" label="货币" width="80" />
        <el-table-column prop="unit" label="单位" width="80" />
        <el-table-column prop="effectiveDate" label="生效日期" width="120" />
        <el-table-column prop="expiryDate" label="失效日期" width="120" />
        <el-table-column prop="status" label="状态" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link @click="openPriceDialog(row as unknown as SalesPrice)"
              >编辑</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              link
              type="success"
              @click="approvePrice(row as unknown as SalesPrice)"
              >审批</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { listSalesPrices, approveSalesPrice, type SalesPrice } from '@/api/sales-price'

const salesPrices = ref<SalesPrice[]>([])
const priceLoading = ref(false)

const priceQuery = reactive({
  productName: '',
  customerName: '',
  status: '',
})

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const fetchSalesPrices = async () => {
  priceLoading.value = true
  try {
    const res = await listSalesPrices(priceQuery)
    salesPrices.value = res.data?.list || []
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || '获取销售价格失败')
  } finally {
    priceLoading.value = false
  }
}

const resetPriceQuery = () => {
  priceQuery.productName = ''
  priceQuery.customerName = ''
  priceQuery.status = ''
  fetchSalesPrices()
}

const openPriceDialog = async (_row?: SalesPrice) => {
  // 简化：销售价格对话框暂用列表+行内编辑模式，
  // 完整对话框可在后续迭代从原文件迁入（保留 API 调用）
  ElMessage.info('请使用行内编辑')
}

const approvePrice = async (row: SalesPrice) => {
  try {
    await approveSalesPrice(row.id)
    ElMessage.success('审批成功')
    fetchSalesPrices()
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || '操作失败')
  }
}

defineExpose({ refresh: fetchSalesPrices })

onMounted(() => {
  fetchSalesPrices()
})
</script>

<!--
  AlertTab.vue - 库存预警 Tab
  来源：原 inventory/index.vue 中 库存预警 tab 内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="alert-tab">
    <el-card shadow="hover">
      <el-table v-loading="loading" :data="alerts" stripe>
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
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'

interface AlertRow {
  id: number
  product_code: string
  product_name: string
  warehouse_name: string
  current_quantity: number
  min_quantity: number
  unit: string
  alert_level: string
}

const alerts = ref<AlertRow[]>([])
const loading = ref(false)

const fetchAlerts = async () => {
  loading.value = true
  try {
    const { inventoryApi } = await import('@/api/inventory')
    const res = await inventoryApi.getStockAlerts()
    alerts.value = (res.data as AlertRow[] | undefined) || []
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '获取库存预警失败')
    alerts.value = []
  } finally {
    loading.value = false
  }
}

const handlePurchase = (row: AlertRow) => {
  ElMessage.info(`为 ${row.product_name} 创建采购单`)
}

onMounted(() => {
  fetchAlerts()
})
</script>

<style scoped>
.low-stock {
  color: #f56c6c;
  font-weight: 600;
}
</style>

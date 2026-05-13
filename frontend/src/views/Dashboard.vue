<template>
  <div class="dashboard">
    <el-row :gutter="20">
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>面料总数</template>
          <div class="stat-value">{{ stats.fabricCount }}</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>库存总量</template>
          <div class="stat-value">{{ stats.inventoryTotal }}</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>本月订单</template>
          <div class="stat-value">{{ stats.monthOrders }}</div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <template #header>客户总数</template>
          <div class="stat-value">{{ stats.customerCount }}</div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'

const stats = ref({
  fabricCount: 0,
  inventoryTotal: 0,
  monthOrders: 0,
  customerCount: 0,
})

onMounted(() => {
  stats.value = {
    fabricCount: 156,
    inventoryTotal: 12500,
    monthOrders: 89,
    customerCount: 45,
  }
})
</script>

<style scoped>
.dashboard {
  padding: 20px;
}
.stat-value {
  font-size: 32px;
  font-weight: bold;
  color: #409eff;
  text-align: center;
}
</style>

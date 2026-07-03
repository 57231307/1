<!--
  sales-analysis/index.vue - 销售分析（拆分重构版）
  任务编号: P14 批 2 I-3 第 6 批
  拆分：535 行 → ~110 行 + 5 子组件 + 3 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="sales-analysis-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">销售分析</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>销售管理</el-breadcrumb-item>
          <el-breadcrumb-item>销售分析</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button @click="saProc.handleExport">
          <el-icon><Download /></el-icon>
          导出报表
        </el-button>
      </div>
    </div>

    <SaStat :stats="sa.stats" />

    <SaTrend
      :period="sa.trendPeriod"
      :data="sa.trendData"
      :composition="sa.productRanking"
      @update:period="(v: string) => (sa.trendPeriod = v)"
    />

    <el-row :gutter="20" class="ranking-row">
      <el-col :xs="24" :lg="12">
        <SaProdRank
          :data="sa.productRanking"
          :type="sa.productRankType"
          @update:type="(v: string) => saProc.handleProductRankTypeChange(v, sa)"
        />
      </el-col>
      <el-col :xs="24" :lg="12">
        <SaCustRank
          :data="sa.customerRanking"
          :type="sa.customerRankType"
          @update:type="(v: string) => saProc.handleCustomerRankTypeChange(v, sa)"
        />
      </el-col>
    </el-row>

    <SaTarget :data="sa.salesTargets" @edit-target="saProc.handleEditTarget" />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { Download } from '@element-plus/icons-vue'
import { useSa } from './composables/useSa'
import { useSaProc } from './composables/useSaProc'
import SaStat from './components/SaStat.vue'
import SaTrend from './components/SaTrend.vue'
import SaProdRank from './components/SaProdRank.vue'
import SaCustRank from './components/SaCustRank.vue'
import SaTarget from './components/SaTarget.vue'

// 业务状态
const sa = useSa()
const saProc = useSaProc()

onMounted(() => {
  sa.getStats()
  sa.getProductRanking()
  sa.getCustomerRanking()
  sa.getSalesTargets()
  sa.getTrendData()
})
</script>

<style scoped>
.sales-analysis-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.page-title {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.ranking-row {
  margin-bottom: 20px;
}
</style>

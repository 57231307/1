<template>
  <div class="advanced-page">
    <el-tabs v-model="activeTab" @tab-change="(tab: string | number) => loadTab(tab)">
      <el-tab-pane label="AI 分析" name="ai">
        <AiPanel
          :forecast-period="ai.forecastPeriod.value"
          :forecast-loading="ai.forecastLoading.value"
          :forecast-result="ai.forecastResult.value"
          :run-sales-forecast="ai.runSalesForecast"
          :inventory-loading="ai.inventoryLoading.value"
          :inventory-result="ai.inventoryResult.value"
          :run-inventory-optimization="ai.runInventoryOptimization"
          :anomaly-type="ai.anomalyType.value"
          :anomaly-loading="ai.anomalyLoading.value"
          :anomaly-result="ai.anomalyResult.value"
          :run-anomaly-detection="ai.runAnomalyDetection"
          :recommend-loading="ai.recommendLoading.value"
          :recommendation-result="ai.recommendationResult.value"
          :get-recommendations="ai.getRecommendations"
          :format-money="ai.formatMoney"
          @update:forecast-period="(v: string) => (ai.forecastPeriod.value = v)"
          @update:anomaly-type="(v: string) => (ai.anomalyType.value = v)"
        />
      </el-tab-pane>

      <el-tab-pane label="报表引擎" name="report">
        <RptPanel
          :report-templates="rpt.reportTemplates.value"
          :report-loading="rpt.reportLoading.value"
          :report-result-visible="rpt.reportResultVisible.value"
          :report-data="rpt.reportData.value"
          :report-columns="rpt.reportColumns.value"
          :execute-report="rpt.executeReport"
          :export-report="rpt.exportReport"
          @update:report-result-visible="(v: boolean) => (rpt.reportResultVisible.value = v)"
        />
      </el-tab-pane>

      <el-tab-pane label="工艺优化" name="recipe">
        <RcpPanel
          :recipe-form="rcp.recipeForm.value"
          :recipe-loading="rcp.recipeLoading.value"
          :recipe-result="rcp.recipeResult.value"
          :run-recipe-optimization="rcp.runRecipeOptimization"
          @update:recipe-form="(v) => { rcp.recipeForm.value = v }"
        />
      </el-tab-pane>

      <el-tab-pane label="质量预测" name="quality">
        <QltPanel
          :quality-form="qlt.qualityForm.value"
          :quality-loading="qlt.qualityLoading.value"
          :quality-result="qlt.qualityResult.value"
          :run-quality-prediction="qlt.runQualityPrediction"
          @update:quality-form="(v) => { qlt.qualityForm.value = v }"
        />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { useAi } from './composables/useAi'
import { useRpt } from './composables/useRpt'
import { useRcp } from './composables/useRcp'
import { useQlt } from './composables/useQlt'
import AiPanel from './components/AiPanel.vue'
import RptPanel from './components/RptPanel.vue'
import RcpPanel from './components/RcpPanel.vue'
import QltPanel from './components/QltPanel.vue'

const activeTab = ref('ai')
const hasLoaded = createLazyLoader()

// 在父组件集中创建所有 composable 实例
// 子组件通过 props 接收需要的数据与函数（保证行为完全一致）
const ai = useAi()
const rpt = useRpt()
const rcp = useRcp()
const qlt = useQlt()

// 各 tab 懒加载映射
const tabLoaders: Record<string, () => void> = {
  ai: () => {},
  report: rpt.fetchReportTemplates,
  recipe: () => {},
  quality: () => {},
}

const loadTab = (tabName: string | number) => {
  const key = String(tabName)
  if (tabLoaders[key]) {
    loadIfNot(key, tabLoaders[key], hasLoaded)
  }
}

const initPage = () => {
  loadTab(activeTab.value)
}

onMounted(() => {
  initPage()
})
</script>

<style scoped>
.advanced-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
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
.card-header {
  font-weight: 600;
}
.mb-20 {
  margin-bottom: 20px;
}
.mb-10 {
  margin-bottom: 10px;
}
.mb-12 {
  margin-bottom: 12px;
}
.report-result {
  max-height: 60vh;
  overflow: auto;
}
.metric-label {
  font-size: 13px;
  color: #909399;
  margin-bottom: 6px;
}
.metric-sub {
  font-size: 12px;
  color: #909399;
  margin-top: 4px;
}
.form-hint {
  display: block;
  font-size: 12px;
  color: #909399;
  margin-top: 4px;
}
.rec-list {
  list-style: none;
  padding-left: 0;
  margin: 0;
}
.rec-list li {
  margin-bottom: 8px;
}
</style>

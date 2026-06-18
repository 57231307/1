<script setup lang="ts">
/**
 * advanced/index.vue - 高级功能总览页（AI 分析 / 报表引擎 / 多租户管理 / 工艺优化 / 质量预测）
 * 任务编号: P13 批 1 B3 I-1（拆分原 993 行大 .vue）
 * 拆分后：5 个 tab 子组件 + 1 个租户对话框 + 5 个 composable
 * 行为完全保持一致（仅结构重构）
 */
import { ref, onMounted } from 'vue'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { useAi } from './composables/useAi'
import { useRpt } from './composables/useRpt'
import { useTnt } from './composables/useTnt'
import { useRcp } from './composables/useRcp'
import { useQlt } from './composables/useQlt'
import AiPanel from './components/AiPanel.vue'
import RptPanel from './components/RptPanel.vue'
import TntPanel from './components/TntPanel.vue'
import RcpPanel from './components/RcpPanel.vue'
import QltPanel from './components/QltPanel.vue'
import TntForm from './components/TntForm.vue'

const activeTab = ref('ai')
const hasLoaded = createLazyLoader()

// 在父组件集中创建所有 composable 实例
// 子组件通过 props 接收需要的数据与函数（保证行为完全一致）
const ai = useAi()
const rpt = useRpt()
const tnt = useTnt()
const rcp = useRcp()
const qlt = useQlt()

// 各 tab 懒加载映射
const tabLoaders: Record<string, () => void> = {
  ai: () => {},
  report: rpt.fetchReportTemplates,
  tenant: tnt.fetchTenants,
  recipe: () => {},
  quality: () => {},
}

const loadTab = (tabName: string) => {
  if (tabLoaders[tabName]) {
    loadIfNot(tabName, tabLoaders[tabName], hasLoaded)
  }
}

const initPage = () => {
  loadTab(activeTab.value as string)
}

onMounted(() => {
  initPage()
})
</script>

<template>
  <div class="advanced-page">
    <el-tabs v-model="activeTab" @tab-change="(tab: any) => loadTab(tab)">
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

      <el-tab-pane label="多租户管理" name="tenant">
        <TntPanel
          :tenants="tnt.tenants.value"
          :tenant-loading="tnt.tenantLoading.value"
          :open-tenant-dialog="tnt.openTenantDialog"
          :update-tenant-status="tnt.updateTenantStatus"
          :delete-tenant="tnt.deleteTenant"
        />
      </el-tab-pane>

      <el-tab-pane label="工艺优化" name="recipe">
        <RcpPanel
          :recipe-form="rcp.recipeForm.value"
          :recipe-loading="rcp.recipeLoading.value"
          :recipe-result="rcp.recipeResult.value"
          :run-recipe-optimization="rcp.runRecipeOptimization"
        />
      </el-tab-pane>

      <el-tab-pane label="质量预测" name="quality">
        <QltPanel
          :quality-form="qlt.qualityForm.value"
          :quality-loading="qlt.qualityLoading.value"
          :quality-result="qlt.qualityResult.value"
          :run-quality-prediction="qlt.runQualityPrediction"
        />
      </el-tab-pane>
    </el-tabs>

    <!-- 租户对话框（与租户 tab 共享 useTnt 状态） -->
    <TntForm
      :model-value="tnt.tenantDialogVisible.value"
      :title="tnt.tenantDialogTitle.value"
      :form="tnt.tenantForm.value"
      :on-submit="tnt.submitTenant"
      :on-cancel="() => (tnt.tenantDialogVisible.value = false)"
      @update:model-value="(v: boolean) => (tnt.tenantDialogVisible.value = v)"
    />
  </div>
</template>

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

<template>
  <div class="advanced-page">
    <el-tabs v-model="activeTab" @tab-change="(tab: any) => loadTab(tab)">
      <el-tab-pane label="AI 分析" name="ai">
        <AdvancedAiTab
          :forecast-period="forecastPeriod"
          :forecast-loading="forecastLoading"
          :forecast-result="forecastResult"
          :optimize-loading="optimizeLoading"
          :optimize-result="optimizeResult"
          :profile-loading="profileLoading"
          :profile-result="profileResult"
          :format-money="formatMoney"
          @run-forecast="runSalesForecast"
          @optimize-inventory="optimizeInventory"
          @run-profile="runCustomerProfile"
        />
      </el-tab-pane>

      <el-tab-pane label="报表引擎" name="report">
        <AdvancedReportTab
          :templates="reportTemplates"
          @design-report="openReportDesign"
          @preview="previewReport"
          @generate="generateReport"
          @edit="editReportTemplate"
        />
      </el-tab-pane>

      <el-tab-pane label="多租户管理" name="tenant">
        <AdvancedTenantTab
          :tenants="tenants"
          :loading="tenantLoading"
          @new-tenant="openTenantDialog"
          @edit-tenant="openTenantDialog"
          @update-status="updateTenantStatus"
          @delete-tenant="deleteTenant"
        />
      </el-tab-pane>

      <el-tab-pane label="工艺优化" name="recipe">
        <AdvancedRecipeTab
          :recipe-form="recipeForm"
          :recipe-loading="recipeLoading"
          :recipe-result="recipeResult"
          :recommend-machines="recommendMachines"
          @update:recipe-form="(v: any) => Object.assign(recipeForm, v)"
          @recommend="getRecipeRecommendation"
          @recommend-machine="recommendMachine"
          @save-template="saveRecipeTemplate"
        />
      </el-tab-pane>

      <el-tab-pane label="质量预测" name="quality">
        <AdvancedQualityTab
          :quality-form="qualityForm"
          :quality-loading="qualityLoading"
          :quality-result="qualityResult"
          :defect-stats="defectStats"
          :format-money="formatMoney"
          @update:quality-form="(v: any) => Object.assign(qualityForm, v)"
          @predict="predictQuality"
          @analyze-defect="analyzeDefect"
        />
      </el-tab-pane>
    </el-tabs>

    <!-- 租户对话框 -->
    <el-dialog v-model="tenantDialogVisible" :title="tenantDialogTitle" width="600px">
      <el-form :model="tenantForm" label-width="100px">
        <el-form-item label="租户名称" required>
          <el-input v-model="tenantForm.name" placeholder="请输入租户名称" />
        </el-form-item>
        <el-form-item label="租户编码" required>
          <el-input v-model="tenantForm.code" placeholder="请输入租户编码" />
        </el-form-item>
        <el-form-item label="联系人">
          <el-input v-model="tenantForm.contact_person" placeholder="请输入联系人" />
        </el-form-item>
        <el-form-item label="联系电话">
          <el-input v-model="tenantForm.contact_phone" placeholder="请输入联系电话" />
        </el-form-item>
        <el-form-item label="邮箱">
          <el-input v-model="tenantForm.email" placeholder="请输入邮箱" />
        </el-form-item>
        <el-form-item label="地址">
          <el-input v-model="tenantForm.address" placeholder="请输入地址" />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="tenantForm.status" placeholder="请选择状态" style="width: 100%">
            <el-option label="正常" value="active" />
            <el-option label="停用" value="inactive" />
            <el-option label="暂停" value="suspended" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="tenantDialogVisible = false">取消</el-button>
          <el-button type="primary" @click="submitTenant">确定</el-button>
        </span>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import AdvancedAiTab from './tabs/AdvancedAiTab.vue'
import AdvancedReportTab from './tabs/AdvancedReportTab.vue'
import AdvancedTenantTab from './tabs/AdvancedTenantTab.vue'
import AdvancedRecipeTab from './tabs/AdvancedRecipeTab.vue'
import AdvancedQualityTab from './tabs/AdvancedQualityTab.vue'
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

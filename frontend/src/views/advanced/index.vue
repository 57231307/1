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
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  forecastSales,
  optimizeInventory,
  detectAnomalies,
  getRecommendations as getRecommendationsApi,
  optimizeRecipe,
  predictQuality,
  listReportTemplates,
  executeReport as executeReportApi,
  listTenants,
  createTenant,
  updateTenant,
  deleteTenant as deleteTenantApi,
} from '@/api/advanced'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'

const activeTab = ref('ai')
const hasLoaded = createLazyLoader()
const forecastPeriod = ref('3m')
const forecastLoading = ref(false)
const forecastResult = ref<any>(null)
const inventoryLoading = ref(false)
const inventoryResult = ref<any>(null)
const anomalyType = ref('sales')
const anomalyLoading = ref(false)
const anomalyResult = ref<any>(null)
const recommendLoading = ref(false)
const recommendationResult = ref<any>(null)
const reportTemplates = ref<any[]>([])
const reportLoading = ref(false)
const reportResultVisible = ref(false)
const reportData = ref<any[]>([])
const reportColumns = ref<any[]>([])
const tenants = ref<any[]>([])
const tenantLoading = ref(false)

// 工艺优化（A2-1）状态
const recipeForm = ref({
  color_no: '',
  fabric_type: '棉',
  dye_type: '',
  color_name: '',
  k: 5,
})
const recipeLoading = ref(false)
const recipeResult = ref<any>(null)

const runRecipeOptimization = async () => {
  if (!recipeForm.value.color_no.trim()) {
    ElMessage.warning('请输入色号')
    return
  }
  if (!recipeForm.value.fabric_type) {
    ElMessage.warning('请选择布类')
    return
  }
  recipeLoading.value = true
  try {
    const payload: any = {
      color_no: recipeForm.value.color_no.trim(),
      fabric_type: recipeForm.value.fabric_type,
      k: recipeForm.value.k,
    }
    if (recipeForm.value.dye_type && recipeForm.value.dye_type.trim()) {
      payload.dye_type = recipeForm.value.dye_type.trim()
    }
    if (recipeForm.value.color_name && recipeForm.value.color_name.trim()) {
      payload.color_name = recipeForm.value.color_name.trim()
    }
    const res: any = await optimizeRecipe(payload)
    recipeResult.value = res.data!
    ElMessage.success('推荐生成完成')
  } catch (e: any) {
    ElMessage.error(e.message || '推荐失败')
  } finally {
    recipeLoading.value = false
  }
}

// 质量预测（A2-2）状态
const qualityForm = ref<{
  product_id: number | null
  inspection_type: string
  window_days: number
}>({
  product_id: null,
  inspection_type: '',
  window_days: 90,
})
const qualityLoading = ref(false)
const qualityResult = ref<any>(null)

const runQualityPrediction = async () => {
  qualityLoading.value = true
  try {
    const payload: any = {
      window_days: qualityForm.value.window_days,
    }
    if (qualityForm.value.product_id !== null && qualityForm.value.product_id !== undefined) {
      payload.product_id = qualityForm.value.product_id
    }
    if (qualityForm.value.inspection_type && qualityForm.value.inspection_type.trim()) {
      payload.inspection_type = qualityForm.value.inspection_type.trim()
    }
    const res: any = await predictQuality(payload)
    qualityResult.value = res.data!
    ElMessage.success('预测完成')
  } catch (e: any) {
    ElMessage.error(e.message || '预测失败')
  } finally {
    qualityLoading.value = false
  }
}

const formatMoney = (amount: number) =>
  '¥' + (amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00')

const runSalesForecast = async () => {
  forecastLoading.value = true
  try {
    const res: any = await forecastSales({ period: forecastPeriod.value })
    forecastResult.value = res.data!
    ElMessage.success('预测完成')
  } catch (e: any) {
    ElMessage.error(e.message || '预测失败')
  } finally {
    forecastLoading.value = false
  }
}

const runInventoryOptimization = async () => {
  inventoryLoading.value = true
  try {
    const res: any = await optimizeInventory()
    inventoryResult.value = res.data!
    ElMessage.success('优化建议生成完成')
  } catch (e: any) {
    ElMessage.error(e.message || '生成失败')
  } finally {
    inventoryLoading.value = false
  }
}

const runAnomalyDetection = async () => {
  anomalyLoading.value = true
  try {
    const res: any = await detectAnomalies({ data_type: anomalyType.value })
    anomalyResult.value = res.data!
    ElMessage.success('检测完成')
  } catch (e: any) {
    ElMessage.error(e.message || '检测失败')
  } finally {
    anomalyLoading.value = false
  }
}

const getRecommendations = async () => {
  recommendLoading.value = true
  try {
    const res: any = await getRecommendationsApi()
    recommendationResult.value = res.data!
    ElMessage.success('推荐获取完成')
  } catch (e: any) {
    ElMessage.error(e.message || '获取失败')
  } finally {
    recommendLoading.value = false
  }
}

const fetchReportTemplates = async () => {
  reportLoading.value = true
  try {
    const res: any = await listReportTemplates()
    reportTemplates.value = res.data! || []
  } finally {
    reportLoading.value = false
  }
}

const executeReport = async (row: any) => {
  try {
    const res: any = await executeReportApi(row.template_code)
    reportData.value = res.data?.data
    reportColumns.value = res.data?.columns || []
    reportResultVisible.value = true
  } catch (e: any) {
    ElMessage.error(e.message || '执行失败')
  }
}

const exportReport = async (_row: any, _format: string) => {
  try {
    ElMessage.success('导出成功')
  } catch (e: any) {
    ElMessage.error(e.message || '导出失败')
  }
}

const fetchTenants = async () => {
  tenantLoading.value = true
  try {
    const res: any = await listTenants()
    tenants.value = res.data! || []
  } finally {
    tenantLoading.value = false
  }
}

const openTenantDialog = (row?: any) => {
  if (row) {
    tenantDialogTitle.value = '编辑租户'
    tenantForm.value = { ...row }
  } else {
    tenantDialogTitle.value = '新建租户'
    tenantForm.value = {
      id: null,
      name: '',
      code: '',
      contact_person: '',
      contact_phone: '',
      email: '',
      address: '',
      status: 'active',
    }
  }
  tenantDialogVisible.value = true
}

const updateTenantStatus = async (row: any) => {
  try {
    const newStatus = row.status === 'active' ? 'inactive' : 'active'
    await ElMessageBox.confirm(
      `确定${newStatus === 'active' ? '启用' : '禁用'}租户 "${row.name}" 吗？`,
      '确认',
      { type: 'warning' }
    )
    await updateTenant(row.id, { status: newStatus })
    ElMessage.success('状态更新成功')
    fetchTenants()
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
  }
}

const tenantDialogVisible = ref(false)
const tenantDialogTitle = ref('新建租户')
const tenantForm = ref({
  id: null as number | null,
  name: '',
  code: '',
  contact_person: '',
  contact_phone: '',
  email: '',
  address: '',
  status: 'active',
})

const submitTenant = async () => {
  try {
    if (tenantForm.value.id) {
      await updateTenant(tenantForm.value.id, tenantForm.value)
      ElMessage.success('更新成功')
    } else {
      await createTenant(tenantForm.value)
      ElMessage.success('创建成功')
    }
    tenantDialogVisible.value = false
    fetchTenants()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  }
}

const deleteTenant = async (row: any) => {
  try {
    await ElMessageBox.confirm(`确定删除租户 "${row.name}" 吗？`, '确认', { type: 'warning' })
    await deleteTenantApi(row.id)
    ElMessage.success('删除成功')
    fetchTenants()
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
  }
}

const tabLoaders: Record<string, () => void> = {
  ai: () => {},
  report: fetchReportTemplates,
  tenant: fetchTenants,
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

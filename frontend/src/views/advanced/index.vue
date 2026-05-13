<template>
  <div class="advanced-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="AI分析" name="ai">
        <div class="page-header">
          <h2 class="page-title">AI分析</h2>
        </div>

        <el-row :gutter="20" class="card-row">
          <el-col :span="12">
            <el-card shadow="hover" class="feature-card">
              <template #header>
                <div class="card-header">
                  <span>销售预测</span>
                </div>
              </template>
              <el-form :inline="true" :model="salesForecastParams">
                <el-form-item label="周期">
                  <el-select v-model="salesForecastParams.period" placeholder="选择周期">
                    <el-option label="月度" value="month" />
                    <el-option label="季度" value="quarter" />
                    <el-option label="年度" value="year" />
                  </el-select>
                </el-form-item>
                <el-form-item label="产品">
                  <el-select v-model="salesForecastParams.product_id" placeholder="选择产品" clearable>
                    <el-option label="全部产品" :value="undefined" />
                  </el-select>
                </el-form-item>
                <el-form-item>
                  <el-button type="primary" :loading="salesForecastLoading" @click="fetchSalesForecast">分析</el-button>
                </el-form-item>
              </el-form>
              <el-divider />
              <div v-if="salesForecastResult" class="result-section">
                <el-descriptions :column="2" border>
                  <el-descriptions-item label="预测周期">
                    {{ salesForecastParams.period === 'month' ? '月度' : salesForecastParams.period === 'quarter' ? '季度' : '年度' }}
                  </el-descriptions-item>
                  <el-descriptions-item label="预测金额">
                    <span class="highlight-amount">{{ formatMoney(salesForecastResult.forecast_amount) }}</span>
                  </el-descriptions-item>
                  <el-descriptions-item label="同比增长">
                    <span :class="salesForecastResult.growth_rate >= 0 ? 'growth-positive' : 'growth-negative'">
                      {{ salesForecastResult.growth_rate >= 0 ? '+' : '' }}{{ salesForecastResult.growth_rate }}%
                    </span>
                  </el-descriptions-item>
                  <el-descriptions-item label="置信度">
                    <el-progress :percentage="salesForecastResult.confidence * 100" :status="salesForecastResult.confidence * 100 >= 80 ? 'success' : 'warning'" />
                  </el-descriptions-item>
                </el-descriptions>
              </div>
              <div v-else class="empty-state">
                <el-empty description="点击分析按钮获取销售预测数据" />
              </div>
            </el-card>
          </el-col>

          <el-col :span="12">
            <el-card shadow="hover" class="feature-card">
              <template #header>
                <div class="card-header">
                  <span>库存优化</span>
                </div>
              </template>
              <el-form :inline="true" :model="inventoryOptimizeParams">
                <el-form-item label="仓库">
                  <el-select v-model="inventoryOptimizeParams.warehouse_id" placeholder="选择仓库" clearable>
                    <el-option label="全部仓库" :value="undefined" />
                  </el-select>
                </el-form-item>
                <el-form-item>
                  <el-button type="primary" :loading="inventoryOptimizeLoading" @click="fetchInventoryOptimize">分析</el-button>
                </el-form-item>
              </el-form>
              <el-divider />
              <div v-if="inventoryOptimizeResult" class="result-section">
                <el-row :gutter="10">
                  <el-col :span="8">
                    <el-statistic title="建议订货点" :value="inventoryOptimizeResult.reorder_point" />
                  </el-col>
                  <el-col :span="8">
                    <el-statistic title="安全库存" :value="inventoryOptimizeResult.safety_stock" />
                  </el-col>
                  <el-col :span="8">
                    <el-statistic title="预计节省" :value="inventoryOptimizeResult.savings" :precision="2" prefix="¥" />
                  </el-col>
                </el-row>
                <el-divider />
                <h4>建议订单</h4>
                <el-table :data="inventoryOptimizeResult.suggestions" stripe size="small">
                  <el-table-column prop="product_name" label="产品" />
                  <el-table-column prop="suggested_qty" label="建议数量" align="right" />
                  <el-table-column prop="priority" label="优先级">
                    <template #default="{ row }">
                      <el-tag :type="getPriorityType(row.priority)">{{ row.priority }}</el-tag>
                    </template>
                  </el-table-column>
                </el-table>
              </div>
              <div v-else class="empty-state">
                <el-empty description="点击分析按钮获取库存优化建议" />
              </div>
            </el-card>
          </el-col>
        </el-row>

        <el-row :gutter="20" class="card-row">
          <el-col :span="12">
            <el-card shadow="hover" class="feature-card">
              <template #header>
                <div class="card-header">
                  <span>异常检测</span>
                </div>
              </template>
              <el-form :inline="true" :model="anomalyParams">
                <el-form-item label="周期">
                  <el-select v-model="anomalyParams.period" placeholder="选择周期">
                    <el-option label="近7天" value="7d" />
                    <el-option label="近30天" value="30d" />
                    <el-option label="近90天" value="90d" />
                  </el-select>
                </el-form-item>
                <el-form-item label="数据类型">
                  <el-select v-model="anomalyParams.data_type" placeholder="选择类型">
                    <el-option label="销售" value="sales" />
                    <el-option label="库存" value="inventory" />
                    <el-option label="采购" value="purchase" />
                  </el-select>
                </el-form-item>
                <el-form-item>
                  <el-button type="primary" :loading="anomalyLoading" @click="fetchAnomalies">检测</el-button>
                </el-form-item>
              </el-form>
              <el-divider />
              <div v-if="anomalyResult" class="result-section">
                <h4>检测到的异常 ({{ anomalyResult.anomalies.length }})</h4>
                <el-table :data="anomalyResult.anomalies" stripe size="small" max-height="300">
                  <el-table-column prop="date" label="日期" width="120" />
                  <el-table-column prop="type" label="类型" width="100">
                    <template #default="{ row }">
                      <el-tag :type="row.type === 'spike' ? 'danger' : 'warning'">
                        {{ row.type === 'spike' ? '异常峰值' : '异常低谷' }}
                      </el-tag>
                    </template>
                  </el-table-column>
                  <el-table-column prop="value" label="数值" width="120" align="right" />
                  <el-table-column prop="severity" label="严重程度" width="100">
                    <template #default="{ row }">
                      <el-tag :type="getSeverityType(row.severity)">{{ row.severity }}</el-tag>
                    </template>
                  </el-table-column>
                </el-table>
              </div>
              <div v-else class="empty-state">
                <el-empty description="点击检测按钮查找数据异常" />
              </div>
            </el-card>
          </el-col>

          <el-col :span="12">
            <el-card shadow="hover" class="feature-card">
              <template #header>
                <div class="card-header">
                  <span>智能建议</span>
                </div>
              </template>
              <el-form :inline="true">
                <el-form-item>
                  <el-button type="primary" :loading="recommendationLoading" @click="fetchRecommendations">获取建议</el-button>
                </el-form-item>
              </el-form>
              <el-divider />
              <div v-if="recommendationResult" class="result-section">
                <h4>智能建议</h4>
                <el-timeline>
                  <el-timeline-item
                    v-for="(item, index) in recommendationResult.recommendations"
                    :key="index"
                    :timestamp="item.date"
                    :type="getRecommendationType(item.type)">
                    <el-card shadow="hover" size="small">
                      <div class="recommendation-item">
                        <strong>{{ item.title }}</strong>
                        <p>{{ item.description }}</p>
                        <el-tag v-if="item.priority === 'high'" type="danger" size="small">高优先级</el-tag>
                        <el-tag v-else-if="item.priority === 'medium'" type="warning" size="small">中优先级</el-tag>
                        <el-tag v-else type="info" size="small">低优先级</el-tag>
                      </div>
                    </el-card>
                  </el-timeline-item>
                </el-timeline>
              </div>
              <div v-else class="empty-state">
                <el-empty description="点击获取建议按钮查看智能建议" />
              </div>
            </el-card>
          </el-col>
        </el-row>
      </el-tab-pane>

      <el-tab-pane label="报表中心" name="report">
        <div class="page-header">
          <h2 class="page-title">报表中心</h2>
        </div>

        <el-card shadow="hover" class="filter-card">
          <el-form :inline="true" :model="reportParams">
            <el-form-item label="报表模板">
              <el-select v-model="reportParams.template_code" placeholder="选择报表" style="width: 200px">
                <el-option
                  v-for="template in reportTemplates"
                  :key="template.template_code"
                  :label="template.template_name"
                  :value="template.template_code" />
              </el-select>
            </el-form-item>
            <el-form-item v-for="param in currentTemplateParams" :key="param.name" :label="param.description">
              <el-input v-if="param.type === 'string'" v-model="reportParams[param.name]" :placeholder="param.description" clearable />
              <el-date-picker
                v-else-if="param.type === 'date'"
                v-model="reportParams[param.name]"
                type="date"
                :placeholder="param.description"
                value-format="YYYY-MM-DD" />
              <el-select v-else-if="param.type === 'select'" v-model="reportParams[param.name]" :placeholder="param.description" clearable>
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="reportLoading" @click="executeReport">生成报表</el-button>
              <el-button :loading="exportLoading" @click="exportReport('pdf')">导出PDF</el-button>
              <el-button :loading="exportLoading" @click="exportReport('excel')">导出Excel</el-button>
            </el-form-item>
          </el-form>
        </el-card>

        <el-card shadow="hover" v-loading="reportLoading">
          <div v-if="reportResult" class="report-result">
            <h3>{{ selectedTemplate?.template_name }}</h3>
            <el-table :data="reportResult.data" stripe border>
              <el-table-column
                v-for="(col, index) in reportResult.columns"
                :key="index"
                :prop="col.key"
                :label="col.label"
                :width="col.width"
                :align="col.align" />
            </el-table>
            <el-pagination
              v-if="reportResult.total > 0"
              v-model:current-page="reportParams.page"
              v-model:page-size="reportParams.page_size"
              :total="reportResult.total"
              layout="total, sizes, prev, pager, next"
              @size-change="executeReport"
              @current-change="executeReport" />
          </div>
          <div v-else class="empty-state">
            <el-empty description="选择报表模板并点击生成报表" />
          </div>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="租户管理" name="tenant">
        <div class="page-header">
          <h2 class="page-title">租户管理</h2>
          <el-button type="primary" @click="openTenantDialog">
            <el-icon><Plus /></el-icon>
            新建租户
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table :data="tenants" v-loading="tenantLoading" stripe>
            <el-table-column prop="tenant_code" label="租户编码" width="140" />
            <el-table-column prop="tenant_name" label="租户名称" min-width="150" />
            <el-table-column prop="domain" label="域名" min-width="150" />
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="getTenantStatusType(row.status)" size="small">
                  {{ getTenantStatusLabel(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="subscription_plan" label="订阅计划" width="120" />
            <el-table-column prop="max_users" label="最大用户数" width="100" align="right" />
            <el-table-column prop="current_users" label="当前用户" width="100" align="right" />
            <el-table-column prop="subscription_start_date" label="开始日期" width="120" />
            <el-table-column prop="subscription_end_date" label="结束日期" width="120" />
            <el-table-column label="操作" width="180" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openStatusDialog(row)">状态</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="tenantDialogVisible" title="新建租户" width="600px">
      <el-form ref="tenantFormRef" :model="tenantForm" :rules="tenantRules" label-width="120px">
        <el-form-item label="租户编码" prop="tenant_code">
          <el-input v-model="tenantForm.tenant_code" />
        </el-form-item>
        <el-form-item label="租户名称" prop="tenant_name">
          <el-input v-model="tenantForm.tenant_name" />
        </el-form-item>
        <el-form-item label="域名" prop="domain">
          <el-input v-model="tenantForm.domain" />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="订阅计划" prop="subscription_plan">
              <el-select v-model="tenantForm.subscription_plan" placeholder="选择计划" style="width: 100%">
                <el-option label="基础版" value="basic" />
                <el-option label="专业版" value="professional" />
                <el-option label="企业版" value="enterprise" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="最大用户数" prop="max_users">
              <el-input-number v-model="tenantForm.max_users" :min="1" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="开始日期" prop="subscription_start_date">
              <el-date-picker v-model="tenantForm.subscription_start_date" type="date" style="width: 100%" value-format="YYYY-MM-DD" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="结束日期" prop="subscription_end_date">
              <el-date-picker v-model="tenantForm.subscription_end_date" type="date" style="width: 100%" value-format="YYYY-MM-DD" />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
      <template #footer>
        <el-button @click="tenantDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="tenantSubmitLoading" @click="submitTenant">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="statusDialogVisible" title="更新状态" width="400px">
      <el-form label-width="100px">
        <el-form-item label="当前状态">
          <el-tag :type="getTenantStatusType(currentTenant?.status)">{{ getTenantStatusLabel(currentTenant?.status) }}</el-tag>
        </el-form-item>
        <el-form-item label="新状态">
          <el-select v-model="newTenantStatus" placeholder="选择状态" style="width: 100%">
            <el-option label="激活" value="active" />
            <el-option label="停用" value="inactive" />
            <el-option label="暂停" value="suspended" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="statusDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="statusSubmitLoading" @click="submitTenantStatus">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import { forecastSales, optimizeInventory, detectAnomalies, getRecommendations } from '@/api/ai'
import { listReportTemplates, executeReport, exportReport, type ReportTemplate } from '@/api/report'
import { listTenants, createTenant, getTenant, updateTenantStatus, type Tenant } from '@/api/tenant'

const activeTab = ref('ai')

const salesForecastParams = reactive({ period: 'month', product_id: undefined as number | undefined })
const inventoryOptimizeParams = reactive({ warehouse_id: undefined as number | undefined })
const anomalyParams = reactive({ period: '30d', data_type: 'sales' })
const salesForecastLoading = ref(false)
const inventoryOptimizeLoading = ref(false)
const anomalyLoading = ref(false)
const recommendationLoading = ref(false)
const salesForecastResult = ref<any>(null)
const inventoryOptimizeResult = ref<any>(null)
const anomalyResult = ref<any>(null)
const recommendationResult = ref<any>(null)

const reportParams = reactive({
  template_code: '',
  page: 1,
  page_size: 20
})
const reportTemplates = ref<ReportTemplate[]>([])
const reportLoading = ref(false)
const exportLoading = ref(false)
const reportResult = ref<any>(null)

const tenants = ref<Tenant[]>([])
const tenantLoading = ref(false)
const tenantDialogVisible = ref(false)
const statusDialogVisible = ref(false)
const tenantFormRef = ref<FormInstance>()
const tenantSubmitLoading = ref(false)
const statusSubmitLoading = ref(false)
const tenantForm = reactive({
  id: 0,
  tenant_code: '',
  tenant_name: '',
  domain: '',
  status: 'active' as 'active' | 'inactive' | 'suspended',
  max_users: 10,
  current_users: 0,
  subscription_plan: 'basic',
  subscription_start_date: '',
  subscription_end_date: ''
})
const currentTenant = ref<Tenant | null>(null)
const newTenantStatus = ref<string>('active')

const tenantRules: FormRules = {
  tenant_code: [{ required: true, message: '请输入租户编码', trigger: 'blur' }],
  tenant_name: [{ required: true, message: '请输入租户名称', trigger: 'blur' }],
  subscription_plan: [{ required: true, message: '请选择订阅计划', trigger: 'change' }],
  max_users: [{ required: true, message: '请输入最大用户数', trigger: 'blur' }]
}

const selectedTemplate = computed(() => 
  reportTemplates.value.find(t => t.template_code === reportParams.template_code)
)

const currentTemplateParams = computed(() => 
  selectedTemplate.value?.parameters || []
)

const formatMoney = (amount: number) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2, style: 'currency', currency: 'CNY' }) || '¥0.00'
}

const getPriorityType = (priority: string) => {
  const map: Record<string, any> = { high: 'danger', medium: 'warning', low: 'info' }
  return map[priority] || 'info'
}

const getSeverityType = (severity: string) => {
  const map: Record<string, any> = { critical: 'danger', high: 'warning', medium: 'info', low: '' }
  return map[severity] || ''
}

const getRecommendationType = (type: string) => {
  const map: Record<string, any> = { opportunity: 'success', warning: 'warning', risk: 'danger', info: 'primary' }
  return map[type] || ''
}

const getTenantStatusLabel = (status?: string) => {
  const map: Record<string, string> = { active: '激活', inactive: '停用', suspended: '暂停' }
  return map[status || ''] || status || ''
}

const getTenantStatusType = (status?: string) => {
  const map: Record<string, any> = { active: 'success', inactive: 'info', suspended: 'warning' }
  return map[status || ''] || 'info'
}

const fetchSalesForecast = async () => {
  salesForecastLoading.value = true
  try {
    const res = await forecastSales(salesForecastParams)
    salesForecastResult.value = res.data
  } catch (error: any) {
    ElMessage.error(error.message || '获取销售预测失败')
  } finally {
    salesForecastLoading.value = false
  }
}

const fetchInventoryOptimize = async () => {
  inventoryOptimizeLoading.value = true
  try {
    const res = await optimizeInventory(inventoryOptimizeParams)
    inventoryOptimizeResult.value = res.data
  } catch (error: any) {
    ElMessage.error(error.message || '获取库存优化建议失败')
  } finally {
    inventoryOptimizeLoading.value = false
  }
}

const fetchAnomalies = async () => {
  anomalyLoading.value = true
  try {
    const res = await detectAnomalies(anomalyParams)
    anomalyResult.value = res.data
  } catch (error: any) {
    ElMessage.error(error.message || '异常检测失败')
  } finally {
    anomalyLoading.value = false
  }
}

const fetchRecommendations = async () => {
  recommendationLoading.value = true
  try {
    const res = await getRecommendations()
    recommendationResult.value = res.data
  } catch (error: any) {
    ElMessage.error(error.message || '获取智能建议失败')
  } finally {
    recommendationLoading.value = false
  }
}

const fetchReportTemplates = async () => {
  try {
    const res = await listReportTemplates()
    reportTemplates.value = res.data || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取报表模板失败')
  }
}

const executeReport = async () => {
  if (!reportParams.template_code) {
    ElMessage.warning('请选择报表模板')
    return
  }
  reportLoading.value = true
  try {
    const res = await executeReport(reportParams.template_code, reportParams)
    reportResult.value = res.data
  } catch (error: any) {
    ElMessage.error(error.message || '生成报表失败')
  } finally {
    reportLoading.value = false
  }
}

const exportReport = async (format: string) => {
  if (!reportParams.template_code) {
    ElMessage.warning('请选择报表模板')
    return
  }
  exportLoading.value = true
  try {
    const res = await exportReport(reportParams.template_code, format as any, reportParams)
    const url = window.URL.createObjectURL(res)
    const link = document.createElement('a')
    link.href = url
    link.download = `${selectedTemplate.value?.template_name || 'report'}.${format}`
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    window.URL.revokeObjectURL(url)
    ElMessage.success('导出成功')
  } catch (error: any) {
    ElMessage.error(error.message || '导出报表失败')
  } finally {
    exportLoading.value = false
  }
}

const fetchTenants = async () => {
  tenantLoading.value = true
  try {
    const res = await listTenants()
    tenants.value = res.data || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取租户列表失败')
  } finally {
    tenantLoading.value = false
  }
}

const openTenantDialog = () => {
  Object.assign(tenantForm, {
    id: 0,
    tenant_code: '',
    tenant_name: '',
    domain: '',
    status: 'active',
    max_users: 10,
    current_users: 0,
    subscription_plan: 'basic',
    subscription_start_date: '',
    subscription_end_date: ''
  })
  tenantDialogVisible.value = true
}

const submitTenant = async () => {
  const valid = await tenantFormRef.value?.validate()
  if (!valid) return

  tenantSubmitLoading.value = true
  try {
    await createTenant(tenantForm)
    ElMessage.success('创建成功')
    tenantDialogVisible.value = false
    fetchTenants()
  } catch (error: any) {
    ElMessage.error(error.message || '创建租户失败')
  } finally {
    tenantSubmitLoading.value = false
  }
}

const openStatusDialog = (row: Tenant) => {
  currentTenant.value = row
  newTenantStatus.value = row.status
  statusDialogVisible.value = true
}

const submitTenantStatus = async () => {
  if (!currentTenant.value) return
  
  statusSubmitLoading.value = true
  try {
    await updateTenantStatus(currentTenant.value.id, { status: newTenantStatus.value })
    ElMessage.success('状态更新成功')
    statusDialogVisible.value = false
    fetchTenants()
  } catch (error: any) {
    ElMessage.error(error.message || '更新状态失败')
  } finally {
    statusSubmitLoading.value = false
  }
}

onMounted(() => {
  fetchReportTemplates()
  fetchTenants()
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

.card-row {
  margin-bottom: 20px;
}

.feature-card {
  height: 100%;
}

.card-header {
  font-weight: 600;
}

.filter-card {
  margin-bottom: 20px;
}

.result-section {
  padding: 10px 0;
}

.highlight-amount {
  font-size: 20px;
  font-weight: 600;
  color: #409eff;
}

.growth-positive {
  color: #67c23a;
  font-weight: 600;
}

.growth-negative {
  color: #f56c6c;
  font-weight: 600;
}

.empty-state {
  padding: 20px 0;
}

.report-result h3 {
  margin-top: 0;
  margin-bottom: 16px;
}

.recommendation-item {
  line-height: 1.6;
}

.recommendation-item strong {
  display: block;
  margin-bottom: 8px;
}
</style>

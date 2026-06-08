<template>
  <div class="advanced-page">
    <el-tabs v-model="activeTab" @tab-change="(tab: any) => loadTab(tab)">
      <el-tab-pane label="AI 分析" name="ai">
        <div class="page-header">
          <h2 class="page-title">AI 智能分析</h2>
        </div>

        <el-row :gutter="20">
          <el-col :span="12">
            <el-card shadow="hover" class="mb-20">
              <template #header><div class="card-header">销售预测</div></template>
              <el-form label-width="100px">
                <el-form-item label="预测周期">
                  <el-select v-model="forecastPeriod" style="width: 100%">
                    <el-option label="未来 3 个月" value="3m" />
                    <el-option label="未来 6 个月" value="6m" />
                    <el-option label="未来 12 个月" value="12m" />
                  </el-select>
                </el-form-item>
                <el-form-item>
                  <el-button type="primary" :loading="forecastLoading" @click="runSalesForecast"
                    >开始预测</el-button
                  >
                </el-form-item>
              </el-form>
              <el-empty v-if="!forecastResult" description="点击开始预测" />
              <div v-else>
                <h4>预测结果</h4>
                <el-divider />
                <el-descriptions :column="2" border>
                  <el-descriptions-item label="预测销售额">{{
                    formatMoney(forecastResult.sales_amount)
                  }}</el-descriptions-item>
                  <el-descriptions-item label="预测订单数">{{
                    forecastResult.order_count
                  }}</el-descriptions-item>
                  <el-descriptions-item label="置信度"
                    >{{ forecastResult.confidence }}%</el-descriptions-item
                  >
                  <el-descriptions-item label="预测趋势">{{
                    forecastResult.trend
                  }}</el-descriptions-item>
                </el-descriptions>
              </div>
            </el-card>

            <el-card shadow="hover">
              <template #header><div class="card-header">库存优化建议</div></template>
              <el-button
                type="primary"
                :loading="inventoryLoading"
                @click="runInventoryOptimization"
                >生成建议</el-button
              >
              <el-divider />
              <el-empty v-if="!inventoryResult" description="点击生成优化建议" />
              <div v-else>
                <el-alert type="success" :title="inventoryResult.summary" show-icon class="mb-10" />
                <el-table :data="inventoryResult.items" stripe>
                  <el-table-column prop="product_name" label="产品" width="150" />
                  <el-table-column prop="suggestion" label="建议" min-width="200" />
                  <el-table-column prop="priority" label="优先级" width="100">
                    <template #default="{ row }">
                      <el-tag
                        :type="
                          row.priority === 'high'
                            ? 'danger'
                            : row.priority === 'medium'
                              ? 'warning'
                              : 'info'
                        "
                        size="small"
                      >
                        {{
                          row.priority === 'high' ? '高' : row.priority === 'medium' ? '中' : '低'
                        }}
                      </el-tag>
                    </template>
                  </el-table-column>
                </el-table>
              </div>
            </el-card>
          </el-col>

          <el-col :span="12">
            <el-card shadow="hover" class="mb-20">
              <template #header><div class="card-header">异常检测</div></template>
              <el-form label-width="100px">
                <el-form-item label="数据类型">
                  <el-select v-model="anomalyType" style="width: 100%">
                    <el-option label="销售数据" value="sales" />
                    <el-option label="库存数据" value="inventory" />
                    <el-option label="质量数据" value="quality" />
                  </el-select>
                </el-form-item>
                <el-form-item>
                  <el-button type="primary" :loading="anomalyLoading" @click="runAnomalyDetection"
                    >检测异常</el-button
                  >
                </el-form-item>
              </el-form>
              <el-empty v-if="!anomalyResult" description="点击开始检测" />
              <div v-else>
                <el-table :data="anomalyResult" stripe>
                  <el-table-column prop="item" label="检测项" width="150" />
                  <el-table-column prop="type" label="类型" width="100">
                    <template #default="{ row }">
                      <el-tag
                        :type="row.severity === 'critical' ? 'danger' : 'warning'"
                        size="small"
                        >{{ row.type }}</el-tag
                      >
                    </template>
                  </el-table-column>
                  <el-table-column prop="description" label="描述" min-width="200" />
                  <el-table-column prop="severity" label="严重程度" width="100" />
                </el-table>
              </div>
            </el-card>

            <el-card shadow="hover">
              <template #header><div class="card-header">智能推荐</div></template>
              <el-button type="primary" :loading="recommendLoading" @click="getRecommendations"
                >获取推荐</el-button
              >
              <el-divider />
              <el-empty v-if="!recommendationResult" description="点击获取推荐" />
              <div v-else>
                <el-timeline>
                  <el-timeline-item
                    v-for="(rec, i) in recommendationResult"
                    :key="i"
                    :type="rec.type === 'suggestion' ? 'primary' : 'success'"
                    :timestamp="rec.created_at"
                  >
                    {{ rec.content }}
                  </el-timeline-item>
                </el-timeline>
              </div>
            </el-card>
          </el-col>
        </el-row>
      </el-tab-pane>

      <el-tab-pane label="报表引擎" name="report">
        <div class="page-header">
          <h2 class="page-title">报表管理</h2>
        </div>

        <el-card shadow="hover">
          <el-table v-loading="reportLoading" :data="reportTemplates" stripe>
            <el-table-column prop="template_name" label="报表名称" width="180" />
            <el-table-column prop="template_code" label="报表编码" width="120" />
            <el-table-column prop="category" label="分类" width="120" />
            <el-table-column prop="description" label="描述" min-width="200" />
            <el-table-column prop="created_at" label="创建时间" width="160" />
            <el-table-column label="操作" width="200" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="executeReport(row as any)"
                  >执行</el-button
                >
                <el-button type="success" link size="small" @click="exportReport(row, 'excel')"
                  >导出 Excel</el-button
                >
                <el-button type="warning" link size="small" @click="exportReport(row, 'pdf')"
                  >导出 PDF</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>

        <el-dialog v-model="reportResultVisible" title="报表结果" width="80%">
          <div class="report-result">
            <el-empty v-if="!reportData" description="暂无数据" />
            <el-table v-else :data="reportData" border stripe>
              <el-table-column
                v-for="col in reportColumns"
                :key="col.key"
                :prop="col.key"
                :label="col.label"
              />
            </el-table>
          </div>
          <template #footer>
            <el-button @click="reportResultVisible = false">关闭</el-button>
          </template>
        </el-dialog>
      </el-tab-pane>

      <el-tab-pane label="多租户管理" name="tenant">
        <div class="page-header">
          <h2 class="page-title">租户管理</h2>
          <el-button type="primary" @click="openTenantDialog">
            <el-icon><Plus /></el-icon>
            新建租户
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table v-loading="tenantLoading" :data="tenants" stripe>
            <el-table-column prop="tenant_code" label="租户编码" width="120" />
            <el-table-column prop="tenant_name" label="租户名称" width="180" />
            <el-table-column prop="domain" label="域名" width="180" />
            <el-table-column prop="subscription_plan" label="订阅方案" width="120" />
            <el-table-column prop="current_users" label="当前用户" width="100" align="right" />
            <el-table-column prop="max_users" label="最大用户" width="100" align="right" />
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag
                  :type="
                    row.status === 'active'
                      ? 'success'
                      : row.status === 'suspended'
                        ? 'danger'
                        : 'info'
                  "
                  size="small"
                >
                  {{
                    row.status === 'active' ? '正常' : row.status === 'suspended' ? '暂停' : '停用'
                  }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="subscription_start_date" label="开始日期" width="120" />
            <el-table-column prop="subscription_end_date" label="结束日期" width="120" />
            <el-table-column label="操作" width="200" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openTenantDialog(row as any)"
                  >编辑</el-button
                >
                <el-button type="warning" link size="small" @click="updateTenantStatus(row as any)"
                  >更新状态</el-button
                >
                <el-button type="danger" link size="small" @click="deleteTenant(row as any)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
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
import { ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  forecastSales,
  optimizeInventory,
  detectAnomalies,
  getRecommendations as getRecommendationsApi,
  listReportTemplates,
  executeReport as executeReportApi,
  listTenants,
  createTenant,
  updateTenant,
  deleteTenant as deleteTenantApi,
} from '@/api/advanced'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'

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
    if (e !== 'cancel') console.error(e)
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
    if (e !== 'cancel') console.error(e)
  }
}

const tabLoaders: Record<string, () => void> = {
  ai: () => {},
  report: fetchReportTemplates,
  tenant: fetchTenants,
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
.report-result {
  max-height: 60vh;
  overflow: auto;
}
</style>

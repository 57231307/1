<template>
  <div class="tenant-billing">
    <div class="page-header">
      <h2>租户计费管理</h2>
    </div>

    <el-tabs v-model="activeTab" type="border-card">
      <!-- 当前套餐 Tab -->
      <el-tab-pane label="当前套餐" name="current">
        <el-card v-loading="currentPlanLoading">
          <template #header>
            <div class="card-header">
              <span>当前订阅信息</span>
              <el-button type="primary" size="small" @click="handleRenew">续费</el-button>
            </div>
          </template>
          <el-descriptions :column="2" border>
            <el-descriptions-item label="套餐名称">{{ currentPlan.plan_name || '-' }}</el-descriptions-item>
            <el-descriptions-item label="套餐编码">{{ currentPlan.plan_code || '-' }}</el-descriptions-item>
            <el-descriptions-item label="计费周期">{{ currentPlan.billing_cycle || '-' }}</el-descriptions-item>
            <el-descriptions-item label="金额">¥{{ currentPlan.amount || 0 }}</el-descriptions-item>
            <el-descriptions-item label="开始日期">{{ currentPlan.start_date || '-' }}</el-descriptions-item>
            <el-descriptions-item label="结束日期">{{ currentPlan.end_date || '-' }}</el-descriptions-item>
            <el-descriptions-item label="自动续费">
              <el-tag :type="currentPlan.auto_renew ? 'success' : 'info'">
                {{ currentPlan.auto_renew ? '是' : '否' }}
              </el-tag>
            </el-descriptions-item>
          </el-descriptions>
        </el-card>

        <!-- 使用情况 -->
        <el-card class="mt-20" v-loading="usageLoading">
          <template #header>使用情况</template>
          <el-row :gutter="20">
            <el-col :span="8">
              <div class="usage-item">
                <div class="usage-label">用户数</div>
                <el-progress
                  :percentage="usage.user_usage_percent || 0"
                  :format="(p: number) => `${usage.current_users || 0}/${usage.max_users || 0}`"
                />
              </div>
            </el-col>
            <el-col :span="8">
              <div class="usage-item">
                <div class="usage-label">存储空间</div>
                <el-progress
                  :percentage="usage.storage_usage_percent || 0"
                  :format="(p: number) => `${usage.storage_used_mb || 0}/${usage.max_storage_mb || 0} MB`"
                />
              </div>
            </el-col>
            <el-col :span="8">
              <div class="usage-item">
                <div class="usage-label">今日API调用</div>
                <el-progress
                  :percentage="usage.api_usage_percent || 0"
                  :format="(p: number) => `${usage.api_calls_today || 0}/${usage.max_api_calls_per_day || 0}`"
                />
              </div>
            </el-col>
          </el-row>
        </el-card>
      </el-tab-pane>

      <!-- 套餐列表 Tab -->
      <el-tab-pane label="套餐列表" name="plans">
        <el-row :gutter="20">
          <el-col :span="8" v-for="plan in plans" :key="plan.id">
            <el-card class="plan-card" :class="{ 'is-current': plan.id === currentPlan.plan_id }">
              <template #header>
                <div class="plan-header">
                  <span>{{ plan.name }}</span>
                  <el-tag v-if="plan.id === currentPlan.plan_id" type="success">当前套餐</el-tag>
                </div>
              </template>
              <div class="plan-price">
                <span class="price-amount">¥{{ plan.price_monthly }}</span>
                <span class="price-unit">/月</span>
              </div>
              <div class="plan-features">
                <div class="feature-item">
                  <el-icon><Check /></el-icon>
                  <span>最多 {{ plan.max_users }} 个用户</span>
                </div>
                <div class="feature-item">
                  <el-icon><Check /></el-icon>
                  <span>{{ plan.max_storage_mb }} MB 存储空间</span>
                </div>
                <div class="feature-item">
                  <el-icon><Check /></el-icon>
                  <span>每日 {{ plan.max_api_calls_per_day }} 次API调用</span>
                </div>
              </div>
              <div class="plan-action">
                <el-button
                  v-if="plan.id !== currentPlan.plan_id"
                  type="primary"
                  @click="handleUpgrade(plan)"
                >
                  升级到此套餐
                </el-button>
                <el-button v-else disabled>当前套餐</el-button>
              </div>
            </el-card>
          </el-col>
        </el-row>
      </el-tab-pane>

      <!-- 账单列表 Tab -->
      <el-tab-pane label="账单列表" name="invoices">
        <el-table :data="invoices" v-loading="invoicesLoading" border stripe>
          <el-table-column prop="invoice_no" label="账单编号" min-width="150" />
          <el-table-column prop="amount" label="金额" min-width="100">
            <template #default="{ row }">
              <span class="amount">¥{{ row.amount }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="billing_cycle" label="计费周期" min-width="120" />
          <el-table-column prop="status" label="状态" width="100" align="center">
            <template #default="{ row }">
              <el-tag :type="getInvoiceStatusType(row.status)">
                {{ getInvoiceStatusText(row.status) }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="paid_at" label="支付时间" min-width="160" />
          <el-table-column prop="created_at" label="创建时间" min-width="160" />
        </el-table>

        <el-pagination
          v-model:current-page="invoiceQuery.page"
          v-model:page-size="invoiceQuery.page_size"
          :total="invoiceTotal"
          :page-sizes="[10, 20, 50, 100]"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="fetchInvoices"
          @current-change="fetchInvoices"
        />
      </el-tab-pane>
    </el-tabs>

    <!-- 升级套餐对话框 -->
    <el-dialog v-model="upgradeDialogVisible" title="升级套餐" width="400px">
      <el-form :model="upgradeForm" label-width="100px">
        <el-form-item label="目标套餐">
          <el-input :value="upgradeForm.plan_name" disabled />
        </el-form-item>
        <el-form-item label="计费周期">
          <el-radio-group v-model="upgradeForm.billing_cycle">
            <el-radio value="monthly">月付</el-radio>
            <el-radio value="yearly">年付 (8折优惠)</el-radio>
          </el-radio-group>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="upgradeDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleConfirmUpgrade" :loading="upgradeLoading">确认升级</el-button>
      </template>
    </el-dialog>

    <!-- 续费对话框 -->
    <el-dialog v-model="renewDialogVisible" title="续费" width="400px">
      <el-form :model="renewForm" label-width="100px">
        <el-form-item label="续费周期">
          <el-radio-group v-model="renewForm.billing_cycle">
            <el-radio value="monthly">月付</el-radio>
            <el-radio value="yearly">年付 (8折优惠)</el-radio>
          </el-radio-group>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="renewDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleConfirmRenew" :loading="renewLoading">确认续费</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { ElMessage } from 'element-plus'
import { Check } from '@element-plus/icons-vue'
import {
  tenantBillingApi,
  type BillingPlan,
  type CurrentPlanInfo,
  type UsageStats,
  type Invoice,
} from '@/api/tenant-billing'

const activeTab = ref('current')

// 当前套餐
const currentPlan = ref<CurrentPlanInfo>({
  plan_id: 0,
  plan_name: '',
  plan_code: '',
  billing_cycle: '',
  start_date: '',
  end_date: '',
  auto_renew: false,
  amount: 0,
})
const currentPlanLoading = ref(false)

// 使用情况
const usage = ref<UsageStats>({
  current_users: 0,
  max_users: 0,
  storage_used_mb: 0,
  max_storage_mb: 0,
  api_calls_today: 0,
  max_api_calls_per_day: 0,
  user_usage_percent: 0,
  storage_usage_percent: 0,
  api_usage_percent: 0,
})
const usageLoading = ref(false)

// 套餐列表
const plans = ref<BillingPlan[]>([])

// 账单列表
const invoices = ref<Invoice[]>([])
const invoicesLoading = ref(false)
const invoiceTotal = ref(0)
const invoiceQuery = reactive({ page: 1, page_size: 20 })

// 升级对话框
const upgradeDialogVisible = ref(false)
const upgradeLoading = ref(false)
const upgradeForm = reactive({
  plan_id: 0,
  plan_name: '',
  billing_cycle: 'monthly',
})

// 续费对话框
const renewDialogVisible = ref(false)
const renewLoading = ref(false)
const renewForm = reactive({
  billing_cycle: 'monthly',
})

const hasLoaded = createLazyLoader()

onMounted(() => {
  fetchCurrentPlan()
  loadIfNot('usage', fetchUsage, hasLoaded)
  loadIfNot('plans', fetchPlans, hasLoaded)
  loadIfNot('invoices', fetchInvoices, hasLoaded)
})

const fetchCurrentPlan = async () => {
  currentPlanLoading.value = true
  try {
    const res = await tenantBillingApi.getCurrentPlan()
    if (res.data) {
      currentPlan.value = res.data
    }
  } catch (error) {
    console.error('获取当前套餐失败:', error)
  } finally {
    currentPlanLoading.value = false
  }
}

const fetchUsage = async () => {
  usageLoading.value = true
  try {
    const res = await tenantBillingApi.getUsage()
    if (res.data) {
      usage.value = res.data
    }
  } catch (error) {
    console.error('获取使用情况失败:', error)
  } finally {
    usageLoading.value = false
  }
}

const fetchPlans = async () => {
  try {
    const res = await tenantBillingApi.getPlans()
    plans.value = res.data || []
  } catch (error) {
    console.error('获取套餐列表失败:', error)
  }
}

const fetchInvoices = async () => {
  invoicesLoading.value = true
  try {
    const res = await tenantBillingApi.getInvoices(invoiceQuery)
    invoices.value = res.data?.list || []
    invoiceTotal.value = res.data?.total || 0
  } catch (error) {
    console.error('获取账单列表失败:', error)
  } finally {
    invoicesLoading.value = false
  }
}

const handleUpgrade = (plan: BillingPlan) => {
  upgradeForm.plan_id = plan.id!
  upgradeForm.plan_name = plan.name
  upgradeForm.billing_cycle = 'monthly'
  upgradeDialogVisible.value = true
}

const handleConfirmUpgrade = async () => {
  upgradeLoading.value = true
  try {
    await tenantBillingApi.upgradePlan({
      plan_id: upgradeForm.plan_id,
      billing_cycle: upgradeForm.billing_cycle,
    })
    ElMessage.success('升级成功')
    upgradeDialogVisible.value = false
    fetchCurrentPlan()
    fetchUsage()
  } catch (error) {
    console.error('升级失败:', error)
  } finally {
    upgradeLoading.value = false
  }
}

const handleRenew = () => {
  renewForm.billing_cycle = 'monthly'
  renewDialogVisible.value = true
}

const handleConfirmRenew = async () => {
  renewLoading.value = true
  try {
    await tenantBillingApi.renew({ billing_cycle: renewForm.billing_cycle })
    ElMessage.success('续费成功')
    renewDialogVisible.value = false
    fetchCurrentPlan()
  } catch (error) {
    console.error('续费失败:', error)
  } finally {
    renewLoading.value = false
  }
}

const getInvoiceStatusType = (status: string) => {
  const map: Record<string, string> = {
    paid: 'success',
    pending: 'warning',
    failed: 'danger',
  }
  return map[status] || 'info'
}

const getInvoiceStatusText = (status: string) => {
  const map: Record<string, string> = {
    paid: '已支付',
    pending: '待支付',
    failed: '支付失败',
  }
  return map[status] || status
}
</script>

<style scoped>
.tenant-billing {
  padding: 20px;
}

.page-header {
  margin-bottom: 20px;
}

.page-header h2 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.mt-20 {
  margin-top: 20px;
}

.usage-item {
  padding: 10px 0;
}

.usage-label {
  margin-bottom: 10px;
  font-weight: 500;
}

.plan-card {
  margin-bottom: 20px;
}

.plan-card.is-current {
  border-color: #67c23a;
}

.plan-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.plan-price {
  text-align: center;
  padding: 20px 0;
}

.price-amount {
  font-size: 36px;
  font-weight: 600;
  color: #409eff;
}

.price-unit {
  font-size: 14px;
  color: #909399;
}

.plan-features {
  padding: 20px 0;
}

.feature-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 0;
  color: #606266;
}

.feature-item .el-icon {
  color: #67c23a;
}

.plan-action {
  text-align: center;
  padding-top: 20px;
}

.amount {
  font-weight: 600;
  color: #f56c6c;
}

.el-pagination {
  margin-top: 20px;
  justify-content: flex-end;
}
</style>

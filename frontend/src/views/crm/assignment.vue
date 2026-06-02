<template>
  <div class="assignment-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">客户分配</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>CRM</el-breadcrumb-item>
          <el-breadcrumb-item>客户分配</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
    </div>

    <el-row :gutter="20">
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">分配客户</div>
          </template>

          <el-form ref="assignFormRef" :model="assignForm" label-width="100px">
            <el-form-item label="分配方式">
              <el-radio-group v-model="assignMode">
                <el-radio value="single">单个分配</el-radio>
                <el-radio value="batch">批量分配</el-radio>
              </el-radio-group>
            </el-form-item>

            <el-form-item label="选择客户" prop="customer_ids">
              <el-select
                v-model="assignForm.customer_ids"
                :multiple="assignMode === 'batch'"
                filterable
                placeholder="搜索并选择客户"
                style="width: 100%"
              >
                <el-option
                  v-for="item in customerOptions"
                  :key="item.id"
                  :label="item.customer_name"
                  :value="item.id"
                />
              </el-select>
            </el-form-item>

            <el-form-item label="选择业务员" prop="assign_to">
              <el-select
                v-model="assignForm.assign_to"
                placeholder="请选择业务员"
                style="width: 100%"
              >
                <el-option
                  v-for="user in salesUsers"
                  :key="user.id"
                  :label="`${user.name} - ${user.department} (${user.customer_count}个客户)`"
                  :value="user.id"
                  :disabled="!user.active"
                />
              </el-select>
            </el-form-item>

            <el-form-item label="分配原因">
              <el-input
                v-model="assignForm.reason"
                type="textarea"
                :rows="3"
                placeholder="请输入分配原因（选填）"
              />
            </el-form-item>

            <el-form-item>
              <el-button type="primary" :loading="assignLoading" @click="handleAssign">
                <el-icon><Promotion /></el-icon>
                确认分配
              </el-button>
              <el-button @click="resetAssignForm">重置</el-button>
            </el-form-item>
          </el-form>
        </el-card>
      </el-col>

      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">业务员负载</div>
          </template>

          <div class="user-stats">
            <div
              v-for="user in salesUsers"
              :key="user.id"
              class="user-stat-item"
              :class="{ inactive: !user.active }"
            >
              <div class="user-info">
                <el-avatar :size="40">{{ user.name.charAt(0) }}</el-avatar>
                <div class="user-detail">
                  <div class="user-name">{{ user.name }}</div>
                  <div class="user-dept">{{ user.department }}</div>
                </div>
              </div>
              <div class="user-metrics">
                <el-statistic title="客户数" :value="user.customer_count" />
              </div>
              <div class="user-status">
                <el-tag :type="user.active ? 'success' : 'info'" size="small">
                  {{ user.active ? '在职' : '停用' }}
                </el-tag>
              </div>
            </div>
            <el-empty v-if="!salesUsers.length" description="暂无业务员数据" />
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="mt-20">
      <template #header>
        <div class="card-header">
          <span>分配历史</span>
        </div>
      </template>

      <el-form :inline="true" :model="historyQuery" class="history-filter">
        <el-form-item label="分配类型">
          <el-select v-model="historyQuery.assign_type" placeholder="选择类型" clearable>
            <el-option label="手动分配" value="manual" />
            <el-option label="自动分配" value="auto" />
            <el-option label="批量分配" value="batch" />
          </el-select>
        </el-form-item>
        <el-form-item label="业务员">
          <el-select v-model="historyQuery.assigned_to" placeholder="选择业务员" clearable>
            <el-option
              v-for="user in salesUsers"
              :key="user.id"
              :label="user.name"
              :value="user.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchHistory">查询</el-button>
          <el-button @click="handleHistoryReset">重置</el-button>
        </el-form-item>
      </el-form>

      <el-table v-loading="historyLoading" :data="historyRecords" stripe>
        <el-table-column prop="customer_name" label="客户名称" min-width="180" />
        <el-table-column prop="assigned_from_name" label="原负责人" width="120" />
        <el-table-column prop="assigned_to_name" label="新负责人" width="120" />
        <el-table-column prop="assign_type" label="分配类型" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getTypeTag(row.assign_type)" size="small">
              {{ getTypeLabel(row.assign_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="reason" label="分配原因" min-width="200" show-overflow-tooltip />
        <el-table-column prop="created_at" label="分配时间" width="180" />
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="historyQuery.page"
          v-model:page-size="historyQuery.page_size"
          :page-sizes="[10, 20, 50]"
          :total="historyTotal"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="fetchHistory"
          @current-change="fetchHistory"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance } from 'element-plus'
import { Promotion } from '@element-plus/icons-vue'
import crmEnhancedApi, {
  type SalesUser,
  type AssignmentRecord,
  type CustomerWithTags,
} from '@/api/crm-enhanced'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'

const hasLoaded = createLazyLoader()

const assignMode = ref<'single' | 'batch'>('single')
const assignLoading = ref(false)
const historyLoading = ref(false)
const historyRecords = ref<AssignmentRecord[]>([])
const historyTotal = ref(0)
const salesUsers = ref<SalesUser[]>([])
const customerOptions = ref<CustomerWithTags[]>([])
const assignFormRef = ref<FormInstance>()

const assignForm = reactive({
  customer_ids: [] as number[],
  assign_to: undefined as number | undefined,
  reason: '',
})

const historyQuery = reactive({
  page: 1,
  page_size: 20,
  assign_type: '',
  assigned_to: undefined as number | undefined,
})

const getTypeLabel = (type: string) => {
  const labels: Record<string, string> = { manual: '手动', auto: '自动', batch: '批量' }
  return labels[type] || type
}

const getTypeTag = (type: string) => {
  const tags: Record<string, string> = { manual: 'primary', auto: 'success', batch: 'warning' }
  return tags[type] || ''
}

const fetchSalesUsers = async () => {
  try {
    const res = await crmEnhancedApi.getSalesUsers()
    salesUsers.value = res.data || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取业务员列表失败')
    salesUsers.value = []
  }
}

const fetchCustomerOptions = async () => {
  try {
    const res = await crmEnhancedApi.getCustomerList({ page: 1, page_size: 100 })
    customerOptions.value = res.data?.list || []
  } catch (error: any) {
    customerOptions.value = []
  }
}

const fetchHistory = async () => {
  historyLoading.value = true
  try {
    const res = await crmEnhancedApi.getAssignmentHistory(historyQuery)
    historyRecords.value = res.data?.list || []
    historyTotal.value = res.data?.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取分配历史失败')
    historyRecords.value = []
    historyTotal.value = 0
  } finally {
    historyLoading.value = false
  }
}

const handleAssign = async () => {
  if (!assignForm.customer_ids.length) {
    ElMessage.warning('请选择客户')
    return
  }
  if (!assignForm.assign_to) {
    ElMessage.warning('请选择业务员')
    return
  }

  assignLoading.value = true
  try {
    await crmEnhancedApi.assignCustomer({
      customer_ids: assignForm.customer_ids,
      assign_to: assignForm.assign_to,
      reason: assignForm.reason,
    })
    ElMessage.success('分配成功')
    resetAssignForm()
    fetchHistory()
  } catch (error: any) {
    ElMessage.error(error.message || '分配失败')
  } finally {
    assignLoading.value = false
  }
}

const resetAssignForm = () => {
  assignForm.customer_ids = []
  assignForm.assign_to = undefined
  assignForm.reason = ''
}

const handleHistoryReset = () => {
  historyQuery.assign_type = ''
  historyQuery.assigned_to = undefined
  historyQuery.page = 1
  fetchHistory()
}

const initPage = () => {
  loadIfNot('fetchSalesUsers', fetchSalesUsers, hasLoaded)
  loadIfNot('fetchCustomerOptions', fetchCustomerOptions, hasLoaded)
  loadIfNot('fetchHistory', fetchHistory, hasLoaded)
}

onMounted(() => {
  initPage()
})
</script>

<style scoped>
.assignment-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}
.header-left .page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}
.card-header {
  font-weight: 600;
}
.mt-20 {
  margin-top: 20px;
}
.history-filter {
  margin-bottom: 16px;
}
.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}

.user-stats {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.user-stat-item {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 12px;
  border-radius: 8px;
  background: #fafafa;
  transition: background 0.2s;
}
.user-stat-item:hover {
  background: #f0f2f5;
}
.user-stat-item.inactive {
  opacity: 0.6;
}
.user-info {
  display: flex;
  align-items: center;
  gap: 12px;
  flex: 1;
}
.user-detail {
  display: flex;
  flex-direction: column;
}
.user-name {
  font-weight: 600;
  color: #303133;
}
.user-dept {
  font-size: 12px;
  color: #909399;
}
.user-metrics {
  text-align: center;
}
.user-status {
  min-width: 60px;
}
</style>

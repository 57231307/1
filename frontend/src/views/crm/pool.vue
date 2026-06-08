<template>
  <div class="pool-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">公海客户池</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>CRM</el-breadcrumb-item>
          <el-breadcrumb-item>公海池</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input v-model="queryParams.keyword" placeholder="客户编码/名称" clearable />
        </el-form-item>
        <el-form-item label="客户类型">
          <el-select v-model="queryParams.customer_type" placeholder="选择类型" clearable>
            <el-option label="普通客户" value="normal" />
            <el-option label="VIP客户" value="vip" />
            <el-option label="批发客户" value="wholesale" />
          </el-select>
        </el-form-item>
        <el-form-item label="来源">
          <el-select v-model="queryParams.source" placeholder="选择来源" clearable>
            <el-option label="网站" value="website" />
            <el-option label="电话" value="phone" />
            <el-option label="展会" value="exhibition" />
            <el-option label="推荐" value="referral" />
          </el-select>
        </el-form-item>
        <el-form-item label="在池天数">
          <el-input-number
            v-model="queryParams.days_min"
            :min="0"
            placeholder="最小"
            style="width: 100px"
          />
          <span class="mx-2">-</span>
          <el-input-number
            v-model="queryParams.days_max"
            :min="0"
            placeholder="最大"
            style="width: 100px"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <div class="toolbar">
        <el-button type="primary" :disabled="!selectedRows.length" @click="handleBatchClaim">
          <el-icon><Select /></el-icon>
          批量领取
        </el-button>
        <span v-if="selectedRows.length" class="selected-count"
          >已选 {{ selectedRows.length }} 项</span
        >
      </div>

      <el-table
        v-loading="loading"
        :data="poolCustomers"
        stripe
        @selection-change="handleSelectionChange"
      >
        <el-table-column type="selection" width="50" />
        <el-table-column prop="customer_code" label="客户编码" width="120" fixed />
        <el-table-column prop="customer_name" label="客户名称" min-width="180" fixed />
        <el-table-column prop="contact_person" label="联系人" width="100" />
        <el-table-column prop="phone" label="电话" width="130" />
        <el-table-column prop="email" label="邮箱" width="180" show-overflow-tooltip />
        <el-table-column prop="customer_type" label="类型" width="100">
          <template #default="{ row }">
            <el-tag :type="getTypeTag(row.customer_type)" size="small">
              {{ getTypeLabel(row.customer_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="source" label="来源" width="100">
          <template #default="{ row }">{{ getSourceLabel(row.source) }}</template>
        </el-table-column>
        <el-table-column prop="days_in_pool" label="在池天数" width="100" align="center">
          <template #default="{ row }">
            <el-tag
              :type="row.days_in_pool > 30 ? 'danger' : row.days_in_pool > 15 ? 'warning' : 'info'"
              size="small"
            >
              {{ row.days_in_pool }}天
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="120" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleClaim(row as any)"
              >领取</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="queryParams.page"
          v-model:page-size="queryParams.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleQuery"
          @current-change="handleQuery"
        />
      </div>
    </el-card>

    <el-row :gutter="20" class="mt-20">
      <el-col :span="24">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>回收规则设置</span>
              <el-button type="primary" size="small" @click="handleAddRule">
                <el-icon><Plus /></el-icon>
                新增规则
              </el-button>
            </div>
          </template>

          <el-table v-loading="rulesLoading" :data="recycleRules" stripe>
            <el-table-column prop="name" label="规则名称" min-width="150" />
            <el-table-column prop="days_limit" label="天数限制" width="100" align="center">
              <template #default="{ row }">{{ row.days_limit }}天</template>
            </el-table-column>
            <el-table-column prop="follow_up_required" label="跟进要求" width="120" align="center">
              <template #default="{ row }">
                <el-tag :type="row.follow_up_required ? 'success' : 'info'" size="small">
                  {{ row.follow_up_required ? '是' : '否' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column
              prop="min_follow_up_count"
              label="最低跟进次数"
              width="120"
              align="center"
            />
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
                  {{ row.status === 'active' ? '启用' : '停用' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="150" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="handleEditRule(row as any)"
                  >编辑</el-button
                >
                <el-button type="danger" link size="small" @click="handleDeleteRule(row as any)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>
    </el-row>

    <!-- 回收规则对话框 -->
    <el-dialog
      v-model="ruleDialogVisible"
      :title="ruleDialogTitle"
      width="500px"
      :close-on-click-modal="false"
    >
      <el-form ref="ruleFormRef" :model="ruleForm" label-width="120px">
        <el-form-item label="规则名称" prop="name">
          <el-input v-model="ruleForm.name" placeholder="请输入规则名称" />
        </el-form-item>
        <el-form-item label="天数限制" prop="days_limit">
          <el-input-number v-model="ruleForm.days_limit" :min="1" :max="365" style="width: 100%" />
        </el-form-item>
        <el-form-item label="需跟进" prop="follow_up_required">
          <el-switch v-model="ruleForm.follow_up_required" />
        </el-form-item>
        <el-form-item
          v-if="ruleForm.follow_up_required"
          label="最低跟进次数"
          prop="min_follow_up_count"
        >
          <el-input-number
            v-model="ruleForm.min_follow_up_count"
            :min="1"
            :max="100"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-radio-group v-model="ruleForm.status">
            <el-radio value="active">启用</el-radio>
            <el-radio value="inactive">停用</el-radio>
          </el-radio-group>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="ruleDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="ruleSubmitLoading" @click="submitRuleForm"
          >保存</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance } from 'element-plus'
import { Plus, Select } from '@element-plus/icons-vue'
import crmEnhancedApi, { type PoolCustomer, type RecycleRule } from '@/api/crm-enhanced'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'

const hasLoaded = createLazyLoader()

const loading = ref(false)
const rulesLoading = ref(false)
const ruleSubmitLoading = ref(false)
const poolCustomers = ref<PoolCustomer[]>([])
const recycleRules = ref<RecycleRule[]>([])
const total = ref(0)
const selectedRows = ref<PoolCustomer[]>([])
const ruleDialogVisible = ref(false)
const ruleDialogTitle = ref('新增回收规则')
const isEditRule = ref(false)
const ruleFormRef = ref<FormInstance>()

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  customer_type: '',
  source: '',
  days_min: undefined as number | undefined,
  days_max: undefined as number | undefined,
})

const ruleForm = reactive({
  id: undefined as number | undefined,
  name: '',
  days_limit: 30,
  follow_up_required: true,
  min_follow_up_count: 3,
  status: 'active' as 'active' | 'inactive',
})

const getTypeLabel = (type: string) => {
  const labels: Record<string, string> = {
    normal: '普通客户',
    vip: 'VIP客户',
    wholesale: '批发客户',
  }
  return labels[type] || type
}

const getTypeTag = (type: string) => {
  const tags: Record<string, string> = { normal: '', vip: 'warning', wholesale: 'success' }
  return tags[type] || ''
}

const getSourceLabel = (source: string) => {
  const labels: Record<string, string> = {
    website: '网站',
    phone: '电话',
    exhibition: '展会',
    referral: '推荐',
  }
  return labels[source] || source
}

const fetchPoolList = async () => {
  loading.value = true
  try {
    const res = await crmEnhancedApi.getPoolList(queryParams)
    poolCustomers.value = res.data?.list || []
    total.value = res.data?.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取公海客户列表失败')
    poolCustomers.value = []
    total.value = 0
  } finally {
    loading.value = false
  }
}

const fetchRecycleRules = async () => {
  rulesLoading.value = true
  try {
    const res = await crmEnhancedApi.getRecycleRules()
    recycleRules.value = res.data || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取回收规则失败')
    recycleRules.value = []
  } finally {
    rulesLoading.value = false
  }
}

const handleQuery = () => {
  queryParams.page = 1
  fetchPoolList()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.customer_type = ''
  queryParams.source = ''
  queryParams.days_min = undefined
  queryParams.days_max = undefined
  handleQuery()
}

const handleSelectionChange = (rows: PoolCustomer[]) => {
  selectedRows.value = rows
}

const handleClaim = async (row: PoolCustomer) => {
  try {
    await ElMessageBox.confirm(`确定领取客户 "${row.customer_name}" 吗？`, '确认领取', {
      type: 'info',
    })
    await crmEnhancedApi.claimFromPool(row.id)
    ElMessage.success('领取成功')
    fetchPoolList()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '领取失败')
    }
  }
}

const handleBatchClaim = async () => {
  if (!selectedRows.value.length) return
  try {
    await ElMessageBox.confirm(
      `确定领取选中的 ${selectedRows.value.length} 个客户吗？`,
      '批量领取确认',
      { type: 'info' }
    )
    const ids = selectedRows.value.map((r) => r.id)
    await crmEnhancedApi.batchClaimFromPool(ids)
    ElMessage.success('批量领取成功')
    selectedRows.value = []
    fetchPoolList()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '批量领取失败')
    }
  }
}

const handleAddRule = () => {
  isEditRule.value = false
  ruleDialogTitle.value = '新增回收规则'
  ruleForm.id = undefined
  ruleForm.name = ''
  ruleForm.days_limit = 30
  ruleForm.follow_up_required = true
  ruleForm.min_follow_up_count = 3
  ruleForm.status = 'active'
  ruleDialogVisible.value = true
}

const handleEditRule = (row: RecycleRule) => {
  isEditRule.value = true
  ruleDialogTitle.value = '编辑回收规则'
  ruleForm.id = row.id
  ruleForm.name = row.name
  ruleForm.days_limit = row.days_limit
  ruleForm.follow_up_required = row.follow_up_required
  ruleForm.min_follow_up_count = row.min_follow_up_count
  ruleForm.status = row.status
  ruleDialogVisible.value = true
}

const submitRuleForm = async () => {
  ruleSubmitLoading.value = true
  try {
    if (isEditRule.value && ruleForm.id !== undefined) {
      await crmEnhancedApi.updateRecycleRule(ruleForm.id, ruleForm)
      ElMessage.success('更新成功')
    } else {
      await crmEnhancedApi.createRecycleRule(ruleForm)
      ElMessage.success('创建成功')
    }
    ruleDialogVisible.value = false
    fetchRecycleRules()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    ruleSubmitLoading.value = false
  }
}

const handleDeleteRule = async (row: RecycleRule) => {
  try {
    await ElMessageBox.confirm(`确定删除回收规则 "${row.name}" 吗？`, '删除确认', {
      type: 'warning',
    })
    await crmEnhancedApi.deleteRecycleRule(row.id)
    ElMessage.success('删除成功')
    fetchRecycleRules()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '删除失败')
    }
  }
}

const initPage = () => {
  loadIfNot('fetchPoolList', fetchPoolList, hasLoaded)
  loadIfNot('fetchRecycleRules', fetchRecycleRules, hasLoaded)
}

onMounted(() => {
  initPage()
})
</script>

<style scoped>
.pool-page {
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
.filter-card {
  margin-bottom: 20px;
}
.table-card {
  margin-bottom: 20px;
}
.toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}
.selected-count {
  color: #909399;
  font-size: 14px;
}
.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 600;
}
.mt-20 {
  margin-top: 20px;
}
.mx-2 {
  margin: 0 8px;
}
</style>

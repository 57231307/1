<!--
  crm/opportunities/index.vue - 商机管理主入口
  拆分：tabs/OpportunityFormTab.vue - 新建/编辑商机对话框 / tabs/OpportunityFollowTab.vue - 跟进记录对话框
  本主入口承担：页面布局 + 列表数据 + 公共样式。
-->
<template>
  <div class="crm-opportunities-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ t('crmOpportunities.title') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{ t('crmOpportunities.breadcrumb.home') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('crmOpportunities.breadcrumb.crm') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ t('crmOpportunities.breadcrumb.opportunities') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="openCreateDialog">
          <el-icon><Plus /></el-icon>
          {{ t('crmOpportunities.create') }}
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          {{ t('crmOpportunities.export') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form" :aria-label="t('crmOpportunities.filter.ariaLabel')">
        <el-form-item :label="t('crmOpportunities.filter.keyword')">
          <el-input
            v-model="queryParams.keyword"
            :placeholder="t('crmOpportunities.filter.keywordPlaceholder')"
            clearable
            @clear="handleQuery"
            @keyup.enter="handleQuery"
          />
        </el-form-item>
        <el-form-item :label="t('crmOpportunities.filter.stage')">
          <el-select
            v-model="queryParams.opportunity_stage"
            :placeholder="t('crmOpportunities.filter.stagePlaceholder')"
            clearable
            @change="handleQuery"
          >
            <el-option :label="t('crmOpportunities.stage.initial')" value="INITIAL" />
            <el-option :label="t('crmOpportunities.stage.requirement')" value="REQUIREMENT" />
            <el-option :label="t('crmOpportunities.stage.proposal')" value="PROPOSAL" />
            <el-option :label="t('crmOpportunities.stage.negotiation')" value="NEGOTIATION" />
            <el-option :label="t('crmOpportunities.stage.won')" value="WON" />
            <el-option :label="t('crmOpportunities.stage.lost')" value="LOST" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('crmOpportunities.filter.owner')">
          <el-select
            v-model="queryParams.owner_id"
            :placeholder="t('crmOpportunities.filter.ownerPlaceholder')"
            clearable
            filterable
            @change="handleQuery"
          >
            <el-option v-for="u in users" :key="u.id" :label="u.real_name" :value="u.id" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('crmOpportunities.filter.priority')">
          <el-select
            v-model="queryParams.priority"
            :placeholder="t('crmOpportunities.filter.priorityPlaceholder')"
            clearable
            @change="handleQuery"
          >
            <el-option :label="t('crmOpportunities.priority.low')" value="LOW" />
            <el-option :label="t('crmOpportunities.priority.medium')" value="MEDIUM" />
            <el-option :label="t('crmOpportunities.priority.high')" value="HIGH" />
            <el-option :label="t('crmOpportunities.priority.urgent')" value="URGENT" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            {{ t('crmOpportunities.filter.query') }}
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            {{ t('crmOpportunities.filter.reset') }}
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="opportunityList" border stripe :aria-label="t('crmOpportunities.table.ariaLabel')">
        <el-table-column type="index" :label="t('crmOpportunities.table.index')" width="60" align="center" />
        <el-table-column prop="opportunity_no" :label="t('crmOpportunities.table.opportunityNo')" width="120" show-overflow-tooltip />
        <el-table-column
          prop="opportunity_name"
          :label="t('crmOpportunities.table.opportunityName')"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column prop="customer_name" :label="t('crmOpportunities.table.customer')" width="150" show-overflow-tooltip />
        <el-table-column prop="estimated_amount" :label="t('crmOpportunities.table.estimatedAmount')" width="120" align="right">
          <template #default="{ row }">
            {{ formatCurrency(row.estimated_amount) }}
          </template>
        </el-table-column>
        <el-table-column prop="win_probability" :label="t('crmOpportunities.table.winProbability')" width="100" align="center">
          <template #default="{ row }"> {{ row.win_probability }}% </template>
        </el-table-column>
        <el-table-column prop="opportunity_stage" :label="t('crmOpportunities.table.stage')" width="120" align="center">
          <template #default="{ row }">
            <el-tag :type="getStageType(row.opportunity_stage)">{{
              getStageLabel(row.opportunity_stage)
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="expected_close_date" :label="t('crmOpportunities.table.expectedCloseDate')" width="120" align="center" />
        <el-table-column prop="owner_name" :label="t('crmOpportunities.table.owner')" width="100" show-overflow-tooltip />
        <el-table-column prop="last_follow_up_date" :label="t('crmOpportunities.table.lastFollowUp')" width="120" align="center" />
        <el-table-column :label="t('crmOpportunities.table.operation')" width="250" align="center" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">{{ t('crmOpportunities.table.view') }}</el-button>
            <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
            <el-button
              v-if="row.opportunity_stage !== 'WON' && row.opportunity_stage !== 'LOST'"
              v-permission="'crm_opportunity:update'"
              type="primary"
              link
              size="small"
              @click="openEditDialog(row)"
              >{{ t('crmOpportunities.table.edit') }}</el-button
            >
            <el-button
              v-if="row.opportunity_stage !== 'WON' && row.opportunity_stage !== 'LOST'"
              type="warning"
              link
              size="small"
              @click="openFollowDialog(row)"
              >{{ t('crmOpportunities.table.follow') }}</el-button
            >
            <el-button
              v-if="row.opportunity_stage === 'NEGOTIATION'"
              type="success"
              link
              size="small"
              @click="handleWin(row)"
              >{{ t('crmOpportunities.table.win') }}</el-button
            >
            <el-button
              v-if="row.opportunity_stage !== 'WON' && row.opportunity_stage !== 'LOST'"
              type="danger"
              link
              size="small"
              @click="handleLost(row)"
              >{{ t('crmOpportunities.table.lost') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="t('crmOpportunities.table.paginationAriaLabel')"
          @size-change="handleSizeChange"
          @current-change="handleCurrentChange"
        />
      </div>
    </el-card>

    <OpportunityFormTab
      v-model="formDialogVisible"
      :title="formDialogTitle"
      :row-data="currentRow"
      :users="users"
      :customers="customers"
      @submitted="handleFormSubmitted"
    />

    <OpportunityFollowTab
      v-model="followDialogVisible"
      :opportunity-id="currentFollowId"
      @submitted="getList"
    />

    <!-- 商机详情对话框（批次 95 P3-19 修复：参考 SalesPriceView.vue 的 el-descriptions 模式） -->
    <el-dialog v-model="viewDialogVisible" :title="t('crmOpportunities.viewDialog.title')" width="640px" :aria-label="t('crmOpportunities.viewDialog.ariaLabel')">
      <el-descriptions v-if="viewData" :column="2" border>
        <el-descriptions-item :label="t('crmOpportunities.viewDialog.opportunityNo')">{{
          viewData.opportunity_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('crmOpportunities.viewDialog.opportunityName')">{{
          viewData.opportunity_name || viewData.name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('crmOpportunities.viewDialog.customer')">{{
          viewData.customer_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('crmOpportunities.viewDialog.owner')">{{
          viewData.owner_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('crmOpportunities.viewDialog.estimatedAmount')">{{
          formatCurrency(viewData.estimated_amount)
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('crmOpportunities.viewDialog.winProbability')"
          >{{ viewData.win_probability ?? viewData.probability ?? 0 }}%</el-descriptions-item
        >
        <el-descriptions-item :label="t('crmOpportunities.viewDialog.stage')">
          <el-tag :type="getStageType(viewData.opportunity_stage || '')">{{
            getStageLabel(viewData.opportunity_stage || '')
          }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('crmOpportunities.viewDialog.priority')">{{
          viewData.priority || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('crmOpportunities.viewDialog.expectedCloseDate')">{{
          viewData.expected_close_date || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('crmOpportunities.viewDialog.lastFollowUp')">{{
          viewData.last_follow_up_date || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('crmOpportunities.viewDialog.createdBy')">{{
          viewData.created_by_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('crmOpportunities.viewDialog.createdAt')">{{
          viewData.created_at || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('crmOpportunities.viewDialog.description')" :span="2">{{
          viewData.description || '-'
        }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Download, Search, Refresh } from '@element-plus/icons-vue'
import {
  updateOpportunity,
  exportOpportunities,
  type Opportunity,
} from '@/api/crm'
import { getUserList, type User } from '@/api/user'
import { getCustomerList, type Customer } from '@/api/customer'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'
import OpportunityFormTab from './tabs/OpportunityFormTab.vue'
import OpportunityFollowTab from './tabs/OpportunityFollowTab.vue'

const { t } = useI18n({ useScope: 'global' })

const hasLoaded = createLazyLoader()

// 实际接口字段名与类型定义不完全一致，扩展包含 UI 展示所需字段
interface OpportunityRow extends Opportunity {
  opportunity_name?: string
  // v11 批次 141 修复：opportunity_stage 已从 Opportunity 继承（字面量联合类型），不再重复声明
  owner_name?: string
  last_follow_up_date?: string
  priority?: string
  // 批次 95 P3-19 修复：补充列表/详情展示所需字段（后端返回，类型定义缺失）
  win_probability?: number
}

const queryParams = reactive({
  keyword: '',
  opportunity_stage: '',
  owner_id: '',
  priority: '',
})

// 批次 269：接入 useTableApi，消除手写分页重复
const {
  data: opportunityList,
  loading,
  page,
  pageSize,
  total,
  refresh: getList,
  setQueryParam,
} = useTableApi<OpportunityRow>({
  url: '/crm/opportunities',
  onError: (e: unknown) => logger.warn(t('crmOpportunities.message.loadFailed'), String(e)),
})

const users = ref<User[]>([])
const customers = ref<Customer[]>([])

const formDialogVisible = ref(false)
const formDialogTitle = ref('')
const currentRow = ref<OpportunityRow | null>(null)
const followDialogVisible = ref(false)
const currentFollowId = ref<number | null>(null)

// 查看详情对话框状态（批次 95 P3-19 修复）
const viewDialogVisible = ref(false)
const viewData = ref<OpportunityRow | null>(null)

const fetchUsers = async () => {
  try {
    const res = await getUserList()
    users.value = res.data?.list || []
  } catch (error) {
    users.value = []
  }
}

const fetchCustomers = async () => {
  try {
    const res = await getCustomerList()
    customers.value = res.data?.list || []
  } catch (error) {
    customers.value = []
  }
}

const handleQuery = () => {
  setQueryParam('keyword', queryParams.keyword || undefined)
  setQueryParam('opportunity_stage', queryParams.opportunity_stage || undefined)
  setQueryParam('owner_id', queryParams.owner_id || undefined)
  setQueryParam('priority', queryParams.priority || undefined)
  page.value = 1
  getList()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.opportunity_stage = ''
  queryParams.owner_id = ''
  queryParams.priority = ''
  handleQuery()
}

const openCreateDialog = () => {
  currentRow.value = null
  formDialogTitle.value = t('crmOpportunities.dialog.createTitle')
  formDialogVisible.value = true
}

const openEditDialog = (row: OpportunityRow) => {
  currentRow.value = row
  formDialogTitle.value = t('crmOpportunities.dialog.editTitle')
  formDialogVisible.value = true
}

const openFollowDialog = (row: OpportunityRow) => {
  currentFollowId.value = row.id
  followDialogVisible.value = true
}

const handleFormSubmitted = () => {
  formDialogVisible.value = false
  getList()
}

// 查看详情（批次 95 P3-19 修复：打开详情对话框展示商机完整信息）
const handleView = (row: OpportunityRow) => {
  viewData.value = row
  viewDialogVisible.value = true
}

const handleWin = async (row: OpportunityRow) => {
  try {
    await ElMessageBox.confirm(t('crmOpportunities.message.winConfirm', { name: row.opportunity_name }), t('crmOpportunities.message.tip'), {
      type: 'warning',
    })
    // v11 批次 141 修复：原占位假成功，现接入真实状态变更 API
    // 后端 UpdateOpportunityRequest 字段名为 opportunity_stage，阶段值大写
    await updateOpportunity(row.id, { opportunity_stage: 'CLOSED_WON' })
    ElMessage.success(t('crmOpportunities.message.winSuccess'))
    getList()
  } catch (error) {
    if (error !== 'cancel') {
      logger.warn(t('crmOpportunities.message.winFailed'), (error as Error).message)
      ElMessage.error(t('crmOpportunities.message.winFailed'))
    }
  }
}

const handleLost = async (row: OpportunityRow) => {
  try {
    await ElMessageBox.confirm(t('crmOpportunities.message.lostConfirm', { name: row.opportunity_name }), t('crmOpportunities.message.tip'), {
      type: 'warning',
    })
    // v11 批次 141 修复：原占位假成功，现接入真实状态变更 API
    // 后端 UpdateOpportunityRequest 字段名为 opportunity_stage，阶段值大写
    await updateOpportunity(row.id, { opportunity_stage: 'CLOSED_LOST' })
    ElMessage.success(t('crmOpportunities.message.lostSuccess'))
    getList()
  } catch (error) {
    if (error !== 'cancel') {
      logger.warn(t('crmOpportunities.message.lostFailed'), (error as Error).message)
      ElMessage.error(t('crmOpportunities.message.lostFailed'))
    }
  }
}

// v11 批次 141 修复：原占位假成功，现接入真实导出 API 并触发浏览器下载
const handleExport = async () => {
  try {
    const blob = await exportOpportunities(queryParams)
    const url = window.URL.createObjectURL(new Blob([blob]))
    const link = document.createElement('a')
    link.href = url
    link.setAttribute('download', `${t('crmOpportunities.message.exportFilename')}_${new Date().toISOString().split('T')[0]}.xlsx`)
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    window.URL.revokeObjectURL(url)
    ElMessage.success(t('crmOpportunities.message.exportSuccess'))
  } catch (error) {
    logger.error(t('crmOpportunities.message.exportFailed'), error)
    ElMessage.error(t('crmOpportunities.message.exportFailed'))
  }
}

const handleSizeChange = (val: number) => {
  pageSize.value = val
  page.value = 1
}

const handleCurrentChange = (val: number) => {
  page.value = val
}

const formatCurrency = (value: number) => {
  return value ? `¥${value.toFixed(2)}` : '¥0.00'
}

const getStageType = (stage: string) => {
  const typeMap: Record<string, string> = {
    INITIAL: 'info',
    REQUIREMENT: '',
    PROPOSAL: 'warning',
    NEGOTIATION: 'primary',
    WON: 'success',
    LOST: 'danger',
  }
  return typeMap[stage] || 'info'
}

const getStageLabel = (stage: string) => {
  const labelMap: Record<string, string> = {
    INITIAL: t('crmOpportunities.stage.initial'),
    REQUIREMENT: t('crmOpportunities.stage.requirement'),
    PROPOSAL: t('crmOpportunities.stage.proposal'),
    NEGOTIATION: t('crmOpportunities.stage.negotiation'),
    WON: t('crmOpportunities.stage.won'),
    LOST: t('crmOpportunities.stage.lost'),
  }
  return labelMap[stage] || stage
}

onMounted(() => {
  // useTableApi 已自动初始加载，此处仅懒加载用户/客户下拉数据
  loadIfNot('users', fetchUsers, hasLoaded)
  loadIfNot('customers', fetchCustomers, hasLoaded)
})
</script>

<style scoped>
.crm-opportunities-page {
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

.filter-card {
  margin-bottom: 20px;
}

.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.table-card {
  margin-bottom: 20px;
}

.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
}
</style>

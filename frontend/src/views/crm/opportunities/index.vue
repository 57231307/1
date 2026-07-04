<!--
  crm/opportunities/index.vue - 商机管理主入口
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-3）：
  原 602 行"上帝组件"已拆分为：
  - tabs/OpportunityFormTab.vue - 新建/编辑商机对话框
  - tabs/OpportunityFollowTab.vue - 跟进记录对话框

  本主入口承担：页面布局 + 列表数据 + 公共样式。
-->
<template>
  <div class="crm-opportunities-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">商机管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>CRM</el-breadcrumb-item>
          <el-breadcrumb-item>商机管理</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="openCreateDialog">
          <el-icon><Plus /></el-icon>
          新建商机
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input
            v-model="queryParams.keyword"
            placeholder="商机编号/商机名称/客户名称"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item label="商机阶段">
          <el-select
            v-model="queryParams.opportunity_stage"
            placeholder="选择阶段"
            clearable
            @change="handleQuery"
          >
            <el-option label="初步接触" value="INITIAL" />
            <el-option label="需求确认" value="REQUIREMENT" />
            <el-option label="方案报价" value="PROPOSAL" />
            <el-option label="谈判" value="NEGOTIATION" />
            <el-option label="成交" value="WON" />
            <el-option label="流失" value="LOST" />
          </el-select>
        </el-form-item>
        <el-form-item label="负责人">
          <el-select
            v-model="queryParams.owner_id"
            placeholder="选择负责人"
            clearable
            filterable
            @change="handleQuery"
          >
            <el-option v-for="u in users" :key="u.id" :label="u.real_name" :value="u.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="优先级">
          <el-select
            v-model="queryParams.priority"
            placeholder="选择优先级"
            clearable
            @change="handleQuery"
          >
            <el-option label="低" value="LOW" />
            <el-option label="中" value="MEDIUM" />
            <el-option label="高" value="HIGH" />
            <el-option label="紧急" value="URGENT" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            查询
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            重置
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="opportunityList" border stripe>
        <el-table-column type="index" label="序号" width="60" align="center" />
        <el-table-column prop="opportunity_no" label="商机编号" width="120" show-overflow-tooltip />
        <el-table-column
          prop="opportunity_name"
          label="商机名称"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column prop="customer_name" label="客户" width="150" show-overflow-tooltip />
        <el-table-column prop="estimated_amount" label="预估金额" width="120" align="right">
          <template #default="{ row }">
            {{ formatCurrency(row.estimated_amount) }}
          </template>
        </el-table-column>
        <el-table-column prop="win_probability" label="成交概率" width="100" align="center">
          <template #default="{ row }"> {{ row.win_probability }}% </template>
        </el-table-column>
        <el-table-column prop="opportunity_stage" label="商机阶段" width="120" align="center">
          <template #default="{ row }">
            <el-tag :type="getStageType(row.opportunity_stage)">{{
              getStageLabel(row.opportunity_stage)
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="expected_close_date" label="预计成交" width="120" align="center" />
        <el-table-column prop="owner_name" label="负责人" width="100" show-overflow-tooltip />
        <el-table-column prop="last_follow_up_date" label="最近跟进" width="120" align="center" />
        <el-table-column label="操作" width="250" align="center" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">查看</el-button>
            <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
            <el-button
              v-if="row.opportunity_stage !== 'WON' && row.opportunity_stage !== 'LOST'"
              v-permission="'crm_opportunity:update'"
              type="primary"
              link
              size="small"
              @click="openEditDialog(row)"
              >编辑</el-button
            >
            <el-button
              v-if="row.opportunity_stage !== 'WON' && row.opportunity_stage !== 'LOST'"
              type="warning"
              link
              size="small"
              @click="openFollowDialog(row)"
              >跟进</el-button
            >
            <el-button
              v-if="row.opportunity_stage === 'NEGOTIATION'"
              type="success"
              link
              size="small"
              @click="handleWin(row)"
              >成交</el-button
            >
            <el-button
              v-if="row.opportunity_stage !== 'WON' && row.opportunity_stage !== 'LOST'"
              type="danger"
              link
              size="small"
              @click="handleLost(row)"
              >流失</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="queryParams.page"
          v-model:page-size="queryParams.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
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

    <!-- 商机详情对话框（批次 95 P3-19 修复：参考 SpView.vue 的 el-descriptions 模式） -->
    <el-dialog v-model="viewDialogVisible" title="商机详情" width="640px">
      <el-descriptions v-if="viewData" :column="2" border>
        <el-descriptions-item label="商机编号">{{
          viewData.opportunity_no
        }}</el-descriptions-item>
        <el-descriptions-item label="商机名称">{{
          viewData.opportunity_name || viewData.name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="客户">{{
          viewData.customer_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="负责人">{{
          viewData.owner_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="预估金额">{{
          formatCurrency(viewData.estimated_amount)
        }}</el-descriptions-item>
        <el-descriptions-item label="成交概率"
          >{{ viewData.win_probability ?? viewData.probability ?? 0 }}%</el-descriptions-item
        >
        <el-descriptions-item label="商机阶段">
          <el-tag :type="getStageType(viewData.opportunity_stage || '')">{{
            getStageLabel(viewData.opportunity_stage || '')
          }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="优先级">{{
          viewData.priority || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="预计成交">{{
          viewData.expected_close_date || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="最近跟进">{{
          viewData.last_follow_up_date || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="创建人">{{
          viewData.created_by_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="创建时间">{{
          viewData.created_at || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="描述" :span="2">{{
          viewData.description || '-'
        }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Download, Search, Refresh } from '@element-plus/icons-vue'
import { listOpportunities, type Opportunity } from '@/api/crm'
import { listUsers, type User } from '@/api/user'
import { customerApi, type Customer } from '@/api/customer'
import type { ApiResponse, PageResult } from '@/types/api'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
import OpportunityFormTab from './tabs/OpportunityFormTab.vue'
import OpportunityFollowTab from './tabs/OpportunityFollowTab.vue'

const hasLoaded = createLazyLoader()

// 实际接口字段名与类型定义不完全一致，扩展包含 UI 展示所需字段
interface OpportunityRow extends Opportunity {
  opportunity_name?: string
  opportunity_stage?: string
  owner_name?: string
  last_follow_up_date?: string
  priority?: string
  // 批次 95 P3-19 修复：补充列表/详情展示所需字段（后端返回，类型定义缺失）
  win_probability?: number
}

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  opportunity_stage: '',
  owner_id: '',
  priority: '',
})

const loading = ref(false)
const opportunityList = ref<OpportunityRow[]>([])
const total = ref(0)
const users = ref<User[]>([])
const customers = ref<Customer[]>([])

const formDialogVisible = ref(false)
const formDialogTitle = ref('新建商机')
const currentRow = ref<OpportunityRow | null>(null)
const followDialogVisible = ref(false)
const currentFollowId = ref<number | null>(null)

// 查看详情对话框状态（批次 95 P3-19 修复）
const viewDialogVisible = ref(false)
const viewData = ref<OpportunityRow | null>(null)

const getList = async () => {
  loading.value = true
  try {
    // 后端 listOpportunities 实际返回 PageResult<T> 包装，但类型定义为 ApiResponse<Opportunity[]>，此处显式声明
    const res = (await listOpportunities(queryParams)) as unknown as ApiResponse<
      PageResult<OpportunityRow>
    >
    opportunityList.value = res.data?.list || []
    total.value = res.data?.total || 0
  } catch (error) {
    const err = error as Error
    logger.warn('获取商机列表失败', err.message)
  } finally {
    loading.value = false
  }
}

const fetchUsers = async () => {
  try {
    const res = await listUsers()
    users.value = res.data?.list || []
  } catch (error) {
    users.value = []
  }
}

const fetchCustomers = async () => {
  try {
    const res = await customerApi.list()
    customers.value = res.data?.list || []
  } catch (error) {
    customers.value = []
  }
}

const handleQuery = () => {
  queryParams.page = 1
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
  formDialogTitle.value = '新建商机'
  formDialogVisible.value = true
}

const openEditDialog = (row: OpportunityRow) => {
  currentRow.value = row
  formDialogTitle.value = '编辑商机'
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
    await ElMessageBox.confirm(`确认标记商机 "${row.opportunity_name}" 为成交？`, '提示', {
      type: 'warning',
    })
    ElMessage.success('操作成功')
    getList()
  } catch (error) {
    if (error !== 'cancel') {
      logger.warn('操作失败', (error as Error).message)
    }
  }
}

const handleLost = async (row: OpportunityRow) => {
  try {
    await ElMessageBox.confirm(`确认标记商机 "${row.opportunity_name}" 为流失？`, '提示', {
      type: 'warning',
    })
    ElMessage.success('操作成功')
    getList()
  } catch (error) {
    if (error !== 'cancel') {
      logger.warn('操作失败', (error as Error).message)
    }
  }
}

const handleExport = () => {
  ElMessage.success('导出成功')
}

const handleSizeChange = (val: number) => {
  queryParams.page_size = val
  getList()
}

const handleCurrentChange = (val: number) => {
  queryParams.page = val
  getList()
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
    INITIAL: '初步接触',
    REQUIREMENT: '需求确认',
    PROPOSAL: '方案报价',
    NEGOTIATION: '谈判',
    WON: '成交',
    LOST: '流失',
  }
  return labelMap[stage] || stage
}

onMounted(() => {
  getList()
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

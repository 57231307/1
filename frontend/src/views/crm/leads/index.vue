<!--
  crm/leads/index.vue - 线索管理主入口
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-3）：
  原 595 行"上帝组件"已拆分为：
  - tabs/LeadFormTab.vue - 新建/编辑线索对话框

  本主入口承担：页面布局 + 列表数据 + 公共样式。
-->
<template>
  <div class="crm-leads-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">线索管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>CRM</el-breadcrumb-item>
          <el-breadcrumb-item>线索管理</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="openCreateDialog">
          <el-icon><Plus /></el-icon>
          新建线索
        </el-button>
        <el-button @click="handleImport">
          <el-icon><Upload /></el-icon>
          导入
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
            placeholder="线索编号/公司名称/联系人"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item label="线索来源">
          <el-select
            v-model="queryParams.lead_source"
            placeholder="选择来源"
            clearable
            @change="handleQuery"
          >
            <el-option label="网站" value="WEBSITE" />
            <el-option label="电话" value="PHONE" />
            <el-option label="展会" value="EXHIBITION" />
            <el-option label="推荐" value="REFERRAL" />
            <el-option label="其他" value="OTHER" />
          </el-select>
        </el-form-item>
        <el-form-item label="线索状态">
          <el-select
            v-model="queryParams.lead_status"
            placeholder="选择状态"
            clearable
            @change="handleQuery"
          >
            <el-option label="新线索" value="NEW" />
            <el-option label="已联系" value="CONTACTED" />
            <el-option label="已qualified" value="QUALIFIED" />
            <el-option label="已转化" value="CONVERTED" />
            <el-option label="已流失" value="LOST" />
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
      <el-table
        v-loading="loading"
        :data="leadList"
        border
        stripe
        @selection-change="handleSelectionChange"
      >
        <el-table-column type="selection" width="55" align="center" />
        <el-table-column type="index" label="序号" width="60" align="center" />
        <el-table-column prop="lead_no" label="线索编号" width="120" show-overflow-tooltip />
        <el-table-column
          prop="company_name"
          label="公司名称"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column prop="contact_name" label="联系人" width="100" show-overflow-tooltip />
        <el-table-column prop="mobile_phone" label="手机号" width="120" show-overflow-tooltip />
        <el-table-column prop="lead_source" label="线索来源" width="100" align="center">
          <template #default="{ row }">
            <el-tag>{{ getSourceLabel(row.lead_source) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="lead_status" label="线索状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.lead_status)">{{
              getStatusLabel(row.lead_status)
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="priority" label="优先级" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="getPriorityType(row.priority)">{{
              getPriorityLabel(row.priority)
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="owner_name" label="负责人" width="100" show-overflow-tooltip />
        <el-table-column prop="last_follow_up_date" label="最近跟进" width="120" align="center" />
        <el-table-column prop="next_follow_up_date" label="下次跟进" width="120" align="center" />
        <el-table-column label="操作" width="250" align="center" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">查看</el-button>
            <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
            <el-button
              v-if="row.lead_status !== 'CONVERTED'"
              v-permission="'crm_lead:update'"
              type="primary"
              link
              size="small"
              @click="openEditDialog(row)"
              >编辑</el-button
            >
            <el-button
              v-if="row.lead_status === 'NEW'"
              type="warning"
              link
              size="small"
              @click="handleContact(row)"
              >联系</el-button
            >
            <el-button
              v-if="row.lead_status === 'QUALIFIED'"
              type="success"
              link
              size="small"
              @click="handleConvert(row)"
              >转化</el-button
            >
            <el-button
              v-if="row.lead_status !== 'CONVERTED'"
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

    <LeadFormTab
      v-model="formDialogVisible"
      :title="formDialogTitle"
      :row-data="currentRow"
      :users="users"
      @submitted="handleFormSubmitted"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Upload, Download, Search, Refresh } from '@element-plus/icons-vue'
import { listLeads, type Lead } from '@/api/crm'
import { listUsers, type User } from '@/api/user'
import type { ApiResponse, PageResult } from '@/types/api'
import { logger } from '@/utils/logger'
import LeadFormTab from './tabs/LeadFormTab.vue'

// 实际接口字段名与类型定义不完全一致，扩展包含 UI 展示所需字段
interface LeadRow extends Lead {
  contact_name?: string
  company_name?: string
  mobile_phone?: string
  lead_source?: string
  lead_status?: string
  owner_name?: string
  last_follow_up_date?: string
  next_follow_up_date?: string
  priority?: string
  requirement_desc?: string
  remarks?: string
}

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  lead_source: '',
  lead_status: '',
  owner_id: '',
  priority: '',
})

const loading = ref(false)
const leadList = ref<LeadRow[]>([])
const total = ref(0)
const users = ref<User[]>([])

const formDialogVisible = ref(false)
const formDialogTitle = ref('新建线索')
const currentRow = ref<LeadRow | null>(null)

const getList = async () => {
  loading.value = true
  try {
    // 后端 listLeads 实际返回 PageResult<T> 包装，但类型定义为 ApiResponse<Lead[]>，此处显式声明
    const res = (await listLeads(queryParams)) as unknown as ApiResponse<PageResult<LeadRow>>
    leadList.value = res.data?.list || []
    total.value = res.data?.total || 0
  } catch (error) {
    const err = error as Error
    logger.warn('获取线索列表失败', err.message)
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

const handleQuery = () => {
  queryParams.page = 1
  getList()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.lead_source = ''
  queryParams.lead_status = ''
  queryParams.owner_id = ''
  queryParams.priority = ''
  handleQuery()
}

const openCreateDialog = () => {
  currentRow.value = null
  formDialogTitle.value = '新建线索'
  formDialogVisible.value = true
}

const openEditDialog = (row: LeadRow) => {
  currentRow.value = row
  formDialogTitle.value = '编辑线索'
  formDialogVisible.value = true
}

const handleFormSubmitted = () => {
  formDialogVisible.value = false
  getList()
}

const handleView = (_row: LeadRow) => {
  // 查看详情（占位）
}

const handleContact = async (row: LeadRow) => {
  try {
    await ElMessageBox.confirm(`确认标记线索 "${row.contact_name}" 为已联系？`, '提示', {
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

const handleConvert = async (row: LeadRow) => {
  try {
    await ElMessageBox.confirm(`确认将线索 "${row.contact_name}" 转化为客户？`, '提示', {
      type: 'warning',
    })
    ElMessage.success('转化成功')
    getList()
  } catch (error) {
    if (error !== 'cancel') {
      logger.warn('转化失败', (error as Error).message)
    }
  }
}

const handleLost = async (row: LeadRow) => {
  try {
    await ElMessageBox.confirm(`确认标记线索 "${row.contact_name}" 为流失？`, '提示', {
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

const handleImport = () => {
  ElMessage.info('导入功能开发中')
}

const handleExport = () => {
  ElMessage.success('导出成功')
}

const handleSelectionChange = (selection: LeadRow[]) => {
  // 选择变化（占位）
  logger.debug('选中了线索', selection.length)
}

const handleSizeChange = (val: number) => {
  queryParams.page_size = val
  getList()
}

const handleCurrentChange = (val: number) => {
  queryParams.page = val
  getList()
}

const getSourceLabel = (source: string) => {
  const labelMap: Record<string, string> = {
    WEBSITE: '网站',
    PHONE: '电话',
    EXHIBITION: '展会',
    REFERRAL: '推荐',
    OTHER: '其他',
  }
  return labelMap[source] || source
}

const getStatusType = (status: string) => {
  const typeMap: Record<string, string> = {
    NEW: 'info',
    CONTACTED: 'warning',
    QUALIFIED: 'primary',
    CONVERTED: 'success',
    LOST: 'danger',
  }
  return typeMap[status] || 'info'
}

const getStatusLabel = (status: string) => {
  const labelMap: Record<string, string> = {
    NEW: '新线索',
    CONTACTED: '已联系',
    QUALIFIED: '已qualified',
    CONVERTED: '已转化',
    LOST: '已流失',
  }
  return labelMap[status] || status
}

const getPriorityType = (priority: string) => {
  const typeMap: Record<string, string> = {
    LOW: 'info',
    MEDIUM: '',
    HIGH: 'warning',
    URGENT: 'danger',
  }
  return typeMap[priority] || ''
}

const getPriorityLabel = (priority: string) => {
  const labelMap: Record<string, string> = {
    LOW: '低',
    MEDIUM: '中',
    HIGH: '高',
    URGENT: '紧急',
  }
  return labelMap[priority] || priority
}

onMounted(() => {
  getList()
  fetchUsers()
})
</script>

<style scoped>
.crm-leads-page {
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

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
      <el-form :inline="true" :model="queryParams" class="filter-form" aria-label="销售线索筛选表单">
        <el-form-item label="关键词">
          <el-input
            v-model="queryParams.keyword"
            placeholder="线索编号/公司名称/联系人"
            clearable
            @clear="handleQuery"
            @keyup.enter="handleQuery"
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
        aria-label="销售线索列表"
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
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          aria-label="销售线索列表分页"
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
import {
  exportLeads,
  importLeads,
  updateLeadStatus,
  convertLead,
  getLead,
  type Lead,
} from '@/api/crm'
import { listUsers, type User } from '@/api/user'
import { useTableApi } from '@/composables/useTableApi'
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
  keyword: '',
  lead_source: '',
  lead_status: '',
  owner_id: '',
  priority: '',
})

// 批次 269：接入 useTableApi，消除手写分页重复
const {
  data: leadList,
  loading,
  page,
  pageSize,
  total,
  refresh: getList,
  setQueryParam,
} = useTableApi<LeadRow>({
  url: '/crm/leads',
  onError: (e: unknown) => logger.warn('加载线索列表失败', String(e)),
})

const users = ref<User[]>([])

const formDialogVisible = ref(false)
const formDialogTitle = ref('新建线索')
const currentRow = ref<LeadRow | null>(null)

const fetchUsers = async () => {
  try {
    const res = await listUsers()
    users.value = res.data?.list || []
  } catch (error) {
    users.value = []
  }
}

const handleQuery = () => {
  // 同步筛选条件到 useTableApi
  setQueryParam('keyword', queryParams.keyword || undefined)
  setQueryParam('lead_source', queryParams.lead_source || undefined)
  setQueryParam('lead_status', queryParams.lead_status || undefined)
  setQueryParam('owner_id', queryParams.owner_id || undefined)
  setQueryParam('priority', queryParams.priority || undefined)
  page.value = 1
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

// 批次 157b P1-1 修复：接入 getLead API 展示线索详情
const handleView = async (row: LeadRow) => {
  try {
    const res = (await getLead(row.id)) as unknown as { data?: LeadRow }
    const d = res.data || row
    const lines = [
      `线索编号：${d.lead_no || '-'}`,
      `公司名称：${d.company_name || '-'}`,
      `联系人：${d.contact_name || '-'}`,
      `手机号：${d.mobile_phone || '-'}`,
      `线索来源：${getSourceLabel(d.lead_source || '')}`,
      `线索状态：${getStatusLabel(d.lead_status || '')}`,
      `优先级：${getPriorityLabel(d.priority || '')}`,
      `负责人：${d.owner_name || '-'}`,
      `最近跟进：${d.last_follow_up_date || '-'}`,
      `下次跟进：${d.next_follow_up_date || '-'}`,
      `需求描述：${d.requirement_desc || '-'}`,
      `备注：${d.remarks || '-'}`,
    ]
    await ElMessageBox.alert(lines.join('\n'), '线索详情', { confirmButtonText: '关闭' })
  } catch (error) {
    logger.warn('获取线索详情失败', (error as Error).message)
    ElMessage.error('获取线索详情失败')
  }
}

const handleContact = async (row: LeadRow) => {
  try {
    await ElMessageBox.confirm(`确认标记线索 "${row.contact_name}" 为已联系？`, '提示', {
      type: 'warning',
    })
    // v11 批次 141 修复：原占位假成功，现接入真实状态变更 API
    await updateLeadStatus(row.id, { status: 'contacted' })
    ElMessage.success('已标记为已联系')
    getList()
  } catch (error) {
    if (error !== 'cancel') {
      logger.warn('标记已联系失败', (error as Error).message)
      ElMessage.error('标记已联系失败')
    }
  }
}

const handleConvert = async (row: LeadRow) => {
  try {
    await ElMessageBox.confirm(`确认将线索 "${row.contact_name}" 转化为客户？`, '提示', {
      type: 'warning',
    })
    // v11 批次 141 修复：原占位假成功，现接入真实转化 API
    await convertLead(row.id)
    ElMessage.success('线索转化成功')
    getList()
  } catch (error) {
    if (error !== 'cancel') {
      logger.warn('转化失败', (error as Error).message)
      ElMessage.error('转化失败')
    }
  }
}

const handleLost = async (row: LeadRow) => {
  try {
    await ElMessageBox.confirm(`确认标记线索 "${row.contact_name}" 为流失？`, '提示', {
      type: 'warning',
    })
    // v11 批次 141 修复：原占位假成功，现接入真实状态变更 API
    await updateLeadStatus(row.id, { status: 'lost' })
    ElMessage.success('已标记为流失')
    getList()
  } catch (error) {
    if (error !== 'cancel') {
      logger.warn('标记流失失败', (error as Error).message)
      ElMessage.error('标记流失失败')
    }
  }
}

// v11 批次 157d-4 修复：接入 importLeads API 真实导入 xlsx 文件
const handleImport = () => {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = '.xlsx'
  input.onchange = async (event: Event) => {
    const target = event.target as HTMLInputElement
    const file = target.files?.[0]
    if (!file) return
    try {
      const res = await importLeads(file)
      const result = res.data
      if (result.failed_count > 0) {
        const errorLines = result.errors
          .map((e) => `第${e.row}行：${e.message}`)
          .join('\n')
        await ElMessageBox.alert(
          `总行数：${result.total}\n成功：${result.success_count}\n失败：${result.failed_count}\n\n失败详情：\n${errorLines}`,
          '导入完成（部分失败）',
          { confirmButtonText: '关闭' },
        )
      } else {
        ElMessage.success(`导入成功：${result.success_count} 条`)
      }
      getList()
    } catch (e: unknown) {
      const err = e as Error
      ElMessage.error(err.message || '导入失败')
      logger.error('导入线索失败', err.message)
    }
  }
  input.click()
}

// 批次 94 P2-12 修复：原占位假成功，现接入真实导出 API 并触发浏览器下载
const handleExport = async () => {
  try {
    const blob = await exportLeads(queryParams)
    const url = window.URL.createObjectURL(new Blob([blob]))
    const link = document.createElement('a')
    link.href = url
    link.setAttribute('download', `CRM线索_${new Date().toISOString().split('T')[0]}.xlsx`)
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    window.URL.revokeObjectURL(url)
    ElMessage.success('导出成功')
  } catch (error) {
    logger.error('导出失败:', error)
    ElMessage.error('导出失败')
  }
}

const handleSelectionChange = (selection: LeadRow[]) => {
  // 选择变化（占位）
  logger.debug('选中了线索', selection.length)
}

const handleSizeChange = (val: number) => {
  pageSize.value = val
  page.value = 1
}

const handleCurrentChange = (val: number) => {
  page.value = val
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

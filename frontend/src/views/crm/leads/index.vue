<!--
  crm/leads/index.vue - 线索管理主入口
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-3）：
  原 595 行"上帝组件"已拆分为：
  - tabs/LeadFormTab.vue - 新建/编辑线索对话框

  本主入口承担：页面布局 + 列表数据 + 公共样式。
  D05 Batch 5：接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts + en-US.ts
-->
<template>
  <div class="crm-leads-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ $t('crmLeads.title') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{ $t('crmLeads.breadcrumb.home') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('crmLeads.breadcrumb.crm') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('crmLeads.breadcrumb.leads') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="openCreateDialog">
          <el-icon><Plus /></el-icon>
          {{ $t('crmLeads.create') }}
        </el-button>
        <el-button @click="handleImport">
          <el-icon><Upload /></el-icon>
          {{ $t('crmLeads.import') }}
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          {{ $t('crmLeads.export') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form" :aria-label="$t('crmLeads.filter.ariaLabel')">
        <el-form-item :label="$t('crmLeads.filter.keyword')">
          <el-input
            v-model="queryParams.keyword"
            :placeholder="$t('crmLeads.filter.keywordPlaceholder')"
            clearable
            @clear="handleQuery"
            @keyup.enter="handleQuery"
          />
        </el-form-item>
        <el-form-item :label="$t('crmLeads.filter.leadSource')">
          <el-select
            v-model="queryParams.lead_source"
            :placeholder="$t('crmLeads.filter.leadSourcePlaceholder')"
            clearable
            @change="handleQuery"
          >
            <el-option :label="$t('crmLeads.leadSource.website')" value="WEBSITE" />
            <el-option :label="$t('crmLeads.leadSource.phone')" value="PHONE" />
            <el-option :label="$t('crmLeads.leadSource.exhibition')" value="EXHIBITION" />
            <el-option :label="$t('crmLeads.leadSource.referral')" value="REFERRAL" />
            <el-option :label="$t('crmLeads.leadSource.other')" value="OTHER" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('crmLeads.filter.leadStatus')">
          <el-select
            v-model="queryParams.lead_status"
            :placeholder="$t('crmLeads.filter.leadStatusPlaceholder')"
            clearable
            @change="handleQuery"
          >
            <el-option :label="$t('crmLeads.leadStatus.new')" value="NEW" />
            <el-option :label="$t('crmLeads.leadStatus.contacted')" value="CONTACTED" />
            <el-option :label="$t('crmLeads.leadStatus.qualified')" value="QUALIFIED" />
            <el-option :label="$t('crmLeads.leadStatus.converted')" value="CONVERTED" />
            <el-option :label="$t('crmLeads.leadStatus.lost')" value="LOST" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('crmLeads.filter.owner')">
          <el-select
            v-model="queryParams.owner_id"
            :placeholder="$t('crmLeads.filter.ownerPlaceholder')"
            clearable
            filterable
            @change="handleQuery"
          >
            <el-option v-for="u in users" :key="u.id" :label="u.real_name" :value="u.id" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('crmLeads.filter.priority')">
          <el-select
            v-model="queryParams.priority"
            :placeholder="$t('crmLeads.filter.priorityPlaceholder')"
            clearable
            @change="handleQuery"
          >
            <el-option :label="$t('crmLeads.priority.low')" value="LOW" />
            <el-option :label="$t('crmLeads.priority.medium')" value="MEDIUM" />
            <el-option :label="$t('crmLeads.priority.high')" value="HIGH" />
            <el-option :label="$t('crmLeads.priority.urgent')" value="URGENT" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            {{ $t('crmLeads.filter.query') }}
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            {{ $t('crmLeads.filter.reset') }}
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
        :aria-label="$t('crmLeads.table.ariaLabel')"
        @selection-change="handleSelectionChange"
      >
        <el-table-column type="selection" width="55" align="center" />
        <el-table-column type="index" :label="$t('crmLeads.table.index')" width="60" align="center" />
        <el-table-column prop="lead_no" :label="$t('crmLeads.table.leadNo')" width="120" show-overflow-tooltip />
        <el-table-column
          prop="company_name"
          :label="$t('crmLeads.table.companyName')"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column prop="contact_name" :label="$t('crmLeads.table.contactName')" width="100" show-overflow-tooltip />
        <el-table-column prop="mobile_phone" :label="$t('crmLeads.table.mobilePhone')" width="120" show-overflow-tooltip />
        <el-table-column prop="lead_source" :label="$t('crmLeads.table.leadSource')" width="100" align="center">
          <template #default="{ row }">
            <el-tag>{{ getSourceLabel(row.lead_source) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="lead_status" :label="$t('crmLeads.table.leadStatus')" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.lead_status)">{{
              getStatusLabel(row.lead_status)
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="priority" :label="$t('crmLeads.table.priority')" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="getPriorityType(row.priority)">{{
              getPriorityLabel(row.priority)
            }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="owner_name" :label="$t('crmLeads.table.owner')" width="100" show-overflow-tooltip />
        <el-table-column prop="last_follow_up_date" :label="$t('crmLeads.table.lastFollowUp')" width="120" align="center" />
        <el-table-column prop="next_follow_up_date" :label="$t('crmLeads.table.nextFollowUp')" width="120" align="center" />
        <el-table-column :label="$t('crmLeads.table.operation')" width="250" align="center" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">{{ $t('crmLeads.table.view') }}</el-button>
            <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
            <el-button
              v-if="row.lead_status !== 'CONVERTED'"
              v-permission="'crm_lead:update'"
              type="primary"
              link
              size="small"
              @click="openEditDialog(row)"
              >{{ $t('crmLeads.table.edit') }}</el-button
            >
            <el-button
              v-if="row.lead_status === 'NEW'"
              type="warning"
              link
              size="small"
              @click="handleContact(row)"
              >{{ $t('crmLeads.table.contact') }}</el-button
            >
            <el-button
              v-if="row.lead_status === 'QUALIFIED'"
              type="success"
              link
              size="small"
              @click="handleConvert(row)"
              >{{ $t('crmLeads.table.convert') }}</el-button
            >
            <el-button
              v-if="row.lead_status !== 'CONVERTED'"
              type="danger"
              link
              size="small"
              @click="handleLost(row)"
              >{{ $t('crmLeads.table.lost') }}</el-button
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
          :aria-label="$t('crmLeads.table.paginationAriaLabel')"
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
import { useI18n } from 'vue-i18n'
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
import { getUserList, type User } from '@/api/user'
import { useTableApi } from '@/composables/useTableApi'
import { logger } from '@/utils/logger'
import LeadFormTab from './tabs/LeadFormTab.vue'

const { t } = useI18n({ useScope: 'global' })

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
  onError: (e: unknown) => logger.warn(t('crmLeads.message.loadListFailed'), String(e)),
})

const users = ref<User[]>([])

const formDialogVisible = ref(false)
const formDialogTitle = ref(t('crmLeads.dialog.createTitle'))
const currentRow = ref<LeadRow | null>(null)

const fetchUsers = async () => {
  try {
    const res = await getUserList()
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
  formDialogTitle.value = t('crmLeads.dialog.createTitle')
  formDialogVisible.value = true
}

const openEditDialog = (row: LeadRow) => {
  currentRow.value = row
  formDialogTitle.value = t('crmLeads.dialog.editTitle')
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
      `${t('crmLeads.detail.leadNo')}${d.lead_no || '-'}`,
      `${t('crmLeads.detail.companyName')}${d.company_name || '-'}`,
      `${t('crmLeads.detail.contactName')}${d.contact_name || '-'}`,
      `${t('crmLeads.detail.mobilePhone')}${d.mobile_phone || '-'}`,
      `${t('crmLeads.detail.leadSource')}${getSourceLabel(d.lead_source || '')}`,
      `${t('crmLeads.detail.leadStatus')}${getStatusLabel(d.lead_status || '')}`,
      `${t('crmLeads.detail.priority')}${getPriorityLabel(d.priority || '')}`,
      `${t('crmLeads.detail.owner')}${d.owner_name || '-'}`,
      `${t('crmLeads.detail.lastFollowUp')}${d.last_follow_up_date || '-'}`,
      `${t('crmLeads.detail.nextFollowUp')}${d.next_follow_up_date || '-'}`,
      `${t('crmLeads.detail.requirementDesc')}${d.requirement_desc || '-'}`,
      `${t('crmLeads.detail.remarks')}${d.remarks || '-'}`,
    ]
    await ElMessageBox.alert(lines.join('\n'), t('crmLeads.detail.title'), { confirmButtonText: t('crmLeads.detail.close') })
  } catch (error) {
    logger.warn(t('crmLeads.message.getDetailFailed'), (error as Error).message)
    ElMessage.error(t('crmLeads.message.getDetailFailed'))
  }
}

const handleContact = async (row: LeadRow) => {
  try {
    await ElMessageBox.confirm(t('crmLeads.message.contactConfirm', { name: row.contact_name }), t('crmLeads.message.tip'), {
      type: 'warning',
    })
    // v11 批次 141 修复：原占位假成功，现接入真实状态变更 API
    await updateLeadStatus(row.id, { status: 'contacted' })
    ElMessage.success(t('crmLeads.message.contactSuccess'))
    getList()
  } catch (error) {
    if (error !== 'cancel') {
      logger.warn(t('crmLeads.message.contactFailed'), (error as Error).message)
      ElMessage.error(t('crmLeads.message.contactFailed'))
    }
  }
}

const handleConvert = async (row: LeadRow) => {
  try {
    await ElMessageBox.confirm(t('crmLeads.message.convertConfirm', { name: row.contact_name }), t('crmLeads.message.tip'), {
      type: 'warning',
    })
    // v11 批次 141 修复：原占位假成功，现接入真实转化 API
    await convertLead(row.id)
    ElMessage.success(t('crmLeads.message.convertSuccess'))
    getList()
  } catch (error) {
    if (error !== 'cancel') {
      logger.warn(t('crmLeads.message.convertFailed'), (error as Error).message)
      ElMessage.error(t('crmLeads.message.convertFailed'))
    }
  }
}

const handleLost = async (row: LeadRow) => {
  try {
    await ElMessageBox.confirm(t('crmLeads.message.lostConfirm', { name: row.contact_name }), t('crmLeads.message.tip'), {
      type: 'warning',
    })
    // v11 批次 141 修复：原占位假成功，现接入真实状态变更 API
    await updateLeadStatus(row.id, { status: 'lost' })
    ElMessage.success(t('crmLeads.message.lostSuccess'))
    getList()
  } catch (error) {
    if (error !== 'cancel') {
      logger.warn(t('crmLeads.message.lostFailed'), (error as Error).message)
      ElMessage.error(t('crmLeads.message.lostFailed'))
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
          .map((e) => t('crmLeads.message.importErrorLine', { row: e.row, message: e.message }))
          .join('\n')
        await ElMessageBox.alert(
          t('crmLeads.message.importPartialResult', { total: result.total, success: result.success_count, failed: result.failed_count, detail: errorLines }),
          t('crmLeads.message.importPartialTitle'),
          { confirmButtonText: t('crmLeads.message.close') },
        )
      } else {
        ElMessage.success(t('crmLeads.message.importSuccess', { count: result.success_count }))
      }
      getList()
    } catch (e: unknown) {
      const err = e as Error
      ElMessage.error(err.message || t('crmLeads.message.importFailed'))
      logger.error(t('crmLeads.message.importFailed'), err.message)
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
    link.setAttribute('download', `${t('crmLeads.exportFile.filename')}_${new Date().toISOString().split('T')[0]}.xlsx`)
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    window.URL.revokeObjectURL(url)
    ElMessage.success(t('crmLeads.message.exportSuccess'))
  } catch (error) {
    logger.error(t('crmLeads.message.exportFailed'), error)
    ElMessage.error(t('crmLeads.message.exportFailed'))
  }
}

const handleSelectionChange = (selection: LeadRow[]) => {
  // 选择变化（占位）
  logger.debug(t('crmLeads.message.selectionChanged'), selection.length)
}

const handleSizeChange = (val: number) => {
  pageSize.value = val
  page.value = 1
}

const handleCurrentChange = (val: number) => {
  page.value = val
}

// D05 Batch 5：getSourceLabel/getStatusLabel/getPriorityLabel 改为函数，使 t() 在每次渲染时响应式求值
const getSourceLabel = (source: string) => {
  const labelMap: Record<string, string> = {
    WEBSITE: t('crmLeads.leadSource.website'),
    PHONE: t('crmLeads.leadSource.phone'),
    EXHIBITION: t('crmLeads.leadSource.exhibition'),
    REFERRAL: t('crmLeads.leadSource.referral'),
    OTHER: t('crmLeads.leadSource.other'),
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
    NEW: t('crmLeads.leadStatus.new'),
    CONTACTED: t('crmLeads.leadStatus.contacted'),
    QUALIFIED: t('crmLeads.leadStatus.qualified'),
    CONVERTED: t('crmLeads.leadStatus.converted'),
    LOST: t('crmLeads.leadStatus.lost'),
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
    LOW: t('crmLeads.priority.low'),
    MEDIUM: t('crmLeads.priority.medium'),
    HIGH: t('crmLeads.priority.high'),
    URGENT: t('crmLeads.priority.urgent'),
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

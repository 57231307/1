<template>
  <div class="email-management">
    <div class="page-header">
      <h2>{{ t('email.index.pageTitle') }}</h2>
    </div>

    <el-tabs
      v-model="activeTab"
      type="border-card"
      :aria-label="t('email.index.ariaTabs')"
      @tab-change="handleTabChange"
    >
      <!-- 邮件模板 Tab -->
      <el-tab-pane :label="t('email.index.tabTemplates')" name="templates">
        <div class="tab-header">
          <el-button type="primary" @click="handleCreateTemplate">
            <el-icon><Plus /></el-icon>
            {{ t('email.index.buttonCreateTemplate') }}
          </el-button>
        </div>

        <el-table
          v-loading="templatesLoading"
          :data="templates"
          border
          stripe
          :aria-label="t('email.index.ariaTemplateTable')"
        >
          <el-table-column prop="name" :label="t('email.index.colTemplateName')" min-width="150" />
          <el-table-column prop="code" :label="t('email.index.colTemplateCode')" min-width="120" />
          <el-table-column
            prop="template_type"
            :label="t('email.index.colTemplateType')"
            min-width="100"
          >
            <template #default="{ row }">
              <el-tag>{{ row.template_type }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column
            prop="description"
            :label="t('email.index.colDescription')"
            min-width="200"
            show-overflow-tooltip
          />
          <el-table-column
            prop="is_active"
            :label="t('email.index.colStatus')"
            width="80"
            align="center"
          >
            <template #default="{ row }">
              <el-tag :type="row.is_active ? 'success' : 'danger'">
                {{
                  row.is_active ? t('email.index.statusEnabled') : t('email.index.statusDisabled')
                }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column :label="t('email.index.colOperation')" width="200" fixed="right">
            <template #default="{ row }">
              <el-button
                v-permission="'email_template:update'"
                size="small"
                @click="handleEditTemplate(row)"
                >{{ t('email.index.buttonEdit') }}</el-button
              >
              <el-button
                v-permission="'email_template:delete'"
                size="small"
                type="danger"
                @click="handleDeleteTemplate(row)"
                >{{ t('email.index.buttonDelete') }}</el-button
              >
            </template>
          </el-table-column>
        </el-table>

        <el-pagination
          v-model:current-page="templatePage"
          v-model:page-size="templatePageSize"
          :total="templateTotal"
          :page-sizes="[10, 20, 50, 100]"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="t('email.index.ariaTemplatePagination')"
        />
      </el-tab-pane>

      <!-- 发送记录 Tab -->
      <el-tab-pane :label="t('email.index.tabRecords')" name="records">
        <div class="tab-header">
          <el-form
            :inline="true"
            :model="recordStatus"
            :aria-label="t('email.index.ariaRecordFilterForm')"
          >
            <el-form-item :label="t('email.index.filterStatus')">
              <el-select
                v-model="recordStatus"
                clearable
                :placeholder="t('email.index.placeholderSelectStatus')"
              >
                <el-option :label="t('email.index.optionSent')" value="sent" />
                <el-option :label="t('email.index.optionFailed')" value="failed" />
                <el-option :label="t('email.index.optionPending')" value="pending" />
              </el-select>
            </el-form-item>
            <el-form-item :label="t('email.index.filterDateRange')">
              <el-date-picker
                v-model="recordDateRange"
                type="daterange"
                :range-separator="t('common.dateRange.to')"
                :start-placeholder="t('email.index.placeholderStartDate')"
                :end-placeholder="t('email.index.placeholderEndDate')"
              />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="handleSearchRecords">{{
                t('email.index.buttonQuery')
              }}</el-button>
              <el-button @click="handleResetRecordQuery">{{
                t('email.index.buttonReset')
              }}</el-button>
            </el-form-item>
          </el-form>
        </div>

        <el-table
          v-loading="recordsLoading"
          :data="records"
          border
          stripe
          :aria-label="t('email.index.ariaRecordTable')"
        >
          <el-table-column prop="to" :label="t('email.index.colRecipient')" min-width="150" />
          <el-table-column
            prop="subject"
            :label="t('email.index.colSubject')"
            min-width="200"
            show-overflow-tooltip
          />
          <el-table-column
            prop="status"
            :label="t('email.index.colStatus')"
            width="80"
            align="center"
          >
            <template #default="{ row }">
              <el-tag :type="getStatusType(row.status)">
                {{ getStatusText(row.status) }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="sent_at" :label="t('email.index.colSentAt')" min-width="160" />
          <el-table-column
            prop="error_message"
            :label="t('email.index.colErrorMessage')"
            min-width="200"
            show-overflow-tooltip
          />
        </el-table>

        <el-pagination
          v-model:current-page="recordPage"
          v-model:page-size="recordPageSize"
          :total="recordTotal"
          :page-sizes="[10, 20, 50, 100]"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="t('email.index.ariaRecordPagination')"
        />
      </el-tab-pane>

      <!-- 发送统计 Tab -->
      <el-tab-pane :label="t('email.index.tabStatistics')" name="statistics">
        <el-row :gutter="20">
          <el-col :span="6">
            <el-card shadow="hover">
              <template #header>{{ t('email.index.statTotalSent') }}</template>
              <div class="stat-value">{{ statistics.total_sent || 0 }}</div>
            </el-card>
          </el-col>
          <el-col :span="6">
            <el-card shadow="hover">
              <template #header>{{ t('email.index.statTotalFailed') }}</template>
              <div class="stat-value text-danger">{{ statistics.total_failed || 0 }}</div>
            </el-card>
          </el-col>
          <el-col :span="6">
            <el-card shadow="hover">
              <template #header>{{ t('email.index.statTodaySent') }}</template>
              <div class="stat-value text-primary">{{ statistics.today_sent || 0 }}</div>
            </el-card>
          </el-col>
          <el-col :span="6">
            <el-card shadow="hover">
              <template #header>{{ t('email.index.statSuccessRate') }}</template>
              <div class="stat-value text-success">{{ statistics.success_rate || 0 }}%</div>
            </el-card>
          </el-col>
        </el-row>
      </el-tab-pane>
    </el-tabs>

    <!-- 模板编辑对话框 -->
    <el-dialog
      v-model="templateDialogVisible"
      :title="
        isEditTemplate ? t('email.index.titleEditTemplate') : t('email.index.titleCreateTemplate')
      "
      width="600px"
      :aria-label="t('email.index.ariaEditDialog')"
    >
      <el-form
        ref="templateFormRef"
        :model="templateForm"
        :rules="templateRules"
        label-width="100px"
        :aria-label="t('email.index.ariaTemplateForm')"
      >
        <el-form-item :label="t('email.index.colTemplateName')" prop="name">
          <el-input
            v-model="templateForm.name"
            :placeholder="t('email.index.placeholderTemplateName')"
          />
        </el-form-item>
        <el-form-item :label="t('email.index.colTemplateCode')" prop="code">
          <el-input
            v-model="templateForm.code"
            :placeholder="t('email.index.placeholderTemplateCode')"
            :disabled="isEditTemplate"
          />
        </el-form-item>
        <el-form-item :label="t('email.index.colTemplateType')" prop="template_type">
          <el-select
            v-model="templateForm.template_type"
            :placeholder="t('email.index.placeholderSelectTemplateType')"
          >
            <el-option :label="t('email.index.optionSystemNotification')" value="system" />
            <el-option :label="t('email.index.optionOrderNotification')" value="order" />
            <el-option :label="t('email.index.optionApprovalNotification')" value="approval" />
            <el-option :label="t('email.index.optionInventoryNotification')" value="inventory" />
            <el-option :label="t('email.index.optionCustom')" value="custom" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('email.index.labelSubject')" prop="subject_template">
          <el-input
            v-model="templateForm.subject_template"
            :placeholder="t('email.index.placeholderSubject')"
          />
        </el-form-item>
        <el-form-item :label="t('email.index.labelBody')" prop="body_template">
          <el-input
            v-model="templateForm.body_template"
            type="textarea"
            :rows="10"
            :placeholder="t('email.index.placeholderBody')"
          />
        </el-form-item>
        <el-form-item :label="t('email.index.colDescription')">
          <el-input
            v-model="templateForm.description"
            type="textarea"
            :rows="3"
            :placeholder="t('email.index.placeholderDescription')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="templateDialogVisible = false">{{
          t('email.index.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmitTemplate">{{
          t('email.index.buttonConfirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox, type TabsPaneContext } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  getEmailStatistics,
  updateEmailTemplate,
  createEmailTemplate,
  deleteEmailTemplate,
  type EmailTemplate,
  type EmailLog,
  type EmailStatistics,
} from '@/api/email'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
// 批次 280：接入 useTableApi，消除手写 templates/records 分页重复
import { useTableApi } from '@/composables/useTableApi'

const { t } = useI18n({ useScope: 'global' })

const activeTab = ref('templates')
const hasLoaded = createLazyLoader()

// 批次 280：模板表格 useTableApi（getEmailTemplateList 返回 { data: { list, total } }）
const {
  data: templates,
  loading: templatesLoading,
  page: templatePage,
  pageSize: templatePageSize,
  total: templateTotal,
  refresh: fetchTemplates,
} = useTableApi<EmailTemplate>({
  url: '/email-templates',
  onError: (err: unknown) => logger.error(t('email.index.messageFetchTemplatesFailed'), err),
})

const templateDialogVisible = ref(false)
const isEditTemplate = ref(false)
const submitLoading = ref(false)
const templateFormRef = ref()
const templateForm = reactive<Partial<EmailTemplate>>({
  name: '',
  code: '',
  subject_template: '',
  body_template: '',
  template_type: '',
  description: '',
})
const templateRules = {
  name: [{ required: true, message: t('email.index.ruleTemplateNameRequired'), trigger: 'blur' }],
  code: [{ required: true, message: t('email.index.ruleTemplateCodeRequired'), trigger: 'blur' }],
  subject_template: [
    { required: true, message: t('email.index.ruleSubjectRequired'), trigger: 'blur' },
  ],
  body_template: [{ required: true, message: t('email.index.ruleBodyRequired'), trigger: 'blur' }],
  template_type: [
    { required: true, message: t('email.index.ruleTemplateTypeRequired'), trigger: 'change' },
  ],
}

// 批次 280：记录表格 useTableApi（getEmailRecordList 返回 { data: { list, total } }）
const {
  data: records,
  loading: recordsLoading,
  page: recordPage,
  pageSize: recordPageSize,
  total: recordTotal,
  refresh: fetchRecords,
  setQueryParam: setRecordQueryParam,
} = useTableApi<EmailLog>({
  url: '/email-records',
  onError: (err: unknown) => logger.error(t('email.index.messageFetchRecordsFailed'), err),
})

// 批次 280：记录筛选状态 + 日期范围（分页由 useTableApi 管理）
const recordStatus = ref('')
const recordDateRange = ref<[Date, Date] | null>(null)

// 统计相关
const statistics = ref<EmailStatistics>({
  total_sent: 0,
  total_failed: 0,
  today_sent: 0,
  success_rate: 0,
})

/// 处理 tab 切换事件（element-plus TabsPaneContext 类型）
const handleTabChange = (tab: TabsPaneContext) => {
  loadTab(tab.paneName as string)
}

const loadTab = (tabName: string) => {
  const tabLoaders: Record<string, () => void> = {
    templates: fetchTemplates,
    records: fetchRecords,
    statistics: fetchStatistics,
  }
  if (tabLoaders[tabName]) {
    loadIfNot(tabName, tabLoaders[tabName], hasLoaded)
  }
}

const initPage = () => {
  loadTab(activeTab.value)
}

// 批次 280：组件 setup 阶段 useTableApi 已自动加载，但 email 页用 lazy-loader 按 tab 加载
// 需要在首次进入 tab 时触发加载（lazy-loader 的 loadIfNot 会调用 fetchTemplates/fetchRecords）
initPage()

const fetchStatistics = async () => {
  try {
    const res = await getEmailStatistics()
    if (res.data) {
      statistics.value = res.data
    }
  } catch (error) {
    logger.error(t('email.index.messageFetchStatisticsFailed'), error)
  }
}

const handleCreateTemplate = () => {
  isEditTemplate.value = false
  Object.assign(templateForm, {
    id: undefined,
    name: '',
    code: '',
    subject_template: '',
    body_template: '',
    template_type: '',
    description: '',
  })
  templateDialogVisible.value = true
}

const handleEditTemplate = (row: EmailTemplate) => {
  isEditTemplate.value = true
  Object.assign(templateForm, row)
  templateDialogVisible.value = true
}

const handleSubmitTemplate = async () => {
  try {
    await templateFormRef.value?.validate()
    submitLoading.value = true
    if (isEditTemplate.value && templateForm.id) {
      await updateEmailTemplate(templateForm.id, templateForm)
      ElMessage.success(t('email.index.messageUpdateSuccess'))
    } else {
      await createEmailTemplate(templateForm)
      ElMessage.success(t('email.index.messageCreateSuccess'))
    }
    templateDialogVisible.value = false
    fetchTemplates()
  } catch (error) {
    logger.error(t('email.index.messageSubmitFailed'), error)
  } finally {
    submitLoading.value = false
  }
}

const handleDeleteTemplate = async (row: EmailTemplate) => {
  try {
    await ElMessageBox.confirm(
      t('email.index.messageConfirmDelete'),
      t('email.index.titlePrompt'),
      { type: 'warning' }
    )
    await deleteEmailTemplate(row.id!)
    ElMessage.success(t('email.index.messageDeleteSuccess'))
    fetchTemplates()
  } catch (error) {
    if (error !== 'cancel') {
      logger.error(t('email.index.messageDeleteFailed'), error)
    }
  }
}

// 批次 280：记录查询时同步筛选条件到 useTableApi.queryParams
const handleSearchRecords = () => {
  setRecordQueryParam('status', recordStatus.value || undefined)
  if (recordDateRange.value) {
    setRecordQueryParam('start_date', recordDateRange.value[0].toISOString())
    setRecordQueryParam('end_date', recordDateRange.value[1].toISOString())
  } else {
    setRecordQueryParam('start_date', undefined)
    setRecordQueryParam('end_date', undefined)
  }
  recordPage.value = 1
  fetchRecords()
}

const handleResetRecordQuery = () => {
  recordStatus.value = ''
  recordDateRange.value = null
  handleSearchRecords()
}

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    sent: 'success',
    failed: 'danger',
    pending: 'warning',
  }
  return map[status] || 'info'
}

const getStatusText = (status: string) => {
  const map: Record<string, string> = {
    sent: t('email.index.optionSent'),
    failed: t('email.index.optionFailed'),
    pending: t('email.index.optionPending'),
  }
  return map[status] || status
}
</script>

<style scoped>
.email-management {
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

.tab-header {
  margin-bottom: 20px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.stat-value {
  font-size: 32px;
  font-weight: 600;
  text-align: center;
  padding: 20px 0;
}

.text-danger {
  color: #f56c6c;
}

.text-primary {
  color: #409eff;
}

.text-success {
  color: #67c23a;
}

.el-pagination {
  margin-top: 20px;
  justify-content: flex-end;
}
</style>

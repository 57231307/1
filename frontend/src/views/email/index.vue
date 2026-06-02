<template>
  <div class="email-management">
    <div class="page-header">
      <h2>邮件管理</h2>
    </div>

    <el-tabs v-model="activeTab" @tab-change="(tab) => loadTab(tab, hasLoaded)" type="border-card">
      <!-- 邮件模板 Tab -->
      <el-tab-pane label="邮件模板" name="templates">
        <div class="tab-header">
          <el-button type="primary" @click="handleCreateTemplate">
            <el-icon><Plus /></el-icon>
            新建模板
          </el-button>
        </div>

        <el-table :data="templates" v-loading="templatesLoading" border stripe>
          <el-table-column prop="name" label="模板名称" min-width="150" />
          <el-table-column prop="code" label="模板编码" min-width="120" />
          <el-table-column prop="template_type" label="模板类型" min-width="100">
            <template #default="{ row }">
              <el-tag>{{ row.template_type }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="description" label="描述" min-width="200" show-overflow-tooltip />
          <el-table-column prop="is_active" label="状态" width="80" align="center">
            <template #default="{ row }">
              <el-tag :type="row.is_active ? 'success' : 'danger'">
                {{ row.is_active ? '启用' : '禁用' }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="200" fixed="right">
            <template #default="{ row }">
              <el-button size="small" @click="handleEditTemplate(row)">编辑</el-button>
              <el-button size="small" type="danger" @click="handleDeleteTemplate(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>

        <el-pagination
          v-model:current-page="templateQuery.page"
          v-model:page-size="templateQuery.page_size"
          :total="templateTotal"
          :page-sizes="[10, 20, 50, 100]"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="fetchTemplates"
          @current-change="fetchTemplates"
        />
      </el-tab-pane>

      <!-- 发送记录 Tab -->
      <el-tab-pane label="发送记录" name="records">
        <div class="tab-header">
          <el-form :inline="true" :model="recordQuery">
            <el-form-item label="状态">
              <el-select v-model="recordQuery.status" clearable placeholder="选择状态">
                <el-option label="成功" value="sent" />
                <el-option label="失败" value="failed" />
                <el-option label="待发送" value="pending" />
              </el-select>
            </el-form-item>
            <el-form-item label="日期范围">
              <el-date-picker
                v-model="recordDateRange"
                type="daterange"
                range-separator="至"
                start-placeholder="开始日期"
                end-placeholder="结束日期"
              />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="fetchRecords">查询</el-button>
              <el-button @click="handleResetRecordQuery">重置</el-button>
            </el-form-item>
          </el-form>
        </div>

        <el-table :data="records" v-loading="recordsLoading" border stripe>
          <el-table-column prop="to" label="收件人" min-width="150" />
          <el-table-column prop="subject" label="主题" min-width="200" show-overflow-tooltip />
          <el-table-column prop="status" label="状态" width="80" align="center">
            <template #default="{ row }">
              <el-tag :type="getStatusType(row.status)">
                {{ getStatusText(row.status) }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="sent_at" label="发送时间" min-width="160" />
          <el-table-column prop="error_message" label="错误信息" min-width="200" show-overflow-tooltip />
        </el-table>

        <el-pagination
          v-model:current-page="recordQuery.page"
          v-model:page-size="recordQuery.page_size"
          :total="recordTotal"
          :page-sizes="[10, 20, 50, 100]"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="fetchRecords"
          @current-change="fetchRecords"
        />
      </el-tab-pane>

      <!-- 发送统计 Tab -->
      <el-tab-pane label="发送统计" name="statistics">
        <el-row :gutter="20">
          <el-col :span="6">
            <el-card shadow="hover">
              <template #header>总发送量</template>
              <div class="stat-value">{{ statistics.total_sent || 0 }}</div>
            </el-card>
          </el-col>
          <el-col :span="6">
            <el-card shadow="hover">
              <template #header>发送失败</template>
              <div class="stat-value text-danger">{{ statistics.total_failed || 0 }}</div>
            </el-card>
          </el-col>
          <el-col :span="6">
            <el-card shadow="hover">
              <template #header>今日发送</template>
              <div class="stat-value text-primary">{{ statistics.today_sent || 0 }}</div>
            </el-card>
          </el-col>
          <el-col :span="6">
            <el-card shadow="hover">
              <template #header>成功率</template>
              <div class="stat-value text-success">{{ statistics.success_rate || 0 }}%</div>
            </el-card>
          </el-col>
        </el-row>
      </el-tab-pane>
    </el-tabs>

    <!-- 模板编辑对话框 -->
    <el-dialog
      v-model="templateDialogVisible"
      :title="isEditTemplate ? '编辑模板' : '新建模板'"
      width="600px"
    >
      <el-form :model="templateForm" :rules="templateRules" ref="templateFormRef" label-width="100px">
        <el-form-item label="模板名称" prop="name">
          <el-input v-model="templateForm.name" placeholder="请输入模板名称" />
        </el-form-item>
        <el-form-item label="模板编码" prop="code">
          <el-input v-model="templateForm.code" placeholder="请输入模板编码" :disabled="isEditTemplate" />
        </el-form-item>
        <el-form-item label="模板类型" prop="template_type">
          <el-select v-model="templateForm.template_type" placeholder="选择模板类型">
            <el-option label="系统通知" value="system" />
            <el-option label="订单通知" value="order" />
            <el-option label="审批通知" value="approval" />
            <el-option label="库存通知" value="inventory" />
            <el-option label="自定义" value="custom" />
          </el-select>
        </el-form-item>
        <el-form-item label="邮件主题" prop="subject_template">
          <el-input v-model="templateForm.subject_template" placeholder="请输入邮件主题模板" />
        </el-form-item>
        <el-form-item label="邮件内容" prop="body_template">
          <el-input
            v-model="templateForm.body_template"
            type="textarea"
            :rows="10"
            placeholder="请输入邮件内容模板（支持HTML）"
          />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="templateForm.description" type="textarea" :rows="3" placeholder="请输入描述" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="templateDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmitTemplate" :loading="submitLoading">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { emailApi, type EmailTemplate, type EmailLog, type EmailStatistics } from '@/api/email'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'

const activeTab = ref('templates')
const hasLoaded = createLazyLoader()

// 模板相关
const templates = ref<EmailTemplate[]>([])
const templatesLoading = ref(false)
const templateTotal = ref(0)
const templateQuery = reactive({ page: 1, page_size: 20 })
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
  name: [{ required: true, message: '请输入模板名称', trigger: 'blur' }],
  code: [{ required: true, message: '请输入模板编码', trigger: 'blur' }],
  subject_template: [{ required: true, message: '请输入邮件主题', trigger: 'blur' }],
  body_template: [{ required: true, message: '请输入邮件内容', trigger: 'blur' }],
  template_type: [{ required: true, message: '请选择模板类型', trigger: 'change' }],
}

// 记录相关
const records = ref<EmailLog[]>([])
const recordsLoading = ref(false)
const recordTotal = ref(0)
const recordQuery = reactive({ page: 1, page_size: 20, status: '' })
const recordDateRange = ref<[Date, Date] | null>(null)

// 统计相关
const statistics = ref<EmailStatistics>({
  total_sent: 0,
  total_failed: 0,
  today_sent: 0,
  success_rate: 0,
})

onMounted(() => {
  initPage()
})

const loadTab = (tabName: string, loader: Record<string, () => void>) => {
  loadIfNot(tabName, loader[tabName], hasLoaded)
}

const initPage = () => {
  loadTab(activeTab.value, {
    templates: fetchTemplates,
    records: fetchRecords,
    statistics: fetchStatistics,
  })
}

const fetchTemplates = async () => {
  templatesLoading.value = true
  try {
    const res = await emailApi.getTemplates(templateQuery)
    templates.value = res.data?.list || []
    templateTotal.value = res.data?.total || 0
  } catch (error) {
    console.error('获取模板列表失败:', error)
  } finally {
    templatesLoading.value = false
  }
}

const fetchRecords = async () => {
  recordsLoading.value = true
  try {
    const params: any = { ...recordQuery }
    if (recordDateRange.value) {
      params.start_date = recordDateRange.value[0].toISOString()
      params.end_date = recordDateRange.value[1].toISOString()
    }
    const res = await emailApi.getRecords(params)
    records.value = res.data?.list || []
    recordTotal.value = res.data?.total || 0
  } catch (error) {
    console.error('获取发送记录失败:', error)
  } finally {
    recordsLoading.value = false
  }
}

const fetchStatistics = async () => {
  try {
    const res = await emailApi.getStatistics()
    if (res.data) {
      statistics.value = res.data
    }
  } catch (error) {
    console.error('获取统计信息失败:', error)
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
      await emailApi.updateTemplate(templateForm.id, templateForm)
      ElMessage.success('更新成功')
    } else {
      await emailApi.createTemplate(templateForm)
      ElMessage.success('创建成功')
    }
    templateDialogVisible.value = false
    fetchTemplates()
  } catch (error) {
    console.error('提交失败:', error)
  } finally {
    submitLoading.value = false
  }
}

const handleDeleteTemplate = async (row: EmailTemplate) => {
  try {
    await ElMessageBox.confirm('确定要删除该模板吗？', '提示', { type: 'warning' })
    await emailApi.deleteTemplate(row.id!)
    ElMessage.success('删除成功')
    fetchTemplates()
  } catch (error) {
    if (error !== 'cancel') {
      console.error('删除失败:', error)
    }
  }
}

const handleResetRecordQuery = () => {
  recordQuery.status = ''
  recordDateRange.value = null
  recordQuery.page = 1
  fetchRecords()
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
    sent: '成功',
    failed: '失败',
    pending: '待发送',
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

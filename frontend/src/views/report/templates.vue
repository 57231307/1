<script setup lang="ts">
import { ref, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Edit, Delete, View, Download, Bell } from '@element-plus/icons-vue'
import {
  listReportTemplates,
  getReportTemplate,
  createReportTemplate,
  updateReportTemplate,
  deleteReportTemplate,
  getAvailableFields,
  exportReport,
  previewReport,
  listSubscriptions,
  createSubscription,
  updateSubscription,
  deleteSubscription,
  toggleSubscription,
  sendSubscriptionNow,
  type ReportTemplate,
  type ReportTemplateField,
  type ReportFilterCondition,
  type ReportField,
  type ReportSubscription
} from '@/api/report-enhanced'

const loading = ref(false)
const templates = ref<ReportTemplate[]>([])
const total = ref(0)
const pagination = ref({ page: 1, pageSize: 20 })

const searchForm = ref({
  name: '',
  type: '',
  category: ''
})

const templateTypes = [
  { label: '销售报表', value: 'sales' },
  { label: '采购报表', value: 'purchase' },
  { label: '库存报表', value: 'inventory' },
  { label: '财务报表', value: 'finance' },
  { label: '应收报表', value: 'ar' },
  { label: '应付报表', value: 'ap' },
  { label: '自定义', value: 'custom' }
]

const categories = [
  { label: '全部', value: '' },
  { label: '运营报表', value: 'operation' },
  { label: '财务报表', value: 'finance' },
  { label: '分析报表', value: 'analysis' },
  { label: '汇总报表', value: 'summary' }
]

const dialogVisible = ref(false)
const dialogTitle = ref('创建模板')
const isEdit = ref(false)
const form = ref<Partial<ReportTemplate>>({
  name: '',
  description: '',
  type: '',
  category: '',
  fields: [],
  filters: [],
  group_by: [],
  sort_by: [],
  chart_type: 'none'
})

const availableFields = ref<ReportField[]>([])
const selectedFieldKeys = ref<string[]>([])
const fieldConfigs = ref<Record<string, Partial<ReportTemplateField>>>({})

const filterConditions = ref<ReportFilterCondition[]>([])

const previewDialogVisible = ref(false)
const previewData = ref<any>(null)

const subscriptionDialogVisible = ref(false)
const subscriptions = ref<ReportSubscription[]>([])
const subscriptionTotal = ref(0)
const subFormVisible = ref(false)
const subForm = ref({
  id: 0,
  template_id: 0,
  template_name: '',
  schedule: 'weekly' as 'daily' | 'weekly' | 'monthly',
  schedule_time: '09:00',
  recipients: '',
  format: 'excel' as 'pdf' | 'excel' | 'both',
  active: true
})

const exportDialogVisible = ref(false)
const exportForm = ref({
  template_id: 0,
  template_name: '',
  format: 'excel' as 'pdf' | 'excel',
  date_range: { start: '', end: '' }
})

const loadTemplates = async () => {
  loading.value = true
  try {
    const res: any = await listReportTemplates({
      page: pagination.value.page,
      pageSize: pagination.value.pageSize,
      ...searchForm.value
    })
    templates.value = res.data?.list || []
    total.value = res.data?.total || 0
  } catch {
    ElMessage.error('加载模板列表失败')
  } finally {
    loading.value = false
  }
}

const handleSearch = () => {
  pagination.value.page = 1
  loadTemplates()
}

const handleReset = () => {
  searchForm.value = { name: '', type: '', category: '' }
  handleSearch()
}

const handlePageChange = (page: number) => {
  pagination.value.page = page
  loadTemplates()
}

const handlePageSizeChange = (pageSize: number) => {
  pagination.value.pageSize = pageSize
  loadTemplates()
}

const openCreateDialog = async () => {
  dialogTitle.value = '创建模板'
  isEdit.value = false
  form.value = {
    name: '',
    description: '',
    type: '',
    category: '',
    fields: [],
    filters: [],
    group_by: [],
    sort_by: [],
    chart_type: 'none'
  }
  selectedFieldKeys.value = []
  fieldConfigs.value = {}
  filterConditions.value = []
  dialogVisible.value = true
}

const openEditDialog = async (row: ReportTemplate) => {
  dialogTitle.value = '编辑模板'
  isEdit.value = true
  try {
    const res: any = await getReportTemplate(row.id)
    const data = res.data
    form.value = { ...data }
    selectedFieldKeys.value = data.fields?.map((f: ReportTemplateField) => f.field_key) || []
    fieldConfigs.value = {}
    data.fields?.forEach((f: ReportTemplateField) => {
      fieldConfigs.value[f.field_key] = {
        display_label: f.display_label,
        width: f.width,
        format: f.format
      }
    })
    filterConditions.value = data.filters || []
    dialogVisible.value = true
  } catch {
    ElMessage.error('获取模板详情失败')
  }
}

const handleDelete = async (row: ReportTemplate) => {
  if (row.is_system) {
    ElMessage.warning('系统模板不可删除')
    return
  }
  try {
    await ElMessageBox.confirm('确定要删除这个模板吗？', '提示', { type: 'warning' })
    await deleteReportTemplate(row.id)
    ElMessage.success('删除成功')
    loadTemplates()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error('删除失败')
    }
  }
}

const handleTypeChange = async () => {
  if (!form.value.type) return
  try {
    const res: any = await getAvailableFields(form.value.type)
    availableFields.value = res.data || []
  } catch {
    ElMessage.error('获取可用字段失败')
  }
}

const selectedFields = computed(() => {
  return availableFields.value.filter(f => selectedFieldKeys.value.includes(f.key))
})

const handleSubmit = async () => {
  if (!form.value.name || !form.value.type) {
    ElMessage.warning('请填写必填字段')
    return
  }
  if (selectedFieldKeys.value.length === 0) {
    ElMessage.warning('请至少选择一个字段')
    return
  }

  const fields: ReportTemplateField[] = selectedFieldKeys.value.map(key => {
    const field = availableFields.value.find(f => f.key === key)
    const config = fieldConfigs.value[key] || {}
    return {
      field_key: key,
      display_label: config.display_label || field?.label || key,
      visible: true,
      width: config.width,
      format: config.format
    }
  })

  const data = {
    name: form.value.name!,
    description: form.value.description,
    type: form.value.type!,
    category: form.value.category || 'operation',
    fields,
    filters: filterConditions.value,
    group_by: form.value.group_by || [],
    sort_by: form.value.sort_by || [],
    chart_type: form.value.chart_type || 'none'
  }

  try {
    if (isEdit.value && form.value.id) {
      await updateReportTemplate(form.value.id, data)
      ElMessage.success('更新成功')
    } else {
      await createReportTemplate(data)
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false
    loadTemplates()
  } catch {
    ElMessage.error('操作失败')
  }
}

const addFilter = () => {
  filterConditions.value.push({
    field: '',
    operator: 'eq',
    value: ''
  })
}

const removeFilter = (index: number) => {
  filterConditions.value.splice(index, 1)
}

const handlePreview = async (row: ReportTemplate) => {
  try {
    const res: any = await previewReport(row.id)
    previewData.value = res.data
    previewDialogVisible.value = true
  } catch {
    ElMessage.error('预览失败')
  }
}

const handleExport = (row: ReportTemplate) => {
  exportForm.value = {
    template_id: row.id,
    template_name: row.name,
    format: 'excel',
    date_range: { start: '', end: '' }
  }
  exportDialogVisible.value = true
}

const doExport = async () => {
  try {
    const blob = await exportReport(exportForm.value.template_id, {
      format: exportForm.value.format,
      date_range: exportForm.value.date_range.start && exportForm.value.date_range.end
        ? { start: exportForm.value.date_range.start, end: exportForm.value.date_range.end }
        : undefined
    })
    const url = window.URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.download = `${exportForm.value.template_name}.${exportForm.value.format === 'pdf' ? 'pdf' : 'xlsx'}`
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    window.URL.revokeObjectURL(url)
    ElMessage.success('导出成功')
    exportDialogVisible.value = false
  } catch {
    ElMessage.error('导出失败')
  }
}

const handleSubscriptions = async (row: ReportTemplate) => {
  subForm.value.template_id = row.id
  subForm.value.template_name = row.name
  try {
    const res: any = await listSubscriptions({ template_id: row.id })
    subscriptions.value = res.data?.list || []
    subscriptionTotal.value = res.data?.total || 0
  } catch {
    console.warn('加载订阅列表失败')
  }
  subscriptionDialogVisible.value = true
}

const openSubForm = (row?: ReportSubscription) => {
  if (row) {
    subForm.value = {
      id: row.id,
      template_id: row.template_id,
      template_name: row.template_name,
      schedule: row.schedule,
      schedule_time: row.schedule_time,
      recipients: row.recipients.join(', '),
      format: row.format,
      active: row.active
    }
  } else {
    subForm.value = {
      id: 0,
      template_id: subForm.value.template_id,
      template_name: subForm.value.template_name,
      schedule: 'weekly',
      schedule_time: '09:00',
      recipients: '',
      format: 'excel',
      active: true
    }
  }
  subFormVisible.value = true
}

const handleSubmitSubscription = async () => {
  if (!subForm.value.recipients) {
    ElMessage.warning('请填写接收人邮箱')
    return
  }
  const recipients = subForm.value.recipients.split(',').map(r => r.trim()).filter(Boolean)
  const data = {
    template_id: subForm.value.template_id,
    schedule: subForm.value.schedule,
    schedule_time: subForm.value.schedule_time,
    recipients,
    format: subForm.value.format
  }

  try {
    if (subForm.value.id) {
      await updateSubscription(subForm.value.id, {
        schedule: subForm.value.schedule,
        schedule_time: subForm.value.schedule_time,
        recipients,
        format: subForm.value.format,
        active: subForm.value.active
      })
      ElMessage.success('更新成功')
    } else {
      await createSubscription(data)
      ElMessage.success('创建成功')
    }
    subFormVisible.value = false
    handleSubscriptions({ id: subForm.value.template_id, name: subForm.value.template_name } as ReportTemplate)
  } catch {
    ElMessage.error('操作失败')
  }
}

const handleToggleSubscription = async (row: ReportSubscription) => {
  try {
    await toggleSubscription(row.id)
    ElMessage.success('状态已切换')
    handleSubscriptions({ id: row.template_id, name: '' } as ReportTemplate)
  } catch {
    ElMessage.error('操作失败')
  }
}

const handleDeleteSubscription = async (row: ReportSubscription) => {
  try {
    await ElMessageBox.confirm('确定要删除这个订阅吗？', '提示', { type: 'warning' })
    await deleteSubscription(row.id)
    ElMessage.success('删除成功')
    handleSubscriptions({ id: row.template_id, name: '' } as ReportTemplate)
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error('删除失败')
    }
  }
}

const handleSendNow = async (row: ReportSubscription) => {
  try {
    await sendSubscriptionNow(row.id)
    ElMessage.success('发送成功')
  } catch {
    ElMessage.error('发送失败')
  }
}

const getScheduleLabel = (schedule: string) => {
  const map: Record<string, string> = { daily: '每天', weekly: '每周', monthly: '每月' }
  return map[schedule] || schedule
}

const getFormatLabel = (format: string) => {
  const map: Record<string, string> = { pdf: 'PDF', excel: 'Excel', both: 'PDF + Excel' }
  return map[format] || format
}

const chartTypeOptions = [
  { label: '无图表', value: 'none' },
  { label: '柱状图', value: 'bar' },
  { label: '折线图', value: 'line' },
  { label: '饼图', value: 'pie' },
  { label: '面积图', value: 'area' }
]

const operatorOptions = [
  { label: '等于', value: 'eq' },
  { label: '不等于', value: 'ne' },
  { label: '大于', value: 'gt' },
  { label: '小于', value: 'lt' },
  { label: '大于等于', value: 'gte' },
  { label: '小于等于', value: 'lte' },
  { label: '包含', value: 'contains' },
  { label: '在...中', value: 'in' },
  { label: '区间', value: 'between' }
]

loadTemplates()
</script>

<template>
  <div class="app-container">
    <div class="filter-container">
      <el-row :gutter="20">
        <el-col :span="7">
          <el-input v-model="searchForm.name" placeholder="模板名称" clearable @keyup.enter="handleSearch" />
        </el-col>
        <el-col :span="5">
          <el-select v-model="searchForm.type" placeholder="报表类型" clearable>
            <el-option v-for="t in templateTypes" :key="t.value" :label="t.label" :value="t.value" />
          </el-select>
        </el-col>
        <el-col :span="5">
          <el-select v-model="searchForm.category" placeholder="分类" clearable>
            <el-option v-for="c in categories" :key="c.value" :label="c.label" :value="c.value" />
          </el-select>
        </el-col>
        <el-col :span="7">
          <el-button type="primary" @click="handleSearch">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
          <el-button type="success" @click="openCreateDialog">
            <el-icon><Plus /></el-icon> 创建模板
          </el-button>
        </el-col>
      </el-row>
    </div>

    <el-table :data="templates" :loading="loading" border fit highlight-current-row style="width: 100%">
      <el-table-column prop="name" label="模板名称" min-width="160" show-overflow-tooltip />
      <el-table-column prop="description" label="描述" min-width="180" show-overflow-tooltip />
      <el-table-column label="类型" width="100">
        <template #default="scope">
          {{ templateTypes.find(t => t.value === scope.row.type)?.label || scope.row.type }}
        </template>
      </el-table-column>
      <el-table-column label="分类" width="100">
        <template #default="scope">
          <el-tag size="small">{{ scope.row.category }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column label="字段数" width="80" align="center">
        <template #default="scope">{{ scope.row.fields?.length || 0 }}</template>
      </el-table-column>
      <el-table-column label="图表" width="80" align="center">
        <template #default="scope">
          <el-tag v-if="scope.row.chart_type !== 'none'" size="small" type="success">{{ scope.row.chart_type }}</el-tag>
          <span v-else>-</span>
        </template>
      </el-table-column>
      <el-table-column prop="updated_at" label="更新时间" width="160" />
      <el-table-column label="操作" width="280" align="center">
        <template #default="scope">
          <el-button size="small" @click="handlePreview(scope.row)">
            <el-icon><View /></el-icon> 预览
          </el-button>
          <el-button size="small" type="warning" @click="handleExport(scope.row)">
            <el-icon><Download /></el-icon> 导出
          </el-button>
          <el-button size="small" type="info" @click="handleSubscriptions(scope.row)">
            <el-icon><Bell /></el-icon> 订阅
          </el-button>
          <el-button
            v-if="!scope.row.is_system"
            size="small"
            type="primary"
            @click="openEditDialog(scope.row)"
          >
            <el-icon><Edit /></el-icon>
          </el-button>
          <el-button
            v-if="!scope.row.is_system"
            size="small"
            type="danger"
            @click="handleDelete(scope.row)"
          >
            <el-icon><Delete /></el-icon>
          </el-button>
        </template>
      </el-table-column>
    </el-table>
    <el-pagination
      v-model:current-page="pagination.page"
      v-model:page-size="pagination.pageSize"
      :total="total"
      :page-sizes="[10, 20, 50, 100]"
      layout="total, sizes, prev, pager, next"
      @current-change="handlePageChange"
      @size-change="handlePageSizeChange"
      class="pagination-container"
    />

    <el-dialog v-model="dialogVisible" :title="dialogTitle" width="900px" :close-on-click-modal="false">
      <el-form :model="form" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="模板名称" required>
              <el-input v-model="form.name" placeholder="请输入模板名称" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="报表类型" required>
              <el-select v-model="form.type" placeholder="请选择报表类型" @change="handleTypeChange">
                <el-option v-for="t in templateTypes" :key="t.value" :label="t.label" :value="t.value" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="分类">
              <el-select v-model="form.category" placeholder="请选择分类">
                <el-option v-for="c in categories" :key="c.value" :label="c.label" :value="c.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="图表类型">
              <el-select v-model="form.chart_type" placeholder="请选择图表类型">
                <el-option v-for="c in chartTypeOptions" :key="c.value" :label="c.label" :value="c.value" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="描述">
          <el-input v-model="form.description" type="textarea" :rows="2" placeholder="请输入模板描述" />
        </el-form-item>

        <el-divider content-position="left">字段配置</el-divider>
        <div v-if="availableFields.length > 0" class="field-config-area">
          <el-checkbox-group v-model="selectedFieldKeys" class="field-checkbox-group">
            <el-checkbox
              v-for="field in availableFields"
              :key="field.key"
              :value="field.key"
              border
              class="field-checkbox"
            >
              {{ field.label }}
              <el-tag size="small" type="info">{{ field.type }}</el-tag>
            </el-checkbox>
          </el-checkbox-group>

          <div v-if="selectedFields.length > 0" class="field-config-detail">
            <h4>字段属性配置</h4>
            <el-table :data="selectedFields" border size="small">
              <el-table-column prop="label" label="字段名" width="150" />
              <el-table-column label="显示名称" width="180">
                <template #default="scope">
                  <el-input
                    v-model="fieldConfigs[scope.row.key].display_label"
                    size="small"
                    :placeholder="scope.row.label"
                  />
                </template>
              </el-table-column>
              <el-table-column label="宽度" width="100">
                <template #default="scope">
                  <el-input-number
                    v-model="fieldConfigs[scope.row.key].width"
                    size="small"
                    :min="50"
                    :max="500"
                    :step="10"
                  />
                </template>
              </el-table-column>
              <el-table-column label="格式化">
                <template #default="scope">
                  <el-input
                    v-model="fieldConfigs[scope.row.key].format"
                    size="small"
                    placeholder="如: YYYY-MM-DD, ¥#,##0.00"
                  />
                </template>
              </el-table-column>
            </el-table>
          </div>
        </div>
        <el-empty v-else description="请先选择报表类型" :image-size="80" />

        <el-divider content-position="left">筛选条件</el-divider>
        <div class="filter-config-area">
          <el-button size="small" @click="addFilter">
            <el-icon><Plus /></el-icon> 添加筛选条件
          </el-button>
          <div v-for="(condition, index) in filterConditions" :key="index" class="filter-row">
            <el-select v-model="condition.field" placeholder="字段" size="small" style="width: 160px">
              <el-option
                v-for="f in availableFields"
                :key="f.key"
                :label="f.label"
                :value="f.key"
              />
            </el-select>
            <el-select v-model="condition.operator" placeholder="操作符" size="small" style="width: 120px; margin-left: 8px">
              <el-option v-for="op in operatorOptions" :key="op.value" :label="op.label" :value="op.value" />
            </el-select>
            <el-input v-model="condition.value" placeholder="值" size="small" style="width: 160px; margin-left: 8px" />
            <el-button size="small" type="danger" @click="removeFilter(index)" style="margin-left: 8px">删除</el-button>
          </div>
        </div>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="previewDialogVisible" title="报表预览" width="900px">
      <div v-if="previewData">
        <el-table :data="previewData.list || []" border style="width: 100%">
          <el-table-column
            v-for="col in previewData.columns || []"
            :key="col.key"
            :prop="col.key"
            :label="col.label"
            :width="col.width"
          />
        </el-table>
        <div class="preview-total">共 {{ previewData.total || 0 }} 条记录</div>
      </div>
    </el-dialog>

    <el-dialog v-model="exportDialogVisible" title="导出报表" width="500px">
      <el-form label-width="100px">
        <el-form-item label="模板名称">
          <el-input v-model="exportForm.template_name" disabled />
        </el-form-item>
        <el-form-item label="导出格式">
          <el-radio-group v-model="exportForm.format">
            <el-radio value="excel">Excel</el-radio>
            <el-radio value="pdf">PDF</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="日期范围">
          <el-date-picker
            v-model="exportForm.date_range.start"
            type="date"
            placeholder="开始日期"
            style="width: 45%; margin-right: 10px"
          />
          <el-date-picker
            v-model="exportForm.date_range.end"
            type="date"
            placeholder="结束日期"
            style="width: 45%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="exportDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="doExport">
          <el-icon><Download /></el-icon> 导出
        </el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="subscriptionDialogVisible" title="订阅管理" width="900px">
      <div class="sub-header">
        <span>模板：{{ subForm.template_name }}</span>
        <el-button type="primary" size="small" @click="openSubForm">
          <el-icon><Plus /></el-icon> 新建订阅
        </el-button>
      </div>
      <el-table :data="subscriptions" border style="width: 100%; margin-top: 16px">
        <el-table-column label="频率" width="80">
          <template #default="scope">{{ getScheduleLabel(scope.row.schedule) }}</template>
        </el-table-column>
        <el-table-column prop="schedule_time" label="发送时间" width="100" />
        <el-table-column label="接收人" min-width="200">
          <template #default="scope">
            <el-tag v-for="(r, i) in scope.row.recipients" :key="i" size="small" style="margin-right: 4px">
              {{ r }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="格式" width="120">
          <template #default="scope">{{ getFormatLabel(scope.row.format) }}</template>
        </el-table-column>
        <el-table-column label="状态" width="80">
          <template #default="scope">
            <el-tag size="small" :type="scope.row.active ? 'success' : 'info'">
              {{ scope.row.active ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="last_sent_at" label="最后发送" width="160" />
        <el-table-column label="操作" width="200" align="center">
          <template #default="scope">
            <el-button size="small" @click="openSubForm(scope.row)">
              <el-icon><Edit /></el-icon>
            </el-button>
            <el-button size="small" :type="scope.row.active ? 'warning' : 'success'" @click="handleToggleSubscription(scope.row)">
              {{ scope.row.active ? '禁用' : '启用' }}
            </el-button>
            <el-button size="small" type="success" @click="handleSendNow(scope.row)">发送</el-button>
            <el-button size="small" type="danger" @click="handleDeleteSubscription(scope.row)">
              <el-icon><Delete /></el-icon>
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-dialog>

    <el-dialog v-model="subFormVisible" :title="subForm.id ? '编辑订阅' : '新建订阅'" width="600px">
      <el-form label-width="100px">
        <el-form-item label="发送频率">
          <el-select v-model="subForm.schedule" style="width: 100%">
            <el-option label="每天" value="daily" />
            <el-option label="每周" value="weekly" />
            <el-option label="每月" value="monthly" />
          </el-select>
        </el-form-item>
        <el-form-item label="发送时间">
          <el-time-picker v-model="subForm.schedule_time" format="HH:mm" value-format="HH:mm" style="width: 100%" />
        </el-form-item>
        <el-form-item label="接收人邮箱">
          <el-input v-model="subForm.recipients" placeholder="多个邮箱用逗号分隔" />
        </el-form-item>
        <el-form-item label="导出格式">
          <el-radio-group v-model="subForm.format">
            <el-radio value="excel">Excel</el-radio>
            <el-radio value="pdf">PDF</el-radio>
            <el-radio value="both">PDF + Excel</el-radio>
          </el-radio-group>
        </el-form-item>
        <el-form-item label="状态">
          <el-switch v-model="subForm.active" active-text="启用" inactive-text="禁用" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="subFormVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmitSubscription">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.app-container {
  padding: 20px;
}

.filter-container {
  margin-bottom: 20px;
  background: #fff;
  padding: 20px;
  border-radius: 8px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.05);
}

.pagination-container {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}

.field-config-area {
  margin: 16px 0;
}

.field-checkbox-group {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.field-checkbox {
  margin-right: 0;
}

.field-config-detail {
  margin-top: 16px;
}

.field-config-detail h4 {
  margin-bottom: 12px;
  color: #303133;
}

.filter-config-area {
  margin: 16px 0;
}

.filter-row {
  display: flex;
  align-items: center;
  margin-top: 8px;
}

.sub-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 16px;
  font-weight: 500;
}

.preview-total {
  margin-top: 16px;
  text-align: right;
  color: #909399;
}
</style>

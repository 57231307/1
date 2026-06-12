<template>
  <div class="data-import-page">
    <div class="page-header">
      <h2 class="page-title">数据导入</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openTemplateDialog()">
          <el-icon><Plus /></el-icon>
          新建模板
        </el-button>
      </div>
    </div>

    <el-tabs v-model="activeTab">
      <el-tab-pane label="导入模板" name="templates">
        <el-card shadow="hover">
          <div class="filter-container">
            <el-input
              v-model="templateQuery.keyword"
              placeholder="搜索模板编号/名称"
              style="width: 200px"
              clearable
              @clear="fetchTemplates"
              @keyup.enter="fetchTemplates"
            />
            <el-select
              v-model="templateQuery.module"
              placeholder="模块"
              clearable
              style="width: 120px"
            >
              <el-option label="客户" value="customer" />
              <el-option label="供应商" value="supplier" />
              <el-option label="产品" value="product" />
              <el-option label="库存" value="inventory" />
              <el-option label="销售" value="sales" />
              <el-option label="采购" value="purchase" />
              <el-option label="财务" value="finance" />
            </el-select>
            <el-button type="primary" @click="fetchTemplates">
              <el-icon><Search /></el-icon>
              搜索
            </el-button>
          </div>

          <el-table v-loading="templateLoading" :data="templates" stripe>
            <el-table-column prop="template_code" label="模板编号" width="140" />
            <el-table-column prop="template_name" label="模板名称" min-width="180" />
            <el-table-column prop="module" label="模块" width="100">
              <template #default="{ row }">
                {{ moduleMap[row.module] }}
              </template>
            </el-table-column>
            <el-table-column prop="file_format" label="文件格式" width="100">
              <template #default="{ row }">
                {{ row.file_format.toUpperCase() }}
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
                  {{ row.status === 'active' ? '启用' : '停用' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="created_at" label="创建时间" width="160" />
            <el-table-column label="操作" width="250" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="handleDownloadTemplate(row)"
                  >下载模板</el-button
                >
                <el-button type="primary" link size="small" @click="openUploadDialog(row)"
                  >导入数据</el-button
                >
                <el-button type="primary" link size="small" @click="openTemplateDialog(row)"
                  >编辑</el-button
                >
                <el-button type="danger" link size="small" @click="handleDeleteTemplate(row)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>

          <div class="pagination-container">
            <el-pagination
              v-model:current-page="templateQuery.page"
              v-model:page-size="templateQuery.page_size"
              :page-sizes="[10, 20, 50, 100]"
              :total="templateTotal"
              layout="total, sizes, prev, pager, next, jumper"
              @size-change="fetchTemplates"
              @current-change="fetchTemplates"
            />
          </div>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="导入任务" name="tasks">
        <el-card shadow="hover">
          <div class="filter-container">
            <el-select v-model="taskQuery.status" placeholder="状态" clearable style="width: 120px">
              <el-option label="待处理" value="pending" />
              <el-option label="处理中" value="processing" />
              <el-option label="已完成" value="completed" />
              <el-option label="失败" value="failed" />
            </el-select>
            <el-button type="primary" @click="fetchTasks">
              <el-icon><Search /></el-icon>
              搜索
            </el-button>
          </div>

          <el-table v-loading="taskLoading" :data="tasks" stripe>
            <el-table-column prop="task_code" label="任务编号" width="140" />
            <el-table-column prop="template_name" label="导入模板" width="150" />
            <el-table-column prop="file_name" label="文件名" min-width="180" />
            <el-table-column prop="status" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="taskStatusTypeMap[row.status]" size="small">
                  {{ taskStatusMap[row.status] }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="progress" label="进度" width="120">
              <template #default="{ row }">
                <el-progress
                  :percentage="row.progress"
                  :status="row.status === 'failed' ? 'exception' : undefined"
                />
              </template>
            </el-table-column>
            <el-table-column prop="total_rows" label="总行数" width="80" />
            <el-table-column prop="success_rows" label="成功" width="80" />
            <el-table-column prop="failed_rows" label="失败" width="80" />
            <el-table-column prop="created_by_name" label="创建人" width="100" />
            <el-table-column prop="created_at" label="创建时间" width="160" />
            <el-table-column label="操作" width="200" fixed="right">
              <template #default="{ row }">
                <el-button
                  v-if="row.status === 'failed'"
                  type="primary"
                  link
                  size="small"
                  @click="handleRetryTask(row)"
                  >重试</el-button
                >
                <el-button
                  v-if="row.status === 'pending' || row.status === 'processing'"
                  type="danger"
                  link
                  size="small"
                  @click="handleCancelTask(row)"
                  >取消</el-button
                >
                <el-button
                  v-if="row.failed_rows > 0"
                  type="warning"
                  link
                  size="small"
                  @click="handleDownloadErrorLog(row)"
                  >错误日志</el-button
                >
              </template>
            </el-table-column>
          </el-table>

          <div class="pagination-container">
            <el-pagination
              v-model:current-page="taskQuery.page"
              v-model:page-size="taskQuery.page_size"
              :page-sizes="[10, 20, 50, 100]"
              :total="taskTotal"
              layout="total, sizes, prev, pager, next, jumper"
              @size-change="fetchTasks"
              @current-change="fetchTasks"
            />
          </div>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <el-dialog
      v-model="templateDialogVisible"
      :title="templateForm.id ? '编辑模板' : '新建模板'"
      width="800px"
    >
      <el-form
        ref="templateFormRef"
        :model="templateForm"
        :rules="templateRules"
        label-width="100px"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="模板编号" prop="template_code">
              <el-input
                v-model="templateForm.template_code"
                :disabled="!!templateForm.id"
                placeholder="请输入模板编号"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="模板名称" prop="template_name">
              <el-input v-model="templateForm.template_name" placeholder="请输入模板名称" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="模块" prop="module">
              <el-select v-model="templateForm.module" placeholder="请选择模块" style="width: 100%">
                <el-option label="客户" value="customer" />
                <el-option label="供应商" value="supplier" />
                <el-option label="产品" value="product" />
                <el-option label="库存" value="inventory" />
                <el-option label="销售" value="sales" />
                <el-option label="采购" value="purchase" />
                <el-option label="财务" value="finance" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="文件格式" prop="file_format">
              <el-select
                v-model="templateForm.file_format"
                placeholder="请选择格式"
                style="width: 100%"
              >
                <el-option label="Excel" value="xlsx" />
                <el-option label="CSV" value="csv" />
                <el-option label="JSON" value="json" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="描述" prop="description">
          <el-input
            v-model="templateForm.description"
            type="textarea"
            :rows="3"
            placeholder="请输入描述"
          />
        </el-form-item>
        <el-form-item label="列配置" prop="columns">
          <el-input
            v-model="columnsText"
            type="textarea"
            :rows="6"
            placeholder='JSON格式列配置，例如：[{"key":"name","label":"名称","type":"string","required":true}]'
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="templateDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="templateSubmitLoading" @click="handleTemplateSubmit"
          >确定</el-button
        >
      </template>
    </el-dialog>

    <el-dialog v-model="uploadDialogVisible" title="导入数据" width="500px">
      <el-upload
        ref="uploadRef"
        :auto-upload="false"
        :limit="1"
        :on-exceed="handleExceed"
        :on-change="handleFileChange"
        accept=".xlsx,.csv,.json"
      >
        <template #trigger>
          <el-button type="primary">选择文件</el-button>
        </template>
        <template #tip>
          <div class="el-upload__tip">只能上传 .xlsx/.csv/.json 文件，且不超过 10MB</div>
        </template>
      </el-upload>
      <template #footer>
        <el-button @click="uploadDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="uploadLoading" @click="handleUpload"
          >开始导入</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import {
  ElMessage,
  ElMessageBox,
  type FormInstance,
  type FormRules,
  type UploadFile,
  type UploadInstance,
} from 'element-plus'
import { Plus, Search } from '@element-plus/icons-vue'
import {
  listImportTemplates,
  createImportTemplate,
  updateImportTemplate,
  deleteImportTemplate,
  downloadImportTemplate,
  listImportTasks,
  uploadImportFile,
  cancelImportTask,
  retryImportTask,
  downloadErrorLog,
  type ImportTemplate,
  type ImportTask,
} from '@/api/data-import'

const activeTab = ref('templates')

// 模板相关
const templates = ref<ImportTemplate[]>([])
const templateTotal = ref(0)
const templateLoading = ref(false)
const templateQuery = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  module: '',
})

const moduleMap: Record<string, string> = {
  customer: '客户',
  supplier: '供应商',
  product: '产品',
  inventory: '库存',
  sales: '销售',
  purchase: '采购',
  finance: '财务',
}

const fetchTemplates = async () => {
  templateLoading.value = true
  try {
    const res = await listImportTemplates(templateQuery)
    templates.value = res.data || []
    templateTotal.value = res.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取模板失败')
  } finally {
    templateLoading.value = false
  }
}

// 任务相关
const tasks = ref<ImportTask[]>([])
const taskTotal = ref(0)
const taskLoading = ref(false)
const taskQuery = reactive({
  page: 1,
  page_size: 20,
  status: '',
})

const taskStatusMap: Record<string, string> = {
  pending: '待处理',
  processing: '处理中',
  completed: '已完成',
  failed: '失败',
}

const taskStatusTypeMap: Record<string, string> = {
  pending: 'info',
  processing: 'warning',
  completed: 'success',
  failed: 'danger',
}

const fetchTasks = async () => {
  taskLoading.value = true
  try {
    const res = await listImportTasks(taskQuery)
    tasks.value = res.data || []
    taskTotal.value = res.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取任务失败')
  } finally {
    taskLoading.value = false
  }
}

// 模板表单
const templateDialogVisible = ref(false)
const templateFormRef = ref<FormInstance>()
const templateSubmitLoading = ref(false)
const columnsText = ref('')
const templateForm = reactive<Partial<ImportTemplate>>({
  id: undefined,
  template_code: '',
  template_name: '',
  description: '',
  module: 'customer',
  file_format: 'xlsx',
  columns: [],
  sample_data: [],
  status: 'active',
})

const templateRules: FormRules = {
  template_code: [{ required: true, message: '请输入模板编号', trigger: 'blur' }],
  template_name: [{ required: true, message: '请输入模板名称', trigger: 'blur' }],
  module: [{ required: true, message: '请选择模块', trigger: 'change' }],
  file_format: [{ required: true, message: '请选择文件格式', trigger: 'change' }],
}

const openTemplateDialog = (row?: ImportTemplate) => {
  if (row) {
    Object.assign(templateForm, row)
    columnsText.value = JSON.stringify(row.columns || [], null, 2)
  } else {
    Object.assign(templateForm, {
      id: undefined,
      template_code: '',
      template_name: '',
      description: '',
      module: 'customer',
      file_format: 'xlsx',
      columns: [],
      sample_data: [],
      status: 'active',
    })
    columnsText.value = ''
  }
  templateDialogVisible.value = true
}

const handleTemplateSubmit = async () => {
  if (!templateFormRef.value) return
  await templateFormRef.value.validate(async (valid) => {
    if (!valid) return

    templateSubmitLoading.value = true
    try {
      if (columnsText.value) {
        try {
          templateForm.columns = JSON.parse(columnsText.value)
        } catch (e) {
          ElMessage.error('列配置格式错误，请检查JSON格式')
          return
        }
      }
      if (templateForm.id) {
        await updateImportTemplate(templateForm.id, templateForm)
      } else {
        await createImportTemplate(templateForm)
      }
      ElMessage.success('操作成功')
      templateDialogVisible.value = false
      fetchTemplates()
    } catch (error: any) {
      ElMessage.error(error.message || '操作失败')
    } finally {
      templateSubmitLoading.value = false
    }
  })
}

const handleDeleteTemplate = async (row: ImportTemplate) => {
  try {
    await ElMessageBox.confirm('确定要删除此模板吗？', '确认删除', { type: 'warning' })
    await deleteImportTemplate(row.id)
    ElMessage.success('删除成功')
    fetchTemplates()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '删除失败')
  }
}

const handleDownloadTemplate = async (row: ImportTemplate) => {
  try {
    const blob = await downloadImportTemplate(row.id)
    const link = document.createElement('a')
    link.href = URL.createObjectURL(blob)
    link.download = `${row.template_name}_模板.${row.file_format}`
    link.click()
    ElMessage.success('模板下载成功')
  } catch (error: any) {
    ElMessage.error(error.message || '下载失败')
  }
}

// 上传相关
const uploadDialogVisible = ref(false)
const uploadLoading = ref(false)
const uploadRef = ref<UploadInstance>()
const currentTemplate = ref<ImportTemplate | null>(null)
const selectedFile = ref<File | null>(null)

const openUploadDialog = (row: ImportTemplate) => {
  currentTemplate.value = row
  selectedFile.value = null
  uploadDialogVisible.value = true
}

const handleExceed = () => {
  ElMessage.warning('只能上传一个文件')
}

const handleFileChange = (file: UploadFile) => {
  selectedFile.value = file.raw || null
}

const handleUpload = async () => {
  if (!selectedFile.value || !currentTemplate.value) {
    ElMessage.warning('请选择文件')
    return
  }

  uploadLoading.value = true
  try {
    await uploadImportFile(currentTemplate.value.id, selectedFile.value)
    ElMessage.success('导入任务已创建')
    uploadDialogVisible.value = false
    fetchTasks()
    activeTab.value = 'tasks'
  } catch (error: any) {
    ElMessage.error(error.message || '导入失败')
  } finally {
    uploadLoading.value = false
  }
}

const handleCancelTask = async (row: ImportTask) => {
  try {
    await ElMessageBox.confirm('确定要取消此任务吗？', '确认取消', { type: 'warning' })
    await cancelImportTask(row.id)
    ElMessage.success('任务已取消')
    fetchTasks()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '取消失败')
  }
}

const handleRetryTask = async (row: ImportTask) => {
  try {
    await retryImportTask(row.id)
    ElMessage.success('任务已重新开始')
    fetchTasks()
  } catch (error: any) {
    ElMessage.error(error.message || '重试失败')
  }
}

const handleDownloadErrorLog = async (row: ImportTask) => {
  try {
    const blob = await downloadErrorLog(row.id)
    const link = document.createElement('a')
    link.href = URL.createObjectURL(blob)
    link.download = `错误日志_${row.task_code}.txt`
    link.click()
    ElMessage.success('错误日志下载成功')
  } catch (error: any) {
    ElMessage.error(error.message || '下载失败')
  }
}

onMounted(() => {
  fetchTemplates()
  fetchTasks()
})
</script>

<style scoped>
.data-import-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
.filter-container {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}
.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
